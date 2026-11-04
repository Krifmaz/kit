//! [`Store`]: the values of a settings table on the app's database The value in
//! effect is the environment variable's, then the saved one, then the default; saved values are the
//! `settings` table's rows (`key`, `value` as JSON text).

use crate::table::Table;
use crate::values::{clean_env, coerce, masked, Def, EnvLookup, Values};
use kit_server_core::{p, Db, Result};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// The settings in effect: defaults, then the values saved in the UI, then the environment. Clones share them.
pub struct Store<D: Db> {
    /// every setting's definition, in the table's order
    pub defs: Arc<Vec<(String, Def)>>,
    /// the table the store follows
    table: Table,
    /// the saved value (or the default) of every key
    cache: Arc<RwLock<HashMap<String, Value>>>,
    /// the keys an environment variable sets, with its value
    env: Arc<RwLock<HashMap<String, Value>>>,
    /// moves on every reload and every save, so what is worked out from the settings can be kept until then
    generation: Arc<AtomicU64>,
    /// where the saved values are
    db: Arc<D>,
    /// reads an environment variable (on every reload)
    lookup: EnvLookup,
}

impl<D: Db> Clone for Store<D> {
    fn clone(&self) -> Self {
        Store {
            defs: self.defs.clone(),
            table: self.table,
            cache: self.cache.clone(),
            env: self.env.clone(),
            generation: self.generation.clone(),
            db: self.db.clone(),
            lookup: self.lookup.clone(),
        }
    }
}

impl<D: Db> Store<D> {
    /// A store of `table` on `db` with the environment variables `lookup` gives, nothing read yet (every key
    /// reads as null until [`Store::reload`] or [`Store::apply`]).
    pub fn new(db: D, table: Table, lookup: impl Fn(&str) -> Option<String> + Send + Sync + 'static) -> Store<D> {
        let defs: Vec<(String, Def)> = table
            .keys
            .iter()
            .map(|d| (d.key.to_string(), Def { env: d.env.map(String::from), default: d.default.to_json(), secret: d.secret }))
            .collect();
        Store {
            defs: Arc::new(defs),
            table,
            cache: Default::default(),
            env: Default::default(),
            generation: Default::default(),
            db: Arc::new(db),
            lookup: Arc::new(lookup),
        }
    }

    /// The values of `table` on `db` (its `settings` table is made when missing), with the process's
    /// environment variables over them.
    pub fn load(db: D, table: Table) -> Result<Store<D>> {
        Store::load_with(db, table, |name| std::env::var(name).ok())
    }

    /// [`Store::load`] with the environment variables `lookup` gives (a test's own set).
    pub fn load_with(db: D, table: Table, lookup: impl Fn(&str) -> Option<String> + Send + Sync + 'static) -> Result<Store<D>> {
        db.execute_batch("CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);")?;
        let store = Store::new(db, table, lookup);
        store.reload()?;
        Ok(store)
    }

    /// Read the saved values and the environment again.
    pub fn reload(&self) -> Result<()> {
        let saved = self.saved()?;
        self.apply(&saved);
        Ok(())
    }

    /// The saved values, as stored (rows whose value is not JSON are left out).
    pub fn saved(&self) -> Result<HashMap<String, Value>> {
        Ok(self
            .db
            .query("SELECT key, value FROM settings", p![])?
            .into_iter()
            .filter_map(|r| {
                let k = r.get("key")?.as_str()?.to_string();
                let v: Value = serde_json::from_str(r.get("value")?.as_str()?).ok()?;
                Some((k, v))
            })
            .collect())
    }

    /// Take `stored` as the saved values and read the environment again (an app that changes saved values
    /// when it starts, such as a one-time migration, reads [`Store::saved`], changes it, then applies it).
    pub fn apply(&self, stored: &HashMap<String, Value>) {
        let mut cache = self.cache.write().unwrap();
        let mut env = self.env.write().unwrap();
        cache.clear();
        env.clear();
        for (key, def) in self.defs.iter() {
            if let Some(name) = &def.env {
                if let Some(raw) = (self.lookup)(name) {
                    let c = clean_env(&raw);
                    if !c.is_empty() {
                        env.insert(key.clone(), coerce(&def.default, &json!(c)));
                    }
                }
            }
            cache.insert(key.clone(), stored.get(key).map(|v| coerce(&def.default, v)).unwrap_or(def.default.clone()));
        }
        drop(cache);
        drop(env);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// The environment variable `name`, as the store reads it.
    pub fn env_var(&self, name: &str) -> Option<String> {
        (self.lookup)(name)
    }

    /// The table the store follows.
    pub fn table(&self) -> Table {
        self.table
    }

    /// The definition of `key`.
    pub fn def(&self, key: &str) -> Option<&Def> {
        self.defs.iter().find(|(k, _)| k == key).map(|(_, d)| d)
    }

    /// The value in effect for `key` (environment first, then saved or default; null for an unknown key).
    pub fn get(&self, key: &str) -> Value {
        if let Some(v) = self.env.read().unwrap().get(key) {
            return v.clone();
        }
        self.cache.read().unwrap().get(key).cloned().unwrap_or(Value::Null)
    }

    /// `key` as text (empty for null).
    pub fn str(&self, key: &str) -> String {
        match self.get(key) {
            Value::String(s) => s,
            Value::Null => String::new(),
            v => v.to_string(),
        }
    }

    /// `key` as a boolean (a non-zero number counts as true).
    pub fn bool(&self, key: &str) -> bool {
        match self.get(key) {
            Value::Bool(b) => b,
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::String(s) => !s.is_empty(),
            _ => false,
        }
    }

    /// `key` as an integer (0 when not a number).
    pub fn int(&self, key: &str) -> i64 {
        self.get(key).as_f64().map(|f| f as i64).unwrap_or(0)
    }

    /// `key` as a real (0 when not a number).
    pub fn float(&self, key: &str) -> f64 {
        self.get(key).as_f64().unwrap_or(0.0)
    }

    /// Whether an environment variable sets `key` (the UI shows it locked).
    pub fn env_locked(&self, key: &str) -> bool {
        self.env.read().unwrap().contains_key(key)
    }

    /// Save `values` (`{key: value}`), each coerced to its setting's type; unknown keys are skipped.
    pub fn set_many(&self, values: &Value) {
        let Some(obj) = values.as_object() else { return };
        for (key, val) in obj {
            let Some(def) = self.def(key) else { continue };
            let v = coerce(&def.default, val);
            let _ = self.db.execute("INSERT OR REPLACE INTO settings VALUES (?,?)", p![key, v.to_string()]);
            self.cache.write().unwrap().insert(key.clone(), v);
        }
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// A number that changes whenever any setting may have changed (a reload, a save). Something worked
    /// out from the settings is still right while this number is the one it was worked out at.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// All settings for the admin UI – secrets only say whether they are set, never their value – except the
    /// secrets in `shown` (for example an API key that the app shows to its owner elsewhere). `_meta` says for each key
    /// where its value comes from (`env`, `ui` when saved, `default`) and its environment variable.
    pub fn dump(&self, shown: &[&str]) -> Value {
        let saved: HashSet<String> = self
            .db
            .query("SELECT key FROM settings", p![])
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| r.get("key")?.as_str().map(String::from))
            .collect();
        let mut out = Map::new();
        let mut meta = Map::new();
        for (key, def) in self.defs.iter() {
            let v = self.get(key);
            let shown = if def.secret && !shown.contains(&key.as_str()) { masked(&v) } else { v };
            out.insert(key.clone(), shown);
            let source = if self.env_locked(key) {
                "env"
            } else if saved.contains(key) {
                "ui"
            } else {
                "default"
            };
            meta.insert(key.clone(), json!({ "source": source, "env": def.env }));
        }
        out.insert("_meta".into(), Value::Object(meta));
        Value::Object(out)
    }
}

impl<D: Db> Values for Store<D> {
    fn table(&self) -> Table {
        self.table
    }
    fn known(&self, key: &str) -> bool {
        self.def(key).is_some()
    }
    fn get(&self, key: &str) -> Value {
        Store::get(self, key)
    }
    fn env_locked(&self, key: &str) -> bool {
        Store::env_locked(self, key)
    }
    fn set_many(&self, values: &Value) {
        Store::set_many(self, values)
    }
    fn dump(&self) -> Value {
        Store::dump(self, &[])
    }
}
