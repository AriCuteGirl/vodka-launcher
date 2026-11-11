//! Finds an installed Mocktail. Phase 1 knows the Flatpak only.

use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

pub const APP_ID: &str = "space.bigrat.mocktail";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallStep {
    pub label: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MocktailStatus {
    pub flatpak_available: bool,
    pub installed: bool,
    pub version: Option<String>,
    pub roblox_version: Option<String>,
    pub running: bool,
    pub distro: String,
    pub install_steps: Vec<InstallStep>,
}

/// Parses `flatpak list --app --columns=application,version`.
pub fn parse_flatpak_list(output: &str) -> Option<Option<String>> {
    output.lines().find_map(|line| {
        let mut cols = line.split('\t').map(str::trim);
        if cols.next()? != APP_ID {
            return None;
        }
        Some(cols.next().filter(|v| !v.is_empty()).map(str::to_owned))
    })
}

/// Parses `flatpak ps --columns=application`.
pub fn parse_flatpak_ps(output: &str) -> bool {
    output.lines().any(|line| line.trim() == APP_ID)
}

/// Reads `version_name` from Mocktail's `current.json`.
pub fn parse_roblox_version(current_json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(current_json).ok()?;
    let root = v.get("current").unwrap_or(&v);
    root.get("version_name")?.as_str().map(str::to_owned)
}

/// Returns (pretty name, package-manager family) from `/etc/os-release`.
pub fn parse_os_release(text: &str) -> (String, &'static str) {
    let field = |key: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('='))
            .map(|v| v.trim_matches('"').to_owned())
    };
    let name = field("PRETTY_NAME")
        .or_else(|| field("NAME"))
        .unwrap_or_else(|| "Linux".into());
    let ids = format!(
        " {} {} ",
        field("ID").unwrap_or_default(),
        field("ID_LIKE").unwrap_or_default()
    )
    .to_lowercase();
    let has = |id: &str| ids.contains(&format!(" {id} "));
    let family = if has("arch") {
        "pacman"
    } else if has("debian") || has("ubuntu") {
        "apt"
    } else if has("fedora") || has("rhel") {
        "dnf"
    } else if has("opensuse") || has("suse") {
        "zypper"
    } else {
        "unknown"
    };
    (name, family)
}

pub fn install_steps(family: &str, flatpak_available: bool) -> Vec<InstallStep> {
    let mut steps = Vec::new();
    if !flatpak_available {
        let command = match family {
            "pacman" => "sudo pacman -S --needed flatpak",
            "apt" => "sudo apt install flatpak",
            "dnf" => "sudo dnf install flatpak",
            "zypper" => "sudo zypper install flatpak",
            _ => "Install Flatpak with your distro's package manager",
        };
        steps.push(InstallStep { label: "Install Flatpak".into(), command: command.into() });
        steps.push(InstallStep {
            label: "Add the Flathub store".into(),
            command: "flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo".into(),
        });
    }
    steps.push(InstallStep {
        label: "Install Mocktail".into(),
        command: format!("flatpak install --user flathub {APP_ID}"),
    });
    steps
}

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new("flatpak").args(args).env("LC_ALL", "C").output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn flatpak_data_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".var/app").join(APP_ID).join("data/mocktail"))
}

pub fn detect() -> MocktailStatus {
    let os = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let (distro, family) = parse_os_release(&os);
    let flatpak_available = run(&["--version"]).is_some();
    let listed = run(&["list", "--app", "--columns=application,version"])
        .and_then(|o| parse_flatpak_list(&o));
    let installed = listed.is_some();
    let running = installed
        && run(&["ps", "--columns=application"]).is_some_and(|o| parse_flatpak_ps(&o));
    let roblox_version = flatpak_data_dir()
        .and_then(|d| std::fs::read_to_string(d.join("current.json")).ok())
        .and_then(|t| parse_roblox_version(&t));
    MocktailStatus {
        flatpak_available,
        installed,
        version: listed.flatten(),
        roblox_version,
        running,
        distro,
        install_steps: if installed { vec![] } else { install_steps(family, flatpak_available) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_mocktail_in_flatpak_list() {
        let out = "org.other.App\t1.0\nspace.bigrat.mocktail\t1.0.5\n";
        assert_eq!(parse_flatpak_list(out), Some(Some("1.0.5".into())));
    }

    #[test]
    fn installed_without_version_still_counts() {
        assert_eq!(parse_flatpak_list("space.bigrat.mocktail\t\n"), Some(None));
    }

    #[test]
    fn missing_mocktail_is_none() {
        assert_eq!(parse_flatpak_list("org.other.App\t1.0\n"), None);
        // A lookalike id must not match.
        assert_eq!(parse_flatpak_list("space.bigrat.mocktail.Debug\t1\n"), None);
    }

    #[test]
    fn detects_running_instance() {
        assert!(parse_flatpak_ps("space.bigrat.mocktail\n"));
        assert!(!parse_flatpak_ps("org.other.App\n"));
        assert!(!parse_flatpak_ps(""));
    }

    #[test]
    fn reads_roblox_version_from_both_layouts() {
        assert_eq!(parse_roblox_version(r#"{"version_name":"2.742.1059"}"#).as_deref(), Some("2.742.1059"));
        assert_eq!(
            parse_roblox_version(r#"{"current":{"version_name":"2.1.0"}}"#).as_deref(),
            Some("2.1.0")
        );
        assert_eq!(parse_roblox_version("not json"), None);
    }

    #[test]
    fn maps_distros_to_package_managers() {
        assert_eq!(parse_os_release("ID=arch\nPRETTY_NAME=\"Arch Linux\"\n"), ("Arch Linux".into(), "pacman"));
        assert_eq!(parse_os_release("ID=cachyos\nID_LIKE=arch\n").1, "pacman");
        assert_eq!(parse_os_release("ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n").1, "apt");
        assert_eq!(parse_os_release("ID=fedora\n").1, "dnf");
        assert_eq!(parse_os_release("ID=nixos\n").1, "unknown");
    }

    #[test]
    fn install_guide_includes_flatpak_only_when_missing() {
        assert_eq!(install_steps("pacman", true).len(), 1);
        let steps = install_steps("apt", false);
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0].command, "sudo apt install flatpak");
    }
}
