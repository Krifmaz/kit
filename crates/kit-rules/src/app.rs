//! An app's identity, as every kit client needs it: a short id, the name people read and the version.
//!
//! The id prefixes everything an app keeps in the browser or on the device (`<id>.update.tried`,
//! `<id>.offline.me`), the events its web app sends (`<id>:server-down`) and the names its service worker
//! answers with (`<id>:version`, `<id>-shell-…`), so two kit apps served from the same origin never share a key.
//! The name goes into the words (`A new version of {app} is ready`).
//!
//! Words that name the app or a number are templates with `{app}`, `{version}`, `{n}` … in them; [`fill`]
//! puts the values in. Every module's `Words` struct holds its defaults; an app passes its own words where it
//! wants others.

/// Who an app is: its id, its name and its version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    /// The short id: lower case letters, digits and `-` (`kit-demo`). It prefixes storage keys, events and
    /// cache names.
    pub id: String,
    /// The name people read (`Kit Demo`).
    pub name: String,
    /// The version this build is (`0.1.0`), "" when unknown.
    pub version: String,
}

impl App {
    /// An app called `name`, its id made from the name ([`id_of`]), no version yet.
    pub fn named(name: &str) -> Self {
        App { id: id_of(name), name: name.to_string(), version: String::new() }
    }

    /// An app with this `id` and `name`, no version yet.
    pub fn new(id: &str, name: &str) -> Self {
        App { id: id.to_string(), name: name.to_string(), version: String::new() }
    }

    /// The same app at `version`.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// A storage key of this app: `<id>.<name>` (`kit-demo.update.tried`).
    pub fn key(&self, name: &str) -> String {
        key(&self.id, name)
    }

    /// `template` with `{app}` as the app's name and `{version}` as its version.
    pub fn fill(&self, template: &str) -> String {
        fill(template, &[("app", &self.name), ("version", &self.version)])
    }
}

impl Default for App {
    fn default() -> Self {
        App::new("app", "App")
    }
}

/// A storage key of the app with id `id`: `<id>.<name>`.
pub fn key(id: &str, name: &str) -> String {
    format!("{id}.{name}")
}

/// An id made from a name: lower case, every run of other characters one `-`, none at the ends
/// (`Kit Demo` → `kit-demo`); `app` when nothing is left.
pub fn id_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "app".into()
    } else {
        out
    }
}

/// `template` with each `{name}` replaced by its value (`fill("{n} files", &[("n", "3")])` → `3 files`). A
/// name without a value stays as it is.
pub fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_app_names_its_keys_and_fills_its_words() {
        let a = App::named("Kit Demo").with_version("1.2.3");
        assert_eq!(a.id, "kit-demo");
        assert_eq!(a.key("update.tried"), "kit-demo.update.tried");
        assert_eq!(a.fill("{app} {version} is ready"), "Kit Demo 1.2.3 is ready");
        assert_eq!(App::new("fv", "FSView").key("offline.me"), "fv.offline.me");
        assert_eq!(App::default().id, "app");
    }

    #[test]
    fn ids_are_lower_case_with_dashes() {
        assert_eq!(id_of("  My  App 2!"), "my-app-2");
        assert_eq!(id_of("Photovault"), "photovault");
        assert_eq!(id_of("…"), "app");
    }

    #[test]
    fn fill_leaves_unknown_names() {
        assert_eq!(fill("{n} of {total}", &[("n", "2")]), "2 of {total}");
        assert_eq!(fill("no names", &[("n", "2")]), "no names");
    }
}
