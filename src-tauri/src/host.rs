//! Starting programs that belong to the host system (flatpak, kill, xdg-open, ...), not to Vodka.
//!
//! The AppImage's `AppRun` points the loader and GTK at the libraries bundled inside it. Those settings are
//! for Vodka alone: a host program that inherits them loads Vodka's copies instead of its own and can crash
//! before it does anything (on NixOS, `flatpak` dies on a missing `libz.so.1`). Every host program is started
//! through here so it gets the environment it expects. Variables meant for the program, like `MOCKTAIL_*`,
//! are added by the caller as usual.

use std::ffi::OsStr;

/// What the AppImage sets for its own bundled libraries.
pub const APPIMAGE_ENV: [&str; 4] = ["LD_LIBRARY_PATH", "GIO_MODULE_DIR", "GTK_PATH", "GI_TYPELIB_PATH"];

/// A blocking command for a host program.
pub fn command(program: impl AsRef<OsStr>) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    for var in APPIMAGE_ENV {
        cmd.env_remove(var);
    }
    cmd
}

/// An async command for a host program.
pub fn async_command(program: impl AsRef<OsStr>) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    for var in APPIMAGE_ENV {
        cmd.env_remove(var);
    }
    cmd
}

/// The last useful lines a failed program wrote to stderr, short enough to show in the UI.
pub fn stderr_tail(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let tail = lines[lines.len().saturating_sub(3)..].join(" | ");
    if tail.chars().count() > 400 {
        format!("{}…", tail.chars().take(400).collect::<String>())
    } else {
        tail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_appimage_env_but_keeps_what_the_caller_adds() {
        let mut cmd = command("flatpak");
        cmd.env("MOCKTAIL_DATA_ROOT", "/data");
        let envs: Vec<_> = cmd.get_envs().map(|(k, v)| (k.to_string_lossy().into_owned(), v.is_some())).collect();
        for var in APPIMAGE_ENV {
            assert!(envs.contains(&(var.to_owned(), false)), "{var} is not removed");
        }
        assert!(envs.contains(&("MOCKTAIL_DATA_ROOT".to_owned(), true)));
        let async_cmd = async_command("flatpak");
        let removed = async_cmd.as_std().get_envs().filter(|(_, v)| v.is_none()).count();
        assert_eq!(removed, APPIMAGE_ENV.len());
    }

    #[test]
    fn stderr_tail_keeps_the_last_lines() {
        assert_eq!(stderr_tail(b"a\n\nb\nc\nd\n"), "b | c | d");
        assert_eq!(stderr_tail(b""), "");
    }
}
