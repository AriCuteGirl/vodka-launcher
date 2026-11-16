//! The plain logic behind the Windows version's game tracking and FastFlags, kept apart from Windows calls so it
//! builds and is tested everywhere.
//!
//! - Which `RobloxPlayerBeta.exe` process belongs to which game Vodka started, from the process list alone.
//! - Merging Vodka's FastFlags into Roblox's `ClientAppSettings.json` without touching flags someone else put there.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashSet};

/// One running Roblox client, from `tasklist /FO CSV /NH`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub memory_bytes: u64,
}

/// Reads `tasklist /FI "IMAGENAME eq RobloxPlayerBeta.exe" /FO CSV /NH`. The memory column is localised
/// ("512,345 K", "512 345 K", "512.345 K"), so only its digits count.
pub fn parse_tasklist(text: &str) -> Vec<Process> {
    text.lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split("\",\"").map(|c| c.trim_matches(|ch| ch == '"' || ch == '\r')).collect();
            if cols.len() < 5 || !cols[0].eq_ignore_ascii_case("RobloxPlayerBeta.exe") {
                return None;
            }
            let pid = cols[1].parse().ok()?;
            let kib: u64 = cols[4].chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0);
            Some(Process { pid, memory_bytes: kib * 1024 })
        })
        .collect()
}

/// What Vodka knows about one game it started.
#[derive(Debug, Clone, PartialEq)]
pub struct Tracked {
    /// The Roblox process found for it, once one appeared.
    pub pid: Option<u32>,
    /// Roblox processes that were already running when it was launched; never this game's.
    pub before: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Still starting (no process yet) or running in `pid`.
    Running(Option<u32>),
    /// Its process is gone: the window was closed, it crashed, or it was closed from Vodka.
    Ended,
}

/// Matches games to processes. Each game claims a process that appeared after it was launched and that no other
/// game has; earlier launches choose first. A game whose process vanished takes over a new unclaimed one if there
/// is one (Roblox restarting itself), and otherwise has ended.
pub fn match_processes(games: &[Tracked], running: &[Process]) -> Vec<Outcome> {
    let alive: HashSet<u32> = running.iter().map(|p| p.pid).collect();
    let mut claimed: HashSet<u32> = games.iter().filter_map(|g| g.pid).filter(|p| alive.contains(p)).collect();
    let mut pids: Vec<u32> = running.iter().map(|p| p.pid).collect();
    pids.sort_unstable();
    games
        .iter()
        .map(|g| {
            if let Some(pid) = g.pid
                && alive.contains(&pid)
            {
                return Outcome::Running(Some(pid));
            }
            let fresh = pids.iter().copied().find(|p| !claimed.contains(p) && !g.before.contains(p));
            match (fresh, g.pid) {
                (Some(p), _) => {
                    claimed.insert(p);
                    Outcome::Running(Some(p))
                }
                (None, Some(_)) => Outcome::Ended,
                (None, None) => Outcome::Running(None),
            }
        })
        .collect()
}

/// The FastFlags that carry Vodka's simpler options on Windows.
pub fn option_flags(fps: Option<&str>, renderer: Option<&str>) -> BTreeMap<String, String> {
    let mut flags = BTreeMap::new();
    match fps {
        Some("unlimited") => {
            flags.insert("DFIntTaskSchedulerTargetFps".into(), "9999".into());
            flags.insert("FFlagGameBasicSettingsFramerateCap5".into(), "False".into());
        }
        Some(n) if n.parse::<u32>().is_ok() => {
            flags.insert("DFIntTaskSchedulerTargetFps".into(), n.into());
            flags.insert("FFlagGameBasicSettingsFramerateCap5".into(), "False".into());
        }
        _ => {}
    }
    let preferred = match renderer {
        Some("d3d11") => Some("FFlagDebugGraphicsPreferD3D11"),
        Some("vulkan") => Some("FFlagDebugGraphicsPreferVulkan"),
        Some("opengl") => Some("FFlagDebugGraphicsPreferOpenGL"),
        _ => None,
    };
    if let Some(flag) = preferred {
        flags.insert(flag.into(), "True".into());
    }
    flags
}

/// Roblox's flag file wants real JSON types: `True` -> true, numbers as numbers, the rest as strings.
fn flag_value(value: &str) -> Value {
    match value {
        "True" => Value::Bool(true),
        "False" => Value::Bool(false),
        v => v.parse::<i64>().map(Value::from).unwrap_or_else(|_| Value::String(v.to_owned())),
    }
}

/// Puts Vodka's `flags` into an existing `ClientAppSettings.json`, first taking out only the flags Vodka wrote last
/// time (`previous`). Flags put there by the person or another tool stay. Returns the new file and the names Vodka
/// now owns. A file that isn't a JSON object is started over, since Roblox couldn't read it anyway.
pub fn merge_flag_file(existing: &str, previous: &[String], flags: &BTreeMap<String, String>) -> (String, Vec<String>) {
    let mut map: Map<String, Value> = match serde_json::from_str(existing) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    };
    for name in previous {
        map.remove(name);
    }
    for (name, value) in flags {
        map.insert(name.clone(), flag_value(value));
    }
    let text = serde_json::to_string_pretty(&Value::Object(map)).expect("plain map");
    (text, flags.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tasklist_in_any_locale() {
        let out = "\"RobloxPlayerBeta.exe\",\"4120\",\"Console\",\"1\",\"812,344 K\"\r\n\
                   \"RobloxPlayerBeta.exe\",\"7788\",\"Console\",\"1\",\"1 204 112 K\"\r\n\
                   INFO: No tasks are running which match the specified criteria.\r\n";
        assert_eq!(parse_tasklist(out), vec![Process { pid: 4120, memory_bytes: 812_344 * 1024 }, Process { pid: 7788, memory_bytes: 1_204_112 * 1024 }]);
        assert!(parse_tasklist("INFO: No tasks are running which match the specified criteria.").is_empty());
    }

    fn proc(pid: u32) -> Process {
        Process { pid, memory_bytes: 0 }
    }

    #[test]
    fn each_game_gets_the_process_that_appeared_after_it() {
        // Roblox (pid 10) was open before; game A was launched, then B.
        let a = Tracked { pid: None, before: vec![10] };
        let b = Tracked { pid: None, before: vec![10] };
        assert_eq!(match_processes(&[a.clone(), b.clone()], &[proc(10)]), vec![Outcome::Running(None), Outcome::Running(None)]);
        // A's window appears first.
        let now = match_processes(&[a, b.clone()], &[proc(10), proc(30)]);
        assert_eq!(now, vec![Outcome::Running(Some(30)), Outcome::Running(None)]);
        // Then B's.
        let a = Tracked { pid: Some(30), before: vec![10] };
        assert_eq!(match_processes(&[a, b], &[proc(10), proc(30), proc(44)]), vec![Outcome::Running(Some(30)), Outcome::Running(Some(44))]);
    }

    #[test]
    fn a_closed_window_ends_only_its_own_game() {
        let a = Tracked { pid: Some(30), before: vec![] };
        let b = Tracked { pid: Some(44), before: vec![] };
        // The person closes A's window by hand.
        assert_eq!(match_processes(&[a.clone(), b.clone()], &[proc(44)]), vec![Outcome::Ended, Outcome::Running(Some(44))]);
        // Both closed.
        assert_eq!(match_processes(&[a, b], &[]), vec![Outcome::Ended, Outcome::Ended]);
    }

    #[test]
    fn a_game_follows_roblox_restarting_itself() {
        let a = Tracked { pid: Some(30), before: vec![] };
        assert_eq!(match_processes(&[a], &[proc(31)]), vec![Outcome::Running(Some(31))]);
    }

    #[test]
    fn options_become_the_right_flags() {
        let f = option_flags(Some("144"), Some("vulkan"));
        assert_eq!(f.get("DFIntTaskSchedulerTargetFps").map(String::as_str), Some("144"));
        assert_eq!(f.get("FFlagDebugGraphicsPreferVulkan").map(String::as_str), Some("True"));
        assert_eq!(option_flags(Some("unlimited"), None).get("DFIntTaskSchedulerTargetFps").map(String::as_str), Some("9999"));
        assert!(option_flags(Some("display"), Some("auto")).is_empty());
    }

    #[test]
    fn merging_keeps_other_peoples_flags() {
        let existing = r#"{"FFlagTheirs": true, "DFIntOld": 5}"#;
        let flags: BTreeMap<String, String> = [("DFIntTaskSchedulerTargetFps".to_string(), "144".to_string())].into();
        let (text, owned) = merge_flag_file(existing, &["DFIntOld".to_string()], &flags);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["FFlagTheirs"], Value::Bool(true), "someone else's flag stays");
        assert!(v.get("DFIntOld").is_none(), "Vodka's old flag is taken out");
        assert_eq!(v["DFIntTaskSchedulerTargetFps"], Value::from(144));
        assert_eq!(owned, vec!["DFIntTaskSchedulerTargetFps".to_string()]);
        let (text, _) = merge_flag_file("not json", &[], &flags);
        assert!(text.contains("DFIntTaskSchedulerTargetFps"));
    }
}
