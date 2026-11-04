//! Version numbers of programs and releases, compared by their numeric dot parts (Encore's
//! `encore_core::version`, same answers).

/// The numeric dot parts of a version ("v2.4.10" → [2, 4, 10], "rip, version 2.4.9" → [2, 4, 9]).
pub fn parts(v: &str) -> Vec<u64> {
    let Some(s) = extract_version(v) else { return vec![] };
    s.split('.').map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect()
}

/// The first dotted version number in a text ("rip, version 2.4.9" → "2.4.9", "v2.5.0" → "2.5.0"); a bare
/// number ("v3") counts too.
pub fn extract_version(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            // digits, then one or more ".digits"
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            let mut end = i;
            let mut dots = 0;
            while end + 1 < b.len() && b[end] == b'.' && b[end + 1].is_ascii_digit() {
                end += 1;
                while end < b.len() && b[end].is_ascii_digit() {
                    end += 1;
                }
                dots += 1;
            }
            if dots > 0 {
                return Some(text[start..end].to_string());
            }
            i = end.max(i);
        } else {
            i += 1;
        }
    }
    let t = text.trim().trim_start_matches(['v', 'V']);
    (!t.is_empty() && t.chars().all(|c| c.is_ascii_digit())).then(|| t.to_string())
}

/// Compare two versions by their numeric dot parts (missing parts count as 0).
pub fn compare(a: &str, b: &str) -> std::cmp::Ordering {
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        let o = x.get(i).unwrap_or(&0).cmp(y.get(i).unwrap_or(&0));
        if o != std::cmp::Ordering::Equal {
            return o;
        }
    }
    std::cmp::Ordering::Equal
}

/// `latest` is a version and a higher one than `installed`.
pub fn newer(latest: &str, installed: &str) -> bool {
    !parts(latest).is_empty() && compare(latest, installed) == std::cmp::Ordering::Greater
}

/// MAJOR.MINOR.PATCH of a version: a hotfix 0.36.0.17 is release 0.36.0.
pub fn base(v: &str) -> String {
    let p = parts(v);
    (0..3).map(|i| p.get(i).copied().unwrap_or(0).to_string()).collect::<Vec<_>>().join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering::{Equal, Greater, Less};

    #[test]
    fn versions_compare_by_number() {
        assert_eq!(compare("2026.10.09", "2026.9.30"), Greater);
        assert_eq!(compare("2.4.9", "2.4.10"), Less);
        assert_eq!(compare("v2.5.0", "2.4.10"), Greater);
        assert_eq!(compare("v2.5.0", "2.5"), Equal);
        assert!(newer("2026.10.14", "2026.10.09"));
        assert!(!newer("2.4.9", "2.4.9"));
        assert!(!newer("", "2.4.9"));
        assert_eq!(extract_version("rip, version 2.4.9\n").as_deref(), Some("2.4.9"));
        assert_eq!(extract_version("v2.5.0").as_deref(), Some("2.5.0"));
        assert_eq!(extract_version("Python 3.14.0").as_deref(), Some("3.14.0"));
        assert_eq!(extract_version("v3").as_deref(), Some("3"));
        assert_eq!(extract_version("build 7 of 1.2").as_deref(), Some("1.2"));
        assert_eq!(extract_version("none"), None);
        assert_eq!(parts("0.0.1-dev"), vec![0, 0, 1]);
    }

    #[test]
    fn a_hotfix_is_its_release() {
        assert_eq!(base("0.36.0.17"), "0.36.0");
        assert_eq!(base("0.36"), "0.36.0");
        assert_eq!(base("v1.2.3"), "1.2.3");
    }
}
