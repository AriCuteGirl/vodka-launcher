//! How long you've played each game, the way Steam counts it: a total, the last two weeks and when you last played.
//! Vodka writes the session it's watching every few seconds, so quitting Vodka (or a crash) loses at most that much.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const FILE: &str = "playtime.json";
/// Sessions older than this are folded into the total and forgotten; "last two weeks" only needs 14 days.
const KEEP_SECS: u64 = 30 * 24 * 3600;
const TWO_WEEKS_SECS: u64 = 14 * 24 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    /// "roblox" or "luduvo".
    pub game: String,
    /// Roblox universe id or Luduvo place id.
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub total_secs: u64,
    /// Unix seconds when the last session ended (or was last seen running).
    pub last_played: u64,
    /// Recent sessions as (start, length in seconds).
    #[serde(default)]
    pub sessions: Vec<(u64, u64)>,
}

/// What the UI shows for one game.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Played {
    pub game: String,
    pub id: u64,
    pub title: String,
    pub icon_url: Option<String>,
    pub total_secs: u64,
    pub two_weeks_secs: u64,
    pub last_played: u64,
    pub sessions: u64,
}

#[derive(Default)]
pub struct Store {
    lock: Mutex<()>,
}

fn key(game: &str, id: u64) -> String {
    format!("{game}:{id}")
}

fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

fn read(dir: &Path) -> BTreeMap<String, Record> {
    std::fs::read(path(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn write(dir: &Path, all: &BTreeMap<String, Record>) {
    let _ = std::fs::create_dir_all(dir);
    let tmp = dir.join(format!("{FILE}.tmp"));
    if let Ok(bytes) = serde_json::to_vec(all)
        && std::fs::write(&tmp, bytes).is_ok()
    {
        let _ = std::fs::rename(&tmp, path(dir));
    }
}

/// Records that a session which began at `start` is still going (or ended) at `now`. Calling it again for the same
/// session only adds the time since the last call, so it can be called as often as needed.
pub fn note(all: &mut BTreeMap<String, Record>, game: &str, id: u64, title: &str, icon_url: Option<&str>, start: u64, now: u64) {
    if id == 0 || now <= start {
        return;
    }
    let r = all.entry(key(game, id)).or_insert_with(|| Record { game: game.to_owned(), id, ..Default::default() });
    if !title.is_empty() {
        r.title = title.to_owned();
    }
    if let Some(icon) = icon_url {
        r.icon_url = Some(icon.to_owned());
    }
    let length = now - start;
    match r.sessions.iter_mut().find(|(s, _)| *s == start) {
        Some(session) => {
            r.total_secs += length.saturating_sub(session.1);
            session.1 = session.1.max(length);
        }
        None => {
            r.total_secs += length;
            r.sessions.push((start, length));
        }
    }
    r.last_played = r.last_played.max(now);
    r.sessions.retain(|(s, len)| s + len + KEEP_SECS > now);
}

pub fn summary(all: &BTreeMap<String, Record>, now: u64) -> Vec<Played> {
    let mut list: Vec<Played> = all
        .values()
        .map(|r| Played {
            game: r.game.clone(),
            id: r.id,
            title: r.title.clone(),
            icon_url: r.icon_url.clone(),
            total_secs: r.total_secs,
            two_weeks_secs: r
                .sessions
                .iter()
                .map(|&(s, len)| {
                    // Only the part of a session inside the last two weeks.
                    let from = now.saturating_sub(TWO_WEEKS_SECS);
                    (s + len).saturating_sub(s.max(from)).min(len)
                })
                .sum(),
            last_played: r.last_played,
            sessions: r.sessions.len() as u64,
        })
        .collect();
    list.sort_by_key(|p| std::cmp::Reverse(p.last_played));
    list
}

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn note(&self, dir: &Path, game: &str, id: u64, title: &str, icon_url: Option<&str>, start: u64, now: u64) {
        let _g = self.lock.lock().unwrap();
        let mut all = read(dir);
        note(&mut all, game, id, title, icon_url, start, now);
        write(dir, &all);
    }

    pub fn list(&self, dir: &Path, now: u64) -> Vec<Played> {
        let _g = self.lock.lock().unwrap();
        summary(&read(dir), now)
    }
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_noted_again_only_adds_the_new_time() {
        let mut all = BTreeMap::new();
        note(&mut all, "roblox", 5, "RIVALS", None, 1000, 1060);
        note(&mut all, "roblox", 5, "RIVALS", None, 1000, 1120);
        note(&mut all, "roblox", 5, "RIVALS", Some("https://tr.rbxcdn.com/x"), 1000, 1120);
        let r = &all["roblox:5"];
        assert_eq!((r.total_secs, r.sessions.len(), r.last_played), (120, 1, 1120));
        assert_eq!(r.icon_url.as_deref(), Some("https://tr.rbxcdn.com/x"));
        note(&mut all, "roblox", 5, "RIVALS", None, 2000, 2030);
        assert_eq!((all["roblox:5"].total_secs, all["roblox:5"].sessions.len()), (150, 2));
    }

    #[test]
    fn two_weeks_counts_only_recent_time_and_old_sessions_fold_into_the_total() {
        let day = 24 * 3600;
        let now = 100 * day;
        let mut all = BTreeMap::new();
        note(&mut all, "luduvo", 9, "Fish.io", None, now - 20 * day, now - 20 * day + 600);
        note(&mut all, "luduvo", 9, "Fish.io", None, now - day, now - day + 300);
        let s = summary(&all, now);
        assert_eq!((s[0].total_secs, s[0].two_weeks_secs), (900, 300));
        // Far later, the old sessions are gone but the total stays.
        note(&mut all, "luduvo", 9, "Fish.io", None, now + 60 * day, now + 60 * day + 60);
        assert_eq!((all["luduvo:9"].total_secs, all["luduvo:9"].sessions.len()), (960, 1));
    }

    #[test]
    fn ignores_nonsense() {
        let mut all = BTreeMap::new();
        note(&mut all, "roblox", 0, "x", None, 10, 20);
        note(&mut all, "roblox", 1, "x", None, 20, 10);
        assert!(all.is_empty());
    }
}
