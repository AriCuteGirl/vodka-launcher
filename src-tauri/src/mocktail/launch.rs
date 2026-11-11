//! What Vodka knows about talking to a Mocktail process: the link it accepts,
//! how to read its output safely, and how to judge its exit.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExitReport {
    pub code: Option<i32>,
    /// "closed", "forwarded" (handed to an already running Mocktail) or "crashed".
    pub outcome: &'static str,
    pub detail: Option<String>,
}

/// Mocktail prints this once the game is drawing frames.
pub fn is_running_marker(line: &str) -> bool {
    line.contains("[perf] present_sample")
}

pub fn launch_uri(place_id: u64) -> String {
    format!("roblox://experiences/start?placeId={place_id}")
}

/// Keeps session cookies out of the log, whatever Mocktail prints.
pub fn redact(line: &str) -> String {
    const MARKERS: [&str; 2] = [".ROBLOSECURITY", "_|WARNING:-DO-NOT-SHARE-THIS"];
    if MARKERS.iter().any(|m| line.contains(m)) {
        "[line hidden: contained a session cookie]".into()
    } else {
        line.to_owned()
    }
}

pub fn classify_exit(code: Option<i32>, log: &[String]) -> ExitReport {
    if log.iter().any(|l| l.contains("sent website join to running Mocktail")) {
        return ExitReport { code, outcome: "forwarded", detail: None };
    }
    if code == Some(0) {
        return ExitReport { code, outcome: "closed", detail: None };
    }
    let detail = log
        .iter()
        .rev()
        .find(|l| l.contains("[FATAL]"))
        .map(|l| l.replace("[FATAL]", "").trim().to_owned())
        .or_else(|| match code {
            Some(c) => Some(format!("Mocktail exited with code {c}.")),
            None => Some("Mocktail was stopped by a signal.".into()),
        });
    ExitReport { code, outcome: "crashed", detail }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_uri_mocktail_parses() {
        assert_eq!(launch_uri(17625359962), "roblox://experiences/start?placeId=17625359962");
    }

    #[test]
    fn cookies_never_reach_the_log() {
        let line = "auth .ROBLOSECURITY=_|WARNING:-DO-NOT-SHARE-THIS.--abc";
        assert!(!redact(line).contains("abc"));
        assert_eq!(redact("  [runtime] ok"), "  [runtime] ok");
    }

    #[test]
    fn spots_the_first_rendered_frame() {
        assert!(is_running_marker("  [perf] present_sample frame=120"));
        assert!(!is_running_marker("  [window] Window ready; waiting for Roblox frames"));
    }

    #[test]
    fn clean_exit_is_closed() {
        assert_eq!(classify_exit(Some(0), &[]).outcome, "closed");
    }

    #[test]
    fn forwarded_join_is_not_a_crash() {
        let log = vec!["  [launch] sent website join to running Mocktail".to_string()];
        assert_eq!(classify_exit(Some(0), &log).outcome, "forwarded");
    }

    #[test]
    fn crash_reports_the_fatal_line() {
        let log = vec!["x".to_string(), "[FATAL] Mocktail is already running for this user".to_string()];
        let r = classify_exit(Some(1), &log);
        assert_eq!(r.outcome, "crashed");
        assert_eq!(r.detail.as_deref(), Some("Mocktail is already running for this user"));
        assert_eq!(classify_exit(None, &[]).detail.as_deref(), Some("Mocktail was stopped by a signal."));
    }
}
