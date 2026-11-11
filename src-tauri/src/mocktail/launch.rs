//! Starts Mocktail as a child process and reports its output and exit.

use super::detect::APP_ID;
use crate::error::{AppError, Result};
use serde::Serialize;
use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

const LOG_LINES: usize = 400;

#[derive(Default)]
pub struct Session {
    pub log: VecDeque<String>,
    pub active: bool,
}

pub type SharedSession = Arc<Mutex<Session>>;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExitReport {
    pub code: Option<i32>,
    /// "closed", "forwarded" (handed to an already running Mocktail) or "crashed".
    pub outcome: &'static str,
    pub detail: Option<String>,
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

fn pump<R: AsyncRead + Unpin + Send + 'static>(reader: R, app: AppHandle, session: SharedSession) {
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = redact(&line);
            {
                let mut s = session.lock().unwrap();
                if s.log.len() == LOG_LINES {
                    s.log.pop_front();
                }
                s.log.push_back(line.clone());
            }
            let _ = app.emit("mocktail://log", line);
        }
    });
}

pub fn launch(
    app: AppHandle,
    session: SharedSession,
    place_id: u64,
    cookie: Option<String>,
) -> Result<()> {
    {
        let mut s = session.lock().unwrap();
        if s.active {
            return Err(AppError::Launch("a launch is already in progress".into()));
        }
        s.log.clear();
        s.active = true;
    }
    let mut cmd = Command::new("flatpak");
    cmd.args(["run", APP_ID, &launch_uri(place_id)])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cookie) = cookie {
        // Passed through the environment so it never appears in a process list.
        cmd.env("MOCKTAIL_ROBLOX_COOKIES", format!(".ROBLOSECURITY={cookie}"));
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            session.lock().unwrap().active = false;
            return Err(if e.kind() == std::io::ErrorKind::NotFound {
                AppError::FlatpakMissing
            } else {
                AppError::Launch(e.to_string())
            });
        }
    };
    if let Some(out) = child.stdout.take() {
        pump(out, app.clone(), session.clone());
    }
    if let Some(err) = child.stderr.take() {
        pump(err, app.clone(), session.clone());
    }
    tauri::async_runtime::spawn(async move {
        let status = child.wait().await;
        // Let the readers drain the last lines.
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        let report = {
            let mut s = session.lock().unwrap();
            s.active = false;
            let log: Vec<String> = s.log.iter().cloned().collect();
            match status {
                Ok(st) => classify_exit(st.code(), &log),
                Err(e) => ExitReport { code: None, outcome: "crashed", detail: Some(e.to_string()) },
            }
        };
        let _ = app.emit("mocktail://exit", report);
    });
    Ok(())
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
