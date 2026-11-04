//! [`Store`]: the values of a settings table. The value in effect is the environment variable's, then the saved
//! one, then the default. Saved values are kept behind [`SettingsStore`]: [`SqliteSettings`] or, with feature
//! `postgres`, [`PgSettings`] keep them in the kit's own `kit_settings` table (`key`, `value` as JSON text);
//! an app may keep them anywhere else by implementing the trait.

use crate::table::Table;
use crate::values::{clean_env, coerce, masked, Def, EnvLookup, Values};
use kit_server_core::{async_trait, p, Db, Migrations, Result, RowExt, SqliteDb};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// Where the saved values are kept: one JSON value per setting key.
#[async_trait]
pub trait SettingsStore: Send + Sync {
    /// Every saved value (a value that can't be read as JSON is left out).
    async fn saved(&self) -> Result<HashMap<String, Value>>;
    /// Keep each `(key, value)`, replacing what was saved for that key.
    async fn save(&self, values: &[(String, Value)]) -> Result<()>;
}

/// The SQLite steps of `kit_settings`.
pub const SQLITE: Migrations = Migrations {
    component: "settings",
    steps: &["CREATE TABLE kit_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at REAL NOT NULL DEFAULT 0)"],
};

/// The Postgres steps of `kit_settings`.
pub const POSTGRES: Migrations = Migrations {
    component: "settings",
    steps: &["CREATE TABLE kit_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at DOUBLE PRECISION NOT NULL DEFAULT 0)"],
};

/// Saved rows as values: rows whose value is not JSON are left out.
fn rows_to_saved(rows: Vec<kit_server_core::Row>) -> HashMap<String, Value> {
    rows.into_iter().filter_map(|r| Some((r.s("key"), serde_json::from_str(&r.s("value")).ok()?))).collect()
}

/// The saved values in SQLite, in `kit_settings`.
#[derive(Clone)]
pub struct SqliteSettings {
    /// the database
    db: SqliteDb,
}

impl SqliteSettings {
    /// The saved values on `db`, its table made or brought up to date.
    pub fn new(db: SqliteDb) -> Result<SqliteSettings> {
        db.migrate(&SQLITE)?;
        Ok(SqliteSettings { db })
    }
}

#[async_trait]
impl SettingsStore for SqliteSettings {
    async fn saved(&self) -> Result<HashMap<String, Value>> {
        Ok(rows_to_saved(self.db.query("SELECT key, value FROM kit_settings", p![])?))
    }

    async fn save(&self, values: &[(String, Value)]) -> Result<()> {
        let now = kit_server_core::db::now();
        for (k, v) in values {
            self.db.execute(
                "INSERT INTO kit_settings (key, value, updated_at) VALUES (?, ?, ?)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                p![k, v.to_string(), now],
            )?;
        }
        Ok(())
    }
}

/// The saved values in Postgres, in `kit_settings` (feature `postgres`).
#[cfg(feature = "postgres")]
#[derive(Clone, Debug)]
pub struct PgSettings {
    /// the database
    db: kit_server_core::pg::PgDb,
}

#[cfg(feature = "postgres")]
impl PgSettings {
    /// The saved values on `db`, its table made or brought up to date.
    pub async fn new(db: kit_server_core::pg::PgDb) -> Result<PgSettings> {
        db.migrate(&POSTGRES).await?;
        Ok(PgSettings { db })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SettingsStore for PgSettings {
    async fn saved(&self) -> Result<HashMap<String, Value>> {
        Ok(rows_to_saved(self.db.query("SELECT key, value FROM kit_settings", p![]).await?))
    }

    async fn save(&self, values: &[(String, Value)]) -> Result<()> {
        let now = kit_server_core::db::now();
        for (k, v) in values {
            self.db
                .execute(
                    "INSERT INTO kit_settings (key, value, updated_at) VALUES ($1, $2, $3)
                     ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
                    p![k, v.to_string(), now],
                )
                .await?;
        }
        Ok(())
    }
}

/// What every [`SettingsStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
pub async fn settings_contract(store: &dyn SettingsStore) {
    assert!(store.saved().await.unwrap().is_empty());
    store.save(&[("a".into(), json!("x")), ("n".into(), json!(3)), ("on".into(), json!(true))]).await.unwrap();
    store.save(&[("a".into(), json!({ "k": [1, 2] }))]).await.unwrap();
    let saved = store.saved().await.unwrap();
    assert_eq!(saved.len(), 3);
    assert_eq!((saved["a"].clone(), saved["n"].clone(), saved["on"].clone()), (json!({ "k": [1, 2] }), json!(3), json!(true)));
}

/// The settings in effect: defaults, then the values saved in the UI, then the environment. Clones share them.
#[derive(Clone)]
pub struct Store {
    /// every setting's definition, in the table's order
    pub defs: Arc<Vec<(String, Def)>>,
    /// the table the store follows
    table: Table,
    /// the saved value (or the default) of every key
    cache: Arc<RwLock<HashMap<String, Value>>>,
    /// the keys that have a saved value
    saved_keys: Arc<RwLock<HashSet<String>>>,
    /// the keys an environment variable sets, with its value
    env: Arc<RwLock<HashMap<String, Value>>>,
    /// moves on every reload and every save, so what is worked out from the settings can be kept until then
    generation: Arc<AtomicU64>,
    /// where the saved values are
    backend: Arc<dyn SettingsStore>,
    /// reads an environment variable (on every reload)
    lookup: EnvLookup,
}

impl Store {
    /// A store of `table` on `backend` with the environment variables `lookup` gives, nothing read yet (every
    /// key reads as null until [`Store::reload`] or [`Store::apply`]).
    pub fn new(
        backend: impl SettingsStore + 'static,
        table: Table,
        lookup: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Store {
        let defs: Vec<(String, Def)> = table
            .keys
            .iter()
            .map(|d| (d.key.to_string(), Def { env: d.env.map(String::from), default: d.default.to_json(), secret: d.secret }))
            .collect();
        Store {
            defs: Arc::new(defs),
            table,
            cache: Default::default(),
            saved_keys: Default::default(),
            env: Default::default(),
            generation: Default::default(),
            backend: Arc::new(backend),
            lookup: Arc::new(lookup),
        }
    }

    /// The values of `table` saved in `backend`, with the process's environment variables over them.
    pub async fn load(backend: impl SettingsStore + 'static, table: Table) -> Result<Store> {
        Store::load_with(backend, table, |name| std::env::var(name).ok()).await
    }

    /// [`Store::load`] with the environment variables `lookup` gives (a test's own set).
    pub async fn load_with(
        backend: impl SettingsStore + 'static,
        table: Table,
        lookup: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Result<Store> {
        let store = Store::new(backend, table, lookup);
        store.reload().await?;
        Ok(store)
    }

    /// Read the saved values and the environment again.
    pub async fn reload(&self) -> Result<()> {
        let saved = self.saved().await?;
        self.apply(&saved);
        Ok(())
    }

    /// The saved values, as stored (values that are not JSON are left out).
    pub async fn saved(&self) -> Result<HashMap<String, Value>> {
        self.backend.saved().await
    }

    /// Take `stored` as the saved values and read the environment again (an app that changes saved values
    /// when it starts, such as a one-time migration, reads [`Store::saved`], changes it, then applies it).
    pub fn apply(&self, stored: &HashMap<String, Value>) {
        let mut cache = self.cache.write().unwrap();
        let mut env = self.env.write().unwrap();
        cache.clear();
        env.clear();
        *self.saved_keys.write().unwrap() = stored.keys().cloned().collect();
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

    /// Save `values` (`{key: value}`), each coerced to its setting's type; unknown keys are skipped. The values
    /// in effect change only once the backend kept them.
    pub async fn set_many(&self, values: &Value) -> Result<()> {
        let Some(obj) = values.as_object() else { return Ok(()) };
        let clean: Vec<(String, Value)> =
            obj.iter().filter_map(|(key, val)| Some((key.clone(), coerce(&self.def(key)?.default, val)))).collect();
        if clean.is_empty() {
            return Ok(());
        }
        self.backend.save(&clean).await?;
        let mut cache = self.cache.write().unwrap();
        let mut saved = self.saved_keys.write().unwrap();
        for (key, v) in clean {
            saved.insert(key.clone());
            cache.insert(key, v);
        }
        drop(cache);
        drop(saved);
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
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
        let saved = self.saved_keys.read().unwrap().clone();
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

#[async_trait]
impl Values for Store {
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
    async fn set_many(&self, values: &Value) -> Result<()> {
        Store::set_many(self, values).await
    }
    fn dump(&self) -> Value {
        Store::dump(self, &[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_keeps_the_contract() {
        settings_contract(&SqliteSettings::new(SqliteDb::memory()).unwrap()).await;
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn postgres_keeps_the_contract() {
        settings_contract(&PgSettings::new(kit_server_core::pg::test_db().await).await.unwrap()).await;
    }
}
