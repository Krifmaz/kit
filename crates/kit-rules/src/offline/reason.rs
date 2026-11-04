//! Why the server can't be reached, in words the person can act on.
//!
//! From Encore's Android `features/offline/OfflineReason.kt` (`OfflineReason.of`, `NO_NETWORK`, `UNKNOWN`; tests
//! in `OfflineReasonTest.kt`) and `OfflineMode.kt` (`probe`, `unreachable`), and the web's
//! `web-rs/src/offline/mode.rs` `unreachable`.
//!
//! Kotlin matches exception types (`UnknownHostException`, `InterruptedIOException`, `ConnectException`, …);
//! here the platform turns its error into a [`Failure`] first, so the words are the same on every platform.

/// The phone (device) itself has no network. From `OfflineReason.NO_NETWORK`.
pub const NO_NETWORK: &str = "This phone has no network connection.";
/// Nothing is known yet (offline mode started before a check finished). From `OfflineReason.UNKNOWN`.
pub const UNKNOWN: &str = "The server didn't answer.";
/// The words for a name that does not resolve. From `OfflineReason.of` (`UnknownHostException`).
pub const DNS: &str = "The server's address couldn't be found (DNS). Check the address in Settings and the phone's network.";
/// The words for a refused connection or no route. From `OfflineReason.of` (`ConnectException`,
/// `NoRouteToHostException`).
pub const REFUSED: &str =
    "Nothing answered at the server's address (connection refused or no route). Is the server, or its proxy, running and reachable from this network?";
/// The words for a broken HTTPS setup. From `OfflineReason.of` (`SSLException`).
pub const TLS: &str = "The secure (HTTPS) connection failed: check the server's certificate.";

/// How a call that got no HTTP answer failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The address does not resolve (Kotlin `UnknownHostException`).
    Dns,
    /// No answer in time (a socket timeout, or a whole-call timeout: Kotlin `InterruptedIOException`).
    Timeout,
    /// Connection refused or no route (Kotlin `ConnectException`, `NoRouteToHostException`).
    Refused,
    /// The HTTPS handshake or certificate failed (Kotlin `SSLException`).
    Tls,
    /// Anything else, with its message (may be blank).
    Other(String),
}

/// The reason for one failed health check: one or two short sentences, never empty.
///
/// - `device_online`: whether the device has a network at all;
/// - `code`: the HTTP status the server (or a proxy in front of it) answered, when it answered at all — it wins
///   over any failure (the proxy answered, the server behind it didn't);
/// - `failure`: how the call failed, when it got no HTTP answer;
/// - `seconds`: how long the check waited, for the timeout's words;
/// - `app`: the app behind the proxy ("Encore").
///
/// From `OfflineReason.of`.
pub fn reason(app: &str, device_online: bool, code: Option<u16>, failure: Option<&Failure>, seconds: u64) -> String {
    if !device_online {
        return NO_NETWORK.to_string();
    }
    if let Some(code) = code {
        return match code {
            502..=504 => format!(
                "The proxy in front of the server answered HTTP {code}: it can't reach {app} behind it. If the container was just recreated, restart the proxy or point it at the container by name."
            ),
            _ => format!("The server answered HTTP {code} to the health check."),
        };
    }
    match failure {
        Some(Failure::Dns) => DNS.to_string(),
        Some(Failure::Timeout) => format!("The server didn't answer within {seconds} s."),
        Some(Failure::Refused) => REFUSED.to_string(),
        Some(Failure::Tls) => TLS.to_string(),
        Some(Failure::Other(m)) if !m.trim().is_empty() => format!("The server couldn't be reached: {m}"),
        Some(Failure::Other(_)) | None => UNKNOWN.to_string(),
    }
}

/// A proxy in front of the server says the server behind it is down (HTTP 502–504): that counts as no answer.
///
/// From `OfflineMode.kt` `probe` / `unreachable` (`code in 502..504`).
pub fn proxy_down(status: u16) -> bool {
    (502..=504).contains(&status)
}

/// An answer that means the server could not be reached: no answer at all (status 0, which is also what the web
/// app makes of a proxy's own error page, such as Traefik's 404 or Cloudflare's 530), or a proxy saying it is
/// down (502–504).
///
/// From `web-rs/src/offline/mode.rs` `unreachable`.
pub fn unreachable(status: u16) -> bool {
    status == 0 || proxy_down(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shorthand: a check from a device with a network, waiting 8 s.
    fn of(code: Option<u16>, failure: Option<Failure>) -> String {
        reason("Encore", true, code, failure.as_ref(), 8)
    }

    /// Ported from `OfflineReasonTest.noNetworkOnThePhoneWinsOverEverything`.
    #[test]
    fn no_network_on_the_phone_wins_over_everything() {
        assert_eq!(reason("Encore", false, None, Some(&Failure::Dns), 8), NO_NETWORK);
        assert_eq!(reason("Encore", false, Some(502), None, 8), NO_NETWORK);
    }

    /// Ported from `OfflineReasonTest.aProxyAnswering502To504NamesTheProxy`.
    #[test]
    fn a_proxy_answering_502_to_504_names_the_proxy() {
        for code in 502..=504 {
            let r = of(Some(code), None);
            assert!(r.contains(&format!("HTTP {code}")) && r.contains("proxy"), "{r}");
        }
        assert_eq!(
            of(Some(502), Some(Failure::Dns)),
            "The proxy in front of the server answered HTTP 502: it can't reach Encore behind it. If the container was just recreated, restart the proxy or point it at the container by name."
        );
        assert_eq!(of(Some(500), None), "The server answered HTTP 500 to the health check.");
    }

    /// Ported from `OfflineReasonTest.eachFailureHasItsOwnWords`.
    #[test]
    fn each_failure_has_its_own_words() {
        assert!(of(None, Some(Failure::Dns)).contains("DNS"));
        assert_eq!(of(None, Some(Failure::Timeout)), "The server didn't answer within 8 s.");
        assert!(of(None, Some(Failure::Refused)).contains("connection refused"));
        assert!(of(None, Some(Failure::Tls)).contains("HTTPS"));
    }

    /// Ported from `OfflineReasonTest.anythingElseSaysItsMessageOrTheDefault`.
    #[test]
    fn anything_else_says_its_message_or_the_default() {
        assert_eq!(of(None, Some(Failure::Other("reset".into()))), "The server couldn't be reached: reset");
        assert_eq!(of(None, Some(Failure::Other(String::new()))), UNKNOWN);
        assert_eq!(of(None, None), UNKNOWN);
    }

    /// From `web-rs/src/offline/mode.rs` `offline_mode_needs_no_answer_a_remembered_user_and_downloads` (its
    /// status part).
    #[test]
    fn no_answer_or_a_proxy_saying_the_server_is_down_is_unreachable() {
        assert!(unreachable(0));
        assert!(unreachable(502) && unreachable(503) && unreachable(504));
        assert!(!unreachable(401) && !unreachable(500) && !unreachable(505));
    }
}
