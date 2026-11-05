//! Invite and confirmation tokens: random, URL-safe, and kept only as their SHA-256.

use sha2::{Digest, Sha256};

/// A new token: 32 random bytes as 64 hex digits (safe in a link, easy to copy whole).
pub fn new_token() -> String {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).expect("the system's random numbers");
    hex(&b)
}

/// What the tables keep of `token`: its SHA-256 as hex, so the database never holds a working link.
pub fn hash(token: &str) -> String {
    hex(&Sha256::digest(token.trim().as_bytes()))
}

/// Bytes as lower-case hex.
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_kept_as_hashes() {
        let (a, b) = (new_token(), new_token());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(hash("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(hash(" abc\n"), hash("abc"), "a pasted token's spaces don't count");
        assert_ne!(hash(&a), a);
    }
}
