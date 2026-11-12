//! Launch options and FFlag overrides: what the user may set, how it is checked,
//! and how it is handed to Mocktail (as environment variables, per launch).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const BACKENDS: [&str; 4] = ["direct-vulkan", "opengl", "system", "angle-vulkan"];
pub const VSYNC: [&str; 3] = ["auto", "on", "off"];
pub const PRESENT_MODES: [&str; 4] = ["auto", "fifo", "mailbox", "immediate"];
pub const GAMEMODE: [&str; 3] = ["auto", "on", "off"];
const MAX_FLAGS: usize = 200;
/// Roblox sits around 1.5 GB and spikes well above 2 GB when a lot loads at once (a 2 GB
/// limit closed a real session mid-trade). Mocktail closes the game when the limit is
/// exceeded, so the lowest limit Vodka accepts leaves real headroom.
pub const MIN_MEMORY_CAP_MB: u32 = 4096;
/// Flags Vodka sets itself through the FPS option; editing them by hand would fight it.
const MANAGED_FLAGS: [&str; 2] = ["DFIntTaskSchedulerTargetFps", "FFlagGameBasicSettingsFramerateCap5"];

/// Every field is optional: `None` means "don't override" (use the level below:
/// account -> Vodka defaults -> Mocktail's own config).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchOptions {
    /// "display", "unlimited" or a whole number of frames per second.
    pub fps: Option<String>,
    pub backend: Option<String>,
    pub vsync: Option<String>,
    pub present_mode: Option<String>,
    pub multithreaded_rendering: Option<bool>,
    pub gamemode: Option<String>,
    /// Megabytes. 0 turns the cap off.
    pub memory_limit_mb: Option<u32>,
    /// Keep the current Roblox client instead of updating on launch.
    pub hold_updates: Option<bool>,
    pub fflags: BTreeMap<String, String>,
}

fn one_of(what: &str, value: &Option<String>, allowed: &[&str]) -> Result<(), String> {
    match value {
        Some(v) if !allowed.contains(&v.as_str()) => Err(format!("{what} must be one of: {}.", allowed.join(", "))),
        _ => Ok(()),
    }
}

pub fn valid_fps(value: &str) -> bool {
    value == "display" || value == "unlimited" || value.parse::<u32>().is_ok_and(|n| (1..=1000).contains(&n)) && !value.starts_with('+')
}

/// The value type a flag's name prefix implies.
fn flag_kind(name: &str) -> Option<&'static str> {
    let rest = ["DF", "SF", "F"].iter().find_map(|p| name.strip_prefix(p))?;
    let kind = ["Flag", "Int", "String", "Log"].into_iter().find(|k| rest.starts_with(k))?;
    let tail = &rest[kind.len()..];
    (!tail.is_empty() && tail.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')).then_some(kind)
}

/// Checks one flag and returns it in the form Roblox expects (for example `true` -> `True`).
pub fn normalize_flag(name: &str, value: &str) -> Result<(String, String), String> {
    let name = name.trim();
    let value = value.trim();
    if name.len() > 120 {
        return Err("That flag name is too long.".into());
    }
    let Some(kind) = flag_kind(name) else {
        return Err("Flag names start with FFlag, FInt, FString or FLog (optionally DF… or SF…), followed by letters and digits.".into());
    };
    if MANAGED_FLAGS.contains(&name) {
        return Err("Vodka sets this flag from the FPS option. Change FPS there instead.".into());
    }
    let value = match kind {
        "Flag" => match value.to_ascii_lowercase().as_str() {
            "true" => "True".to_owned(),
            "false" => "False".to_owned(),
            _ => return Err(format!("{name} is an on/off flag: use True or False.")),
        },
        "Int" | "Log" => match value.parse::<i64>() {
            Ok(n) => n.to_string(),
            Err(_) => return Err(format!("{name} needs a whole number.")),
        },
        _ => {
            if value.len() > 1024 || value.chars().any(char::is_control) {
                return Err(format!("{name} needs plain text, up to 1024 characters."));
            }
            value.to_owned()
        }
    };
    Ok((name.to_owned(), value))
}

/// Checks a whole set of options and returns it with flags normalized.
pub fn validate(options: &LaunchOptions) -> Result<LaunchOptions, String> {
    if let Some(fps) = &options.fps {
        if !valid_fps(fps) {
            return Err("FPS must be display, unlimited, or a number from 1 to 1000.".into());
        }
    }
    one_of("Graphics backend", &options.backend, &BACKENDS)?;
    one_of("VSync", &options.vsync, &VSYNC)?;
    one_of("Present mode", &options.present_mode, &PRESENT_MODES)?;
    one_of("GameMode", &options.gamemode, &GAMEMODE)?;
    if let Some(mb) = options.memory_limit_mb {
        if mb != 0 && !(MIN_MEMORY_CAP_MB..=262_144).contains(&mb) {
            return Err(format!(
                "The memory limit must be off or at least {MIN_MEMORY_CAP_MB} MB. Roblox can spike past 2 GB, and Mocktail closes the game when the limit is exceeded."
            ));
        }
    }
    if options.fflags.len() > MAX_FLAGS {
        return Err(format!("That's more than {MAX_FLAGS} flags."));
    }
    let mut out = options.clone();
    out.fflags = BTreeMap::new();
    for (name, value) in &options.fflags {
        let (name, value) = normalize_flag(name, value)?;
        if out.fflags.insert(name.clone(), value).is_some() {
            return Err(format!("{name} is listed twice."));
        }
    }
    Ok(out)
}

/// Account settings on top of Vodka's defaults. Flags from both are combined; the account wins.
pub fn effective(defaults: &LaunchOptions, account: Option<&LaunchOptions>) -> LaunchOptions {
    let Some(a) = account else { return defaults.clone() };
    let mut fflags = defaults.fflags.clone();
    fflags.extend(a.fflags.clone());
    LaunchOptions {
        fps: a.fps.clone().or_else(|| defaults.fps.clone()),
        backend: a.backend.clone().or_else(|| defaults.backend.clone()),
        vsync: a.vsync.clone().or_else(|| defaults.vsync.clone()),
        present_mode: a.present_mode.clone().or_else(|| defaults.present_mode.clone()),
        multithreaded_rendering: a.multithreaded_rendering.or(defaults.multithreaded_rendering),
        gamemode: a.gamemode.clone().or_else(|| defaults.gamemode.clone()),
        memory_limit_mb: a.memory_limit_mb.or(defaults.memory_limit_mb),
        hold_updates: a.hold_updates.or(defaults.hold_updates),
        fflags,
    }
}

/// Brings options saved by an older Vodka up to today's rules instead of refusing to launch:
/// a memory limit that is now considered too low is raised to the minimum.
pub fn modernize(mut options: LaunchOptions) -> LaunchOptions {
    if let Some(mb) = options.memory_limit_mb {
        if mb != 0 && mb < MIN_MEMORY_CAP_MB {
            options.memory_limit_mb = Some(MIN_MEMORY_CAP_MB);
        }
    }
    options
}

/// The environment variables Mocktail reads for these options. Unset options add nothing,
/// which leaves Mocktail's own config.yaml and fflags.json in charge.
pub fn to_env(options: &LaunchOptions) -> Vec<(&'static str, String)> {
    let mut env = Vec::new();
    let mut put = |name, value: Option<String>| {
        if let Some(v) = value {
            env.push((name, v));
        }
    };
    put("MOCKTAIL_FRAME_RATE_LIMIT", options.fps.clone());
    put("MOCKTAIL_GRAPHICS_BACKEND", options.backend.clone());
    put("MOCKTAIL_VSYNC", options.vsync.clone());
    put("MOCKTAIL_PRESENT_MODE", options.present_mode.clone());
    put("MOCKTAIL_MULTITHREADED_RENDERING", options.multithreaded_rendering.map(|on| if on { "1" } else { "0" }.to_owned()));
    put("MOCKTAIL_GAMEMODE", options.gamemode.clone());
    put("MOCKTAIL_MEMORY_LIMIT_MB", options.memory_limit_mb.map(|mb| mb.to_string()));
    put("MOCKTAIL_SKIP_UPDATE_CHECK", options.hold_updates.filter(|hold| *hold).map(|_| "1".to_owned()));
    if !options.fflags.is_empty() {
        let json = serde_json::to_string(&options.fflags).expect("string map");
        put("MOCKTAIL_CLIENT_SETTINGS_OVERRIDES_JSON", Some(json));
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn fps_accepts_keywords_and_sane_numbers_only() {
        for ok in ["display", "unlimited", "1", "30", "240", "1000"] {
            assert!(valid_fps(ok), "{ok}");
        }
        for bad in ["", "0", "1001", "-5", "60fps", "6.5", "+60", "Unlimited"] {
            assert!(!valid_fps(bad), "{bad}");
        }
    }

    #[test]
    fn flag_names_must_look_like_roblox_flags() {
        assert!(normalize_flag("FFlagDisablePostFx", "True").is_ok());
        assert!(normalize_flag("DFIntDebugFRMQualityLevelOverride", "1").is_ok());
        assert!(normalize_flag("SFFlagSomething", "false").is_ok());
        for bad in ["", "Flag", "FFlag", "fflagLower", "FFlagWith Space", "FFlagWith\"Quote", "XFlagNope", "FBoolNope"] {
            assert!(normalize_flag(bad, "True").is_err(), "{bad}");
        }
    }

    #[test]
    fn flag_values_are_checked_against_the_type_in_the_name() {
        assert_eq!(normalize_flag(" FFlagX ", " true ").unwrap(), ("FFlagX".into(), "True".into()));
        assert_eq!(normalize_flag("DFFlagX", "FALSE").unwrap().1, "False");
        assert!(normalize_flag("FFlagX", "1").is_err());
        assert_eq!(normalize_flag("FIntX", "0042").unwrap().1, "42");
        assert_eq!(normalize_flag("DFIntX", "-3").unwrap().1, "-3");
        assert!(normalize_flag("FIntX", "fast").is_err());
        assert!(normalize_flag("FIntX", "1.5").is_err());
        assert_eq!(normalize_flag("FStringX", "a,b;c").unwrap().1, "a,b;c");
        assert!(normalize_flag("FStringX", "line\nbreak").is_err());
        assert!(normalize_flag("FLogX", "7").is_ok());
    }

    #[test]
    fn fps_flags_are_reserved_for_the_fps_option() {
        let err = normalize_flag("DFIntTaskSchedulerTargetFps", "999").unwrap_err();
        assert!(err.contains("FPS option"));
    }

    #[test]
    fn validate_rejects_values_mocktail_would_not_accept() {
        let base = LaunchOptions::default();
        assert!(validate(&base).is_ok());
        assert!(validate(&LaunchOptions { fps: Some("fast".into()), ..base.clone() }).is_err());
        assert!(validate(&LaunchOptions { backend: Some("directx".into()), ..base.clone() }).is_err());
        assert!(validate(&LaunchOptions { vsync: Some("maybe".into()), ..base.clone() }).is_err());
        assert!(validate(&LaunchOptions { present_mode: Some("mailbox".into()), ..base.clone() }).is_ok());
        assert!(validate(&LaunchOptions { memory_limit_mb: Some(600), ..base.clone() }).unwrap_err().contains("closes the game"));
        assert!(validate(&LaunchOptions { memory_limit_mb: Some(4096), ..base.clone() }).is_ok());
        assert!(validate(&LaunchOptions { memory_limit_mb: Some(2048), ..base.clone() }).is_err());
        assert!(validate(&LaunchOptions { memory_limit_mb: Some(0), ..base.clone() }).is_ok());
        assert!(validate(&LaunchOptions { fflags: flags(&[("FFlagX", "maybe")]), ..base.clone() }).is_err());
        // Two spellings of one flag collapse into a duplicate.
        assert!(validate(&LaunchOptions { fflags: flags(&[("FFlagX", "True"), (" FFlagX", "False")]), ..base }).is_err());
    }

    #[test]
    fn validate_normalizes_flag_values() {
        let out = validate(&LaunchOptions { fflags: flags(&[("FFlagX", "true")]), ..Default::default() }).unwrap();
        assert_eq!(out.fflags["FFlagX"], "True");
    }

    #[test]
    fn account_options_override_defaults_field_by_field() {
        let defaults = LaunchOptions {
            fps: Some("60".into()),
            vsync: Some("on".into()),
            fflags: flags(&[("FFlagA", "True"), ("FIntB", "1")]),
            ..Default::default()
        };
        let account = LaunchOptions { fps: Some("unlimited".into()), fflags: flags(&[("FIntB", "2")]), ..Default::default() };
        let e = effective(&defaults, Some(&account));
        assert_eq!(e.fps.as_deref(), Some("unlimited"));
        assert_eq!(e.vsync.as_deref(), Some("on"));
        assert_eq!(e.fflags, flags(&[("FFlagA", "True"), ("FIntB", "2")]));
        assert_eq!(effective(&defaults, None), defaults);
    }

    #[test]
    fn an_old_too_low_memory_limit_is_raised_not_rejected() {
        let old = LaunchOptions { memory_limit_mb: Some(2048), ..Default::default() };
        assert_eq!(modernize(old).memory_limit_mb, Some(MIN_MEMORY_CAP_MB));
        let off = LaunchOptions { memory_limit_mb: Some(0), ..Default::default() };
        assert_eq!(modernize(off).memory_limit_mb, Some(0));
        let high = LaunchOptions { memory_limit_mb: Some(8192), ..Default::default() };
        assert_eq!(modernize(high).memory_limit_mb, Some(8192));
        assert!(validate(&modernize(LaunchOptions { memory_limit_mb: Some(2048), ..Default::default() })).is_ok());
    }

    #[test]
    fn unset_options_add_no_environment() {
        assert!(to_env(&LaunchOptions::default()).is_empty());
        assert!(to_env(&LaunchOptions { hold_updates: Some(false), ..Default::default() }).is_empty());
    }

    #[test]
    fn options_map_to_the_variables_mocktail_reads() {
        let env = to_env(&LaunchOptions {
            fps: Some("unlimited".into()),
            backend: Some("opengl".into()),
            multithreaded_rendering: Some(true),
            memory_limit_mb: Some(6144),
            hold_updates: Some(true),
            fflags: flags(&[("FFlagX", "True")]),
            ..Default::default()
        });
        let get = |k: &str| env.iter().find(|(n, _)| *n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("MOCKTAIL_FRAME_RATE_LIMIT"), Some("unlimited"));
        assert_eq!(get("MOCKTAIL_GRAPHICS_BACKEND"), Some("opengl"));
        assert_eq!(get("MOCKTAIL_MULTITHREADED_RENDERING"), Some("1"));
        assert_eq!(get("MOCKTAIL_MEMORY_LIMIT_MB"), Some("6144"));
        assert_eq!(get("MOCKTAIL_SKIP_UPDATE_CHECK"), Some("1"));
        assert_eq!(get("MOCKTAIL_CLIENT_SETTINGS_OVERRIDES_JSON"), Some(r#"{"FFlagX":"True"}"#));
        assert_eq!(get("MOCKTAIL_VSYNC"), None);
    }
}
