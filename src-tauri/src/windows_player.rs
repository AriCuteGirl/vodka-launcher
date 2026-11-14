//! Building the link that starts the official Roblox client on Windows as a given account.
//! Pure string work, kept free of Windows APIs so it is tested on every platform.

/// What Vodka's own launch link (the one Mocktail takes) asks for.
#[derive(Debug, PartialEq)]
pub struct Destination {
    pub place_id: u64,
    pub server: Option<String>,
    pub access_code: Option<String>,
}

fn safe_id(value: &str) -> bool {
    (1..=128).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Reads `roblox://experiences/start?placeId=…[&gameInstanceId=…][&accessCode=…]`.
pub fn destination(launch_uri: &str) -> Option<Destination> {
    let query = launch_uri.strip_prefix("roblox://experiences/start?")?;
    let mut out = Destination { place_id: 0, server: None, access_code: None };
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        match key {
            "placeId" => out.place_id = value.parse().ok()?,
            "gameInstanceId" if safe_id(value) => out.server = Some(value.to_owned()),
            "accessCode" if safe_id(value) => out.access_code = Some(value.to_owned()),
            _ => return None,
        }
    }
    (out.place_id > 0).then_some(out)
}

fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

/// The `roblox-player:` link for the official client: who (a one-time sign-in ticket) and where.
/// `tracker` is any number; Roblox only uses it to tell launches apart.
pub fn player_link(ticket: &str, to: &Destination, now_ms: u64, tracker: u64) -> Option<String> {
    // The ticket goes into a link verbatim, so it must not be able to add fields of its own.
    if ticket.is_empty() || !ticket.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return None;
    }
    let base = "https://assetgame.roblox.com/game/PlaceLauncher.ashx";
    let place = to.place_id;
    let request = match (&to.server, &to.access_code) {
        (Some(server), _) => format!("{base}?request=RequestGameJob&browserTrackerId={tracker}&placeId={place}&gameId={server}&isPlayTogetherGame=false"),
        (None, Some(code)) => format!("{base}?request=RequestPrivateGame&browserTrackerId={tracker}&placeId={place}&accessCode={code}"),
        (None, None) => format!("{base}?request=RequestGame&browserTrackerId={tracker}&placeId={place}&isPlayTogetherGame=false"),
    };
    Some(format!(
        "roblox-player:1+launchmode:play+gameinfo:{ticket}+launchtime:{now_ms}+placelauncherurl:{}+browsertrackerid:{tracker}+robloxLocale:en_us+gameLocale:en_us+channel:",
        percent_encode(&request)
    ))
}

/// Picks the newest installed Roblox client from the names in `%LOCALAPPDATA%\Roblox\Versions`
/// (each paired with whether it contains `RobloxPlayerBeta.exe` and its modified time).
pub fn newest_player(folders: &[(String, bool, u64)]) -> Option<&str> {
    folders.iter().filter(|(_, has_player, _)| *has_player).max_by_key(|(_, _, modified)| *modified).map(|(name, _, _)| name.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_three_kinds_of_launch_link() {
        assert_eq!(destination("roblox://experiences/start?placeId=5"), Some(Destination { place_id: 5, server: None, access_code: None }));
        let server = destination("roblox://experiences/start?placeId=5&gameInstanceId=0f0e-11").unwrap();
        assert_eq!(server.server.as_deref(), Some("0f0e-11"));
        let private = destination("roblox://experiences/start?placeId=5&accessCode=zz_9").unwrap();
        assert_eq!(private.access_code.as_deref(), Some("zz_9"));
    }

    #[test]
    fn refuses_links_it_does_not_fully_understand() {
        for bad in [
            "roblox://experiences/start?placeId=0",
            "roblox://experiences/start?placeId=abc",
            "roblox://experiences/start?placeId=5&gameInstanceId=a+b",
            "roblox://experiences/start?placeId=5&launchData=x",
            "https://example.com/?placeId=5",
        ] {
            assert_eq!(destination(bad), None, "{bad}");
        }
    }

    #[test]
    fn builds_the_official_client_link() {
        let to = Destination { place_id: 5, server: None, access_code: None };
        let link = player_link("TICKET123", &to, 1000, 42).unwrap();
        assert!(link.starts_with("roblox-player:1+launchmode:play+gameinfo:TICKET123+launchtime:1000+placelauncherurl:https%3A%2F%2Fassetgame.roblox.com%2Fgame%2FPlaceLauncher.ashx%3Frequest%3DRequestGame%26browserTrackerId%3D42%26placeId%3D5"));
        assert!(link.ends_with("+browsertrackerid:42+robloxLocale:en_us+gameLocale:en_us+channel:"));
        let server = Destination { place_id: 5, server: Some("0f0e-11".into()), access_code: None };
        assert!(player_link("T", &server, 1, 2).unwrap().contains("request%3DRequestGameJob") && player_link("T", &server, 1, 2).unwrap().contains("gameId%3D0f0e-11"));
        let private = Destination { place_id: 5, server: None, access_code: Some("zz".into()) };
        assert!(player_link("T", &private, 1, 2).unwrap().contains("request%3DRequestPrivateGame%26browserTrackerId%3D2%26placeId%3D5%26accessCode%3Dzz"));
    }

    #[test]
    fn a_ticket_cannot_smuggle_extra_fields() {
        let to = Destination { place_id: 5, server: None, access_code: None };
        assert_eq!(player_link("abc+launchmode:app", &to, 1, 2), None);
        assert_eq!(player_link("", &to, 1, 2), None);
    }

    #[test]
    fn picks_the_newest_folder_that_has_the_player() {
        let folders = vec![("version-old".to_string(), true, 10), ("version-studio".to_string(), false, 99), ("version-new".to_string(), true, 50)];
        assert_eq!(newest_player(&folders), Some("version-new"));
        assert_eq!(newest_player(&[("version-studio".to_string(), false, 1)]), None);
    }
}
