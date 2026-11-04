//! [`Registry`]: the app's checks (in groups) and its settings cards' tests.

use crate::Check;
use kit_server_core::{Account, Result};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A boxed future the registry keeps.
type Boxed<T> = Pin<Box<dyn Future<Output = T> + Send>>;
/// A group: the app's state in, its checks out.
type GroupFn<S> = Arc<dyn Fn(S) -> Boxed<Result<Vec<Check>>> + Send + Sync>;
/// A test: the app's state and who pressed Test in, the answer out.
type TestFn<S> = Arc<dyn Fn(S, Account) -> Boxed<TestResult> + Send + Sync>;
/// The tests the app answers itself by name.
type FallbackFn<S> = Arc<dyn Fn(S, String, Account) -> Boxed<Option<TestResult>> + Send + Sync>;

/// A Test button's outcome: the answer (`{ok, message, …}`), or why the test failed on the way (answered as
/// `{ok: false, message}`, see [`test_failed`]).
pub type TestResult = std::result::Result<Value, String>;

/// The answer of a test that failed on the way: `{"ok": false, "message": why}` (Encore's words, never cut).
pub fn test_failed(why: impl Into<String>) -> Value {
    json!({ "ok": false, "message": why.into() })
}

/// The app's checks and tests. Clones share the closures.
pub struct Registry<S> {
    /// the groups of checks, in the order the page lists them
    groups: Vec<(String, GroupFn<S>)>,
    /// the tests by name
    tests: Vec<(String, TestFn<S>)>,
    /// the tests the app looks up itself
    fallback: Option<FallbackFn<S>>,
}

impl<S> Clone for Registry<S> {
    fn clone(&self) -> Self {
        Registry { groups: self.groups.clone(), tests: self.tests.clone(), fallback: self.fallback.clone() }
    }
}

impl<S> Default for Registry<S> {
    fn default() -> Self {
        Registry { groups: vec![], tests: vec![], fallback: None }
    }
}

impl<S: Clone + Send + Sync + 'static> Registry<S> {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a group of checks named `id`: none (a service that is not set up), one or several (every folder).
    /// The page lists the groups in the order they were added; all of them run at once. A group's error fails
    /// the whole page with that error.
    pub fn group<F, Fut>(mut self, id: impl Into<String>, f: F) -> Self
    where
        F: Fn(S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<Check>>> + Send + 'static,
    {
        self.groups.push((id.into(), Arc::new(move |s| Box::pin(f(s)))));
        self
    }

    /// Add one check that always runs.
    pub fn check<F, Fut>(self, id: impl Into<String>, f: F) -> Self
    where
        F: Fn(S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Check> + Send + 'static,
    {
        let f = Arc::new(f);
        self.group(id, move |s| {
            let f = f.clone();
            async move { Ok(vec![f(s).await]) }
        })
    }

    /// Add the Test button `service` of a settings card (`POST /api/system/test/{service}`); the test is run as
    /// the admin who pressed it.
    pub fn test<F, Fut>(mut self, service: impl Into<String>, f: F) -> Self
    where
        F: Fn(S, Account) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = TestResult> + Send + 'static,
    {
        self.tests.push((service.into(), Arc::new(move |s, a| Box::pin(f(s, a)))));
        self
    }

    /// Let the app answer every test not added with [`Registry::test`] itself (Encore's tests are one function
    /// its plugins share); `None` for a service there is no test for.
    pub fn tests<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(S, String, Account) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Option<TestResult>> + Send + 'static,
    {
        self.fallback = Some(Arc::new(move |s, n, a| Box::pin(f(s, n, a))));
        self
    }

    /// The ids of the groups, in order.
    pub fn groups(&self) -> Vec<&str> {
        self.groups.iter().map(|(id, _)| id.as_str()).collect()
    }

    /// Every group at once; their checks in the order the groups were added.
    pub async fn run(&self, st: &S) -> Result<Vec<Check>> {
        let futs = self.groups.iter().map(|(_, f)| f(st.clone()));
        let mut out = vec![];
        for r in futures_util::future::join_all(futs).await {
            out.extend(r?);
        }
        Ok(out)
    }

    /// The checks of the group `id` alone; `None` when there is no such group.
    pub async fn run_group(&self, st: &S, id: &str) -> Option<Result<Vec<Check>>> {
        let (_, f) = self.groups.iter().find(|(g, _)| g == id)?;
        Some(f(st.clone()).await)
    }

    /// The test `service` as `who`; `None` when there is no such test.
    pub async fn run_test(&self, st: &S, service: &str, who: Account) -> Option<TestResult> {
        if let Some((_, f)) = self.tests.iter().find(|(n, _)| n == service) {
            return Some(f(st.clone(), who).await);
        }
        match &self.fallback {
            Some(f) => f(st.clone(), service.to_string(), who).await,
            None => None,
        }
    }
}
