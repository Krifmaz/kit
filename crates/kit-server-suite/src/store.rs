//! [`SuiteStore`]: the apps this one links to ([`Link`]) and the keys other apps link here with ([`Key`]).
//! [`SqliteSuite`] or, with feature `postgres`, `PgSuite` keep them in the kit's own `kit_suite_links` and
//! `kit_suite_keys` tables; an app may implement the trait on its own storage.

use kit_rules::suite::Manifest;
use kit_server_core::{async_trait, p, Db, Migrations, Result, Row, RowExt, SqliteDb};

/// An app this one links to.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// The link's id (0 before it is kept).
    pub id: i64,
    /// The other app's base URL (normalized: no trailing `/`).
    pub base_url: String,
    /// The key the other app made for this one; sent as `Authorization: Bearer <key>`, never shown again.
    pub api_key: String,
    /// People sign in here through the other app (at most one link is).
    pub account_source: bool,
    /// The other app's manifest, as last read.
    pub manifest: Option<Manifest>,
    /// Why the last read failed, in one sentence; None when it worked.
    pub error: Option<String>,
    /// When the manifest was last read (seconds since 1970).
    pub checked_at: f64,
    /// When the link was made.
    pub created_at: f64,
}

impl Link {
    /// The other app's name as last read, its address otherwise.
    pub fn name(&self) -> String {
        self.manifest.as_ref().map(|m| m.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| self.base_url.clone())
    }

    /// The link as admins see it: everything but the key.
    pub fn public(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "base_url": self.base_url,
            "name": self.name(),
            "account_source": self.account_source,
            "manifest": self.manifest,
            "error": self.error,
            "ok": self.error.is_none(),
            "checked_at": self.checked_at,
            "created_at": self.created_at,
        })
    }
}

/// A key this app made for another app, kept as the SHA-256 of its secret half.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    /// The public half (`ks_<12 hex>`): the OIDC client id.
    pub id: String,
    /// What the admin called it (usually the other app's name).
    pub name: String,
    /// `sha256:<hex>` of the secret half.
    pub secret_hash: String,
    /// The other app's base URL: sign-in may only send people back under it. None: no sign-in through this key.
    pub redirect_url: Option<String>,
    /// When it was made.
    pub created_at: f64,
    /// When it was last used, 0 for never.
    pub last_used_at: f64,
}

impl Key {
    /// The key as admins see it: everything but the hash.
    pub fn public(&self) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "name": self.name,
            "redirect_url": self.redirect_url,
            "created_at": self.created_at,
            "last_used_at": self.last_used_at,
        })
    }
}

/// Where the links and keys are kept.
#[async_trait]
pub trait SuiteStore: Send + Sync {
    /// Every link, oldest first.
    async fn links(&self) -> Result<Vec<Link>>;
    /// Link `id`, if there is one.
    async fn link(&self, id: i64) -> Result<Option<Link>>;
    /// Keep a new link; its id.
    async fn add_link(&self, link: &Link) -> Result<i64>;
    /// Keep `link`'s key, manifest, error and checked time (by its id).
    async fn update_link(&self, link: &Link) -> Result<()>;
    /// Make link `id` the one account source (None: no link is).
    async fn set_account_source(&self, id: Option<i64>) -> Result<()>;
    /// Remove link `id`; whether there was one.
    async fn remove_link(&self, id: i64) -> Result<bool>;
    /// Every key, oldest first.
    async fn keys(&self) -> Result<Vec<Key>>;
    /// Key `id`, if there is one.
    async fn key(&self, id: &str) -> Result<Option<Key>>;
    /// Keep a new key.
    async fn add_key(&self, key: &Key) -> Result<()>;
    /// Key `id` was used at `at`.
    async fn touch_key(&self, id: &str, at: f64) -> Result<()>;
    /// Remove key `id`; whether there was one.
    async fn remove_key(&self, id: &str) -> Result<bool>;
}

/// The SQLite steps of `kit_suite_links` and `kit_suite_keys`.
pub const SQLITE: Migrations = Migrations {
    component: "suite",
    steps: &["CREATE TABLE kit_suite_links (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            base_url TEXT NOT NULL UNIQUE,
            api_key TEXT NOT NULL,
            account_source INTEGER NOT NULL DEFAULT 0,
            manifest TEXT,
            error TEXT,
            checked_at REAL NOT NULL DEFAULT 0,
            created_at REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE kit_suite_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            secret_hash TEXT NOT NULL,
            redirect_url TEXT,
            created_at REAL NOT NULL DEFAULT 0,
            last_used_at REAL NOT NULL DEFAULT 0
        );"],
};

/// The Postgres steps of `kit_suite_links` and `kit_suite_keys`.
pub const POSTGRES: Migrations = Migrations {
    component: "suite",
    steps: &["CREATE TABLE kit_suite_links (
            id BIGSERIAL PRIMARY KEY,
            base_url TEXT NOT NULL UNIQUE,
            api_key TEXT NOT NULL,
            account_source BOOLEAN NOT NULL DEFAULT FALSE,
            manifest TEXT,
            error TEXT,
            checked_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            created_at DOUBLE PRECISION NOT NULL DEFAULT 0
        );
        CREATE TABLE kit_suite_keys (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            secret_hash TEXT NOT NULL,
            redirect_url TEXT,
            created_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            last_used_at DOUBLE PRECISION NOT NULL DEFAULT 0
        );"],
};

/// The columns of a link row, in the order every query selects them.
const LINK_COLS: &str = "id, base_url, api_key, account_source, manifest, error, checked_at, created_at";
/// The columns of a key row.
const KEY_COLS: &str = "id, name, secret_hash, redirect_url, created_at, last_used_at";

/// Text or None for an empty column.
fn opt(r: &Row, k: &str) -> Option<String> {
    Some(r.s(k)).filter(|s| !s.is_empty())
}

/// A link from its row (a manifest that no longer parses reads as none).
fn link_of(r: &Row) -> Link {
    Link {
        id: r.i("id"),
        base_url: r.s("base_url"),
        api_key: r.s("api_key"),
        account_source: r.b("account_source"),
        manifest: opt(r, "manifest").and_then(|t| serde_json::from_str(&t).ok()),
        error: opt(r, "error"),
        checked_at: r.f("checked_at"),
        created_at: r.f("created_at"),
    }
}

/// A key from its row.
fn key_of(r: &Row) -> Key {
    Key {
        id: r.s("id"),
        name: r.s("name"),
        secret_hash: r.s("secret_hash"),
        redirect_url: opt(r, "redirect_url"),
        created_at: r.f("created_at"),
        last_used_at: r.f("last_used_at"),
    }
}

/// A manifest as the text a row keeps (null for none).
fn manifest_text(m: &Option<Manifest>) -> serde_json::Value {
    match m {
        Some(m) => serde_json::Value::String(serde_json::to_string(m).unwrap_or_default()),
        None => serde_json::Value::Null,
    }
}

/// The links and keys in SQLite.
#[derive(Clone)]
pub struct SqliteSuite {
    /// the database
    db: SqliteDb,
}

impl SqliteSuite {
    /// The links and keys on `db`, their tables made or brought up to date.
    pub fn new(db: SqliteDb) -> Result<SqliteSuite> {
        db.migrate(&SQLITE)?;
        Ok(SqliteSuite { db })
    }
}

#[async_trait]
impl SuiteStore for SqliteSuite {
    async fn links(&self) -> Result<Vec<Link>> {
        Ok(self.db.query(&format!("SELECT {LINK_COLS} FROM kit_suite_links ORDER BY id"), p![])?.iter().map(link_of).collect())
    }

    async fn link(&self, id: i64) -> Result<Option<Link>> {
        Ok(self.db.one(&format!("SELECT {LINK_COLS} FROM kit_suite_links WHERE id = ?"), p![id])?.as_ref().map(link_of))
    }

    async fn add_link(&self, l: &Link) -> Result<i64> {
        self.db.execute(
            "INSERT INTO kit_suite_links (base_url, api_key, account_source, manifest, error, checked_at, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
            vec![
                l.base_url.clone().into(),
                l.api_key.clone().into(),
                l.account_source.into(),
                manifest_text(&l.manifest),
                l.error.clone().into(),
                l.checked_at.into(),
                l.created_at.into(),
            ],
        )
    }

    async fn update_link(&self, l: &Link) -> Result<()> {
        self.db.execute(
            "UPDATE kit_suite_links SET api_key = ?, manifest = ?, error = ?, checked_at = ? WHERE id = ?",
            vec![l.api_key.clone().into(), manifest_text(&l.manifest), l.error.clone().into(), l.checked_at.into(), l.id.into()],
        )?;
        Ok(())
    }

    async fn set_account_source(&self, id: Option<i64>) -> Result<()> {
        self.db.execute("UPDATE kit_suite_links SET account_source = CASE WHEN id = ? THEN 1 ELSE 0 END", p![id.unwrap_or(0)])?;
        Ok(())
    }

    async fn remove_link(&self, id: i64) -> Result<bool> {
        let had = self.link(id).await?.is_some();
        self.db.execute("DELETE FROM kit_suite_links WHERE id = ?", p![id])?;
        Ok(had)
    }

    async fn keys(&self) -> Result<Vec<Key>> {
        Ok(self.db.query(&format!("SELECT {KEY_COLS} FROM kit_suite_keys ORDER BY created_at, id"), p![])?.iter().map(key_of).collect())
    }

    async fn key(&self, id: &str) -> Result<Option<Key>> {
        Ok(self.db.one(&format!("SELECT {KEY_COLS} FROM kit_suite_keys WHERE id = ?"), p![id])?.as_ref().map(key_of))
    }

    async fn add_key(&self, k: &Key) -> Result<()> {
        self.db.execute(
            "INSERT INTO kit_suite_keys (id, name, secret_hash, redirect_url, created_at, last_used_at) VALUES (?, ?, ?, ?, ?, ?)",
            p![k.id, k.name, k.secret_hash, k.redirect_url, k.created_at, k.last_used_at],
        )?;
        Ok(())
    }

    async fn touch_key(&self, id: &str, at: f64) -> Result<()> {
        self.db.execute("UPDATE kit_suite_keys SET last_used_at = ? WHERE id = ?", p![at, id])?;
        Ok(())
    }

    async fn remove_key(&self, id: &str) -> Result<bool> {
        let had = self.key(id).await?.is_some();
        self.db.execute("DELETE FROM kit_suite_keys WHERE id = ?", p![id])?;
        Ok(had)
    }
}

/// The links and keys in Postgres (feature `postgres`).
#[cfg(feature = "postgres")]
#[derive(Clone, Debug)]
pub struct PgSuite {
    /// the database
    db: kit_server_core::pg::PgDb,
}

#[cfg(feature = "postgres")]
impl PgSuite {
    /// The links and keys on `db`, their tables made or brought up to date.
    pub async fn new(db: kit_server_core::pg::PgDb) -> Result<PgSuite> {
        db.migrate(&POSTGRES).await?;
        Ok(PgSuite { db })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl SuiteStore for PgSuite {
    async fn links(&self) -> Result<Vec<Link>> {
        Ok(self.db.query(&format!("SELECT {LINK_COLS} FROM kit_suite_links ORDER BY id"), p![]).await?.iter().map(link_of).collect())
    }

    async fn link(&self, id: i64) -> Result<Option<Link>> {
        Ok(self.db.one(&format!("SELECT {LINK_COLS} FROM kit_suite_links WHERE id = $1"), p![id]).await?.as_ref().map(link_of))
    }

    async fn add_link(&self, l: &Link) -> Result<i64> {
        let row = self
            .db
            .one(
                "INSERT INTO kit_suite_links (base_url, api_key, account_source, manifest, error, checked_at, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
                vec![
                    l.base_url.clone().into(),
                    l.api_key.clone().into(),
                    l.account_source.into(),
                    manifest_text(&l.manifest),
                    l.error.clone().into(),
                    l.checked_at.into(),
                    l.created_at.into(),
                ],
            )
            .await?;
        Ok(row.map(|r| r.i("id")).unwrap_or(0))
    }

    async fn update_link(&self, l: &Link) -> Result<()> {
        self.db
            .execute(
                "UPDATE kit_suite_links SET api_key = $1, manifest = $2, error = $3, checked_at = $4 WHERE id = $5",
                vec![l.api_key.clone().into(), manifest_text(&l.manifest), l.error.clone().into(), l.checked_at.into(), l.id.into()],
            )
            .await?;
        Ok(())
    }

    async fn set_account_source(&self, id: Option<i64>) -> Result<()> {
        self.db.execute("UPDATE kit_suite_links SET account_source = (id = $1)", p![id.unwrap_or(0)]).await?;
        Ok(())
    }

    async fn remove_link(&self, id: i64) -> Result<bool> {
        Ok(self.db.execute("DELETE FROM kit_suite_links WHERE id = $1", p![id]).await? > 0)
    }

    async fn keys(&self) -> Result<Vec<Key>> {
        Ok(self
            .db
            .query(&format!("SELECT {KEY_COLS} FROM kit_suite_keys ORDER BY created_at, id"), p![])
            .await?
            .iter()
            .map(key_of)
            .collect())
    }

    async fn key(&self, id: &str) -> Result<Option<Key>> {
        Ok(self.db.one(&format!("SELECT {KEY_COLS} FROM kit_suite_keys WHERE id = $1"), p![id]).await?.as_ref().map(key_of))
    }

    async fn add_key(&self, k: &Key) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO kit_suite_keys (id, name, secret_hash, redirect_url, created_at, last_used_at) VALUES ($1, $2, $3, $4, $5, $6)",
                p![k.id, k.name, k.secret_hash, k.redirect_url, k.created_at, k.last_used_at],
            )
            .await?;
        Ok(())
    }

    async fn touch_key(&self, id: &str, at: f64) -> Result<()> {
        self.db.execute("UPDATE kit_suite_keys SET last_used_at = $1 WHERE id = $2", p![at, id]).await?;
        Ok(())
    }

    async fn remove_key(&self, id: &str) -> Result<bool> {
        Ok(self.db.execute("DELETE FROM kit_suite_keys WHERE id = $1", p![id]).await? > 0)
    }
}

/// What every [`SuiteStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
pub async fn suite_contract(store: &dyn SuiteStore) {
    use kit_rules::suite::Section;
    assert!(store.links().await.unwrap().is_empty());
    let m = Manifest {
        kit_suite: 1,
        id: "two".into(),
        name: "Two".into(),
        base_url: "https://b.test".into(),
        sections: vec![Section { id: "s".into(), title: "S".into(), icon: "M0 0".into(), path: "/s".into() }],
        ..Manifest::default()
    };
    let mut l = Link {
        id: 0,
        base_url: "https://b.test".into(),
        api_key: "ks_a.b".into(),
        account_source: false,
        manifest: Some(m.clone()),
        error: None,
        checked_at: 5.0,
        created_at: 5.0,
    };
    l.id = store.add_link(&l).await.unwrap();
    assert!(l.id > 0);
    let other = Link { base_url: "https://c.test".into(), manifest: None, error: Some("Down".into()), ..l.clone() };
    let other_id = store.add_link(&other).await.unwrap();
    assert!(store.add_link(&l).await.is_err(), "one link per address");
    assert_eq!(store.link(l.id).await.unwrap().unwrap(), l);
    assert_eq!(store.link(999).await.unwrap(), None);

    l.manifest = Some(Manifest { name: "Two again".into(), ..m });
    l.error = Some("Timed out".into());
    l.api_key = "ks_c.d".into();
    l.checked_at = 9.0;
    store.update_link(&l).await.unwrap();
    assert_eq!(store.link(l.id).await.unwrap().unwrap(), l);

    store.set_account_source(Some(other_id)).await.unwrap();
    store.set_account_source(Some(l.id)).await.unwrap();
    let all = store.links().await.unwrap();
    assert_eq!(all.iter().map(|x| (x.id, x.account_source)).collect::<Vec<_>>(), [(l.id, true), (other_id, false)]);
    assert_eq!(all[1].error.as_deref(), Some("Down"));
    store.set_account_source(None).await.unwrap();
    assert!(store.links().await.unwrap().iter().all(|x| !x.account_source));
    assert!(store.remove_link(other_id).await.unwrap());
    assert!(!store.remove_link(other_id).await.unwrap());
    assert_eq!(store.links().await.unwrap().len(), 1);

    assert!(store.keys().await.unwrap().is_empty());
    let k = Key {
        id: "ks_0123456789ab".into(),
        name: "Other".into(),
        secret_hash: "sha256:00".into(),
        redirect_url: Some("https://c.test".into()),
        created_at: 1.0,
        last_used_at: 0.0,
    };
    store.add_key(&k).await.unwrap();
    store.add_key(&Key { id: "ks_2".into(), redirect_url: None, created_at: 2.0, ..k.clone() }).await.unwrap();
    assert_eq!(store.key(&k.id).await.unwrap().unwrap(), k);
    store.touch_key(&k.id, 7.0).await.unwrap();
    assert_eq!(store.key(&k.id).await.unwrap().unwrap().last_used_at, 7.0);
    let keys = store.keys().await.unwrap();
    assert_eq!(keys.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), [k.id.as_str(), "ks_2"]);
    assert_eq!(keys[1].redirect_url, None);
    assert!(store.remove_key("ks_2").await.unwrap());
    assert!(!store.remove_key("ks_2").await.unwrap());
    assert_eq!(store.key("ks_2").await.unwrap(), None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_keeps_the_contract() {
        suite_contract(&SqliteSuite::new(SqliteDb::memory()).unwrap()).await;
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn postgres_keeps_the_contract() {
        suite_contract(&PgSuite::new(kit_server_core::pg::test_db().await).await.unwrap()).await;
    }
}
