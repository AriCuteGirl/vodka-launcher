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

/// Reads Mocktail's periodic frame counter line: (frames presented so far, monotonic nanoseconds).
pub fn parse_present_sample(line: &str) -> Option<(u64, u64)> {
    let rest = line.split("present_sample frame=").nth(1)?;
    let (frame, rest) = rest.split_once(' ')?;
    let ns = rest.strip_prefix("monotonic_ns=")?.split_whitespace().next()?;
    Some((frame.parse().ok()?, ns.parse().ok()?))
}

/// Frames per second between two samples.
pub fn fps_between(before: (u64, u64), now: (u64, u64)) -> Option<f64> {
    let frames = now.0.checked_sub(before.0)?;
    let ns = now.1.checked_sub(before.1).filter(|ns| *ns > 0)?;
    Some(frames as f64 / (ns as f64 / 1e9))
}

pub fn launch_uri(place_id: u64) -> String {
    format!("roblox://experiences/start?placeId={place_id}")
}

/// A link into a private server. `access_code` must already be validated.
pub fn private_uri(place_id: u64, access_code: &str) -> String {
    format!("roblox://experiences/start?placeId={place_id}&accessCode={access_code}")
}

/// A link into one specific server, for joining a friend. `game_id` must already be validated.
pub fn join_uri(place_id: u64, game_id: &str) -> String {
    format!("roblox://experiences/start?placeId={place_id}&gameInstanceId={game_id}")
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
    // Mocktail's memory safety limit closing the game is a crash to the player, whatever the exit code.
    if log.iter().any(|l| l.contains("[memory] limit exceeded")) {
        return ExitReport {
            code,
            outcome: "crashed",
            detail: Some(
                "The game went over its memory safety limit, so Mocktail closed it. Raise the limit or turn it off in Settings → Performance."
                    .into(),
            ),
        };
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
    fn builds_a_server_link_for_joining_a_friend() {
        assert_eq!(join_uri(5, "0f0e-11"), "roblox://experiences/start?placeId=5&gameInstanceId=0f0e-11");
    }

    #[test]
    fn builds_a_private_server_link() {
        assert_eq!(private_uri(5, "3f2a-9c"), "roblox://experiences/start?placeId=5&accessCode=3f2a-9c");
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
    fn measures_fps_from_two_frame_samples() {
        // Real lines from a Mocktail log: 240 frames in 4.0002 s.
        let a = parse_present_sample("  [perf] present_sample frame=19680 monotonic_ns=337838012812").unwrap();
        let b = parse_present_sample("  [perf] present_sample frame=19920 monotonic_ns=341838189558").unwrap();
        assert_eq!(a, (19680, 337838012812));
        let fps = fps_between(a, b).unwrap();
        assert!((fps - 60.0).abs() < 0.1, "{fps}");
        assert_eq!(fps_between(b, a), None);
        assert_eq!(fps_between(a, a), None);
        assert_eq!(parse_present_sample("  [perf] something else"), None);
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
    fn hitting_the_memory_limit_is_explained() {
        let log = vec!["  [memory] limit exceeded: RSS+swap=2101 MiB, limit=2048 MiB; terminating".to_string()];
        for code in [Some(0), Some(137), None] {
            let r = classify_exit(code, &log);
            assert_eq!(r.outcome, "crashed");
            assert!(r.detail.unwrap().contains("memory safety limit"));
        }
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
