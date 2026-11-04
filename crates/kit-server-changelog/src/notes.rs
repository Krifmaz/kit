//! Release notes as parts, and which ones are new for an account: kit-rules `changelog`, so the server and
//! every client read notes the same way.

pub use kit_rules::changelog::{parse, plain, unseen, Block, Release, HIGHLIGHTS, POPUP_RELEASES};
pub use kit_server_core::version::base;

#[cfg(test)]
#[path = "notes_tests.rs"]
mod tests;
