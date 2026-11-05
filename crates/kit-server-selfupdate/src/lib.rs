//! "Update now" for a kit app's server running in Docker, over the Docker socket mounted into its container.
//!
//! An update runs in two places. The server checks, pulls the new image and starts a helper container of it
//! ([`start::start_update`]: steps `check`, `pull`, `helper`); the helper, the NEW image started as
//! `<binary> self-update-helper …` ([`swap::main_if_helper`]), swaps the server's container the way
//! Watchtower does ([`swap::run_helper`]: `inspect`, `schema`, `stop`, `rename`, `create`, `start`,
//! `verify`, `cleanup`) and rolls back when the new one does not keep running. The helper prints one line
//! per step; the server that comes back reads them into the [`report::Report`] of the last update
//! ([`report::read_back`]) and checks its own version (step `verify`).
//!
//! - [`docker`]: a small Docker Engine API client on the unix socket (HTTP/1.1 through hyper).
//! - [`probe`]: whether Update now can work here, and in plain words why not.
//! - [`start`]: the server's part: the tag check, the pull and the helper's start.
//! - [`swap`]: the helper's part, with its pure planners (which settings carry over to the new container).
//! - [`report`]: the step-by-step record of the last update; where it is kept is the app's.
//!
//! What belongs to the app comes in as an [`App`]: its name, image, the program inside the image and the
//! checks only it knows (its database schema, what its exit codes mean, whether its log shows a migration).
//!
//! ```ignore
//! let app = kit_server_selfupdate::App::new("Demo", "ghcr.io/acme/demo", "/usr/local/bin/demo")
//!     .version(option_env!("DEMO_VERSION"))
//!     .schema(Some(db::SCHEMA_VERSION));
//! // first thing in main(): the program may have been started as the helper
//! if let Some(code) = kit_server_selfupdate::swap::main_if_helper(&app).await {
//!     std::process::exit(code);
//! }
//! ```

pub mod docker;
pub mod probe;
pub mod report;
pub mod start;
pub mod swap;

pub use docker::Docker;
pub use report::{Report, State};

/// Reads a database migration out of a server's log: `Some((from, to))` when the log shows it began
/// migrating its database from schema `from` to `to`.
pub type MigrationInLog = fn(&str) -> Option<(i64, i64)>;

/// What the self-update needs to know about the app. [`App::new`] gives neutral defaults for the rest.
#[derive(Clone, Debug)]
pub struct App {
    /// the app's name in the words people read ("Demo")
    pub name: String,
    /// the only image the app updates itself to, without a tag (`ghcr.io/acme/demo`)
    pub image: String,
    /// the program inside the image (`/usr/local/bin/demo`); the helper is started as
    /// `<binary> self-update-helper …`
    pub binary: String,
    /// the label key on the helper's container, so it can be found again; default `org.<program>.role`
    pub label: String,
    /// what every helper line starts with; default `<program> self-update: `
    pub line_prefix: String,
    /// this build's release version (`0.4.2`); None in a build without one, which is not checked
    pub version: Option<String>,
    /// the database schema this build knows; None for an app without one, which is not checked
    pub schema: Option<i64>,
    /// the oldest release whose image has the helper in it; older tags are refused
    pub first_self_updating: Option<String>,
    /// where the backups taken before a database migration are, for the way back by hand (`/config/backups`)
    pub backups: Option<String>,
    /// what the app's own exit codes mean (the code it exits with when it refuses a newer database, …)
    pub exit_meanings: Vec<(i32, String)>,
    /// whether a server's log shows it began migrating its database
    pub migration_in_log: Option<MigrationInLog>,
}

impl App {
    /// The app `name` updating itself to `image` (no tag), whose program in the image is `binary`.
    pub fn new(name: impl Into<String>, image: impl Into<String>, binary: impl Into<String>) -> App {
        let binary = binary.into();
        let program = binary.rsplit('/').next().unwrap_or(&binary).to_string();
        App {
            name: name.into(),
            image: image.into(),
            label: format!("org.{program}.role"),
            line_prefix: format!("{program} self-update: "),
            binary,
            version: None,
            schema: None,
            first_self_updating: None,
            backups: None,
            exit_meanings: vec![],
            migration_in_log: None,
        }
    }

    /// This build's release version (None: a build without one).
    pub fn version(mut self, v: Option<&str>) -> App {
        self.version = v.map(String::from);
        self
    }

    /// The database schema this build knows.
    pub fn schema(mut self, s: Option<i64>) -> App {
        self.schema = s;
        self
    }

    /// The oldest release whose image has the helper in it.
    pub fn first_self_updating(mut self, v: impl Into<String>) -> App {
        self.first_self_updating = Some(v.into());
        self
    }

    /// Where the backups taken before a migration are.
    pub fn backups(mut self, path: impl Into<String>) -> App {
        self.backups = Some(path.into());
        self
    }

    /// What exit code `code` of the app's server means.
    pub fn exit_meaning(mut self, code: i32, meaning: impl Into<String>) -> App {
        self.exit_meanings.push((code, meaning.into()));
        self
    }

    /// How to tell a migration from the server's log.
    pub fn migration_in_log(mut self, f: MigrationInLog) -> App {
        self.migration_in_log = Some(f);
        self
    }

    /// The label key on the helper's container (default `org.<program>.role`).
    pub fn label(mut self, key: impl Into<String>) -> App {
        self.label = key.into();
        self
    }

    /// What every helper line starts with (default `<program> self-update: `).
    pub fn line_prefix(mut self, prefix: impl Into<String>) -> App {
        self.line_prefix = prefix.into();
        self
    }

    /// The program's file name (`demo` for `/usr/local/bin/demo`).
    pub fn program(&self) -> &str {
        self.binary.rsplit('/').next().unwrap_or(&self.binary)
    }
}

/// Seconds since 1970.
pub(crate) fn now() -> f64 {
    kit_server_core::db::now()
}
