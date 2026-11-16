//! Turns api.luduvo.com answers into the few fields Vodka shows. Lenient: a missing field becomes a default
//! rather than an error, since Luduvo is young and its API changes often.

use reqwest::Url;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub id: u64,
    pub title: String,
    pub creator: String,
    pub description: String,
    /// Square picture for tiles.
    pub icon_url: Option<String>,
    /// Wide picture for banners.
    pub banner_url: Option<String>,
    pub playing: u64,
    pub max_players: u64,
    pub visits: u64,
    pub likes: u64,
    pub dislikes: u64,
    /// Your total time in this game, in seconds; only recently-played lists have it.
    pub playtime: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shelf {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub places: Vec<Place>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    /// Luduvo's own currency; `None` when the API leaves it out.
    pub balance: Option<i64>,
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_owned()
}

fn num(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Pictures may only come from Luduvo's asset host, which is also all the window's CSP allows.
pub fn asset_url(v: Option<&Value>) -> Option<String> {
    let raw = v?.as_str()?;
    let url = Url::parse(raw).ok()?;
    (url.scheme() == "https" && url.host_str() == Some("assets.luduvo.com") && url.port().is_none() && url.username().is_empty()).then(|| url.to_string())
}

pub fn place(v: &Value) -> Option<Place> {
    // Some lists wrap each game: {"place": {...}, "last_played_at": ...}.
    let v = v.get("place").filter(|p| p.is_object()).unwrap_or(v);
    // Recently-played entries name the game `place_id` (an `id` there, if any, would be the entry's own).
    let id = v.get("place_id").or_else(|| v.get("id")).and_then(Value::as_u64).filter(|id| *id > 0)?;
    let title = text(v, "title");
    let title = if title.is_empty() { text(v, "name") } else { title };
    let square = asset_url(v.get("square_thumbnail_url"));
    let wide = asset_url(v.get("thumbnail_url"));
    Some(Place {
        id,
        title: if title.is_empty() { format!("Game {id}") } else { title },
        creator: {
            let shown = text(v, "owner_display_name");
            if shown.is_empty() { text(v, "owner_username") } else { shown }
        },
        description: text(v, "description"),
        icon_url: square.clone().or_else(|| wide.clone()),
        banner_url: wide.or(square),
        playing: num(v, "active_players"),
        max_players: num(v, "max_players"),
        visits: num(v, "visit_count"),
        likes: num(v, "thumbs_up"),
        dislikes: num(v, "thumbs_down"),
        playtime: v.get("playtime_seconds").and_then(Value::as_u64),
    })
}

/// Any list of games: `{"places": [...]}`, `{"items": [...]}`, `{"data": [...]}` or a bare array.
pub fn place_list(v: &Value) -> Vec<Place> {
    let list = ["places", "items", "data", "results"].iter().find_map(|k| v.get(*k).and_then(Value::as_array)).or_else(|| v.as_array());
    let mut seen = std::collections::HashSet::new();
    list.map(|l| l.iter().filter_map(place).filter(|p| seen.insert(p.id)).collect()).unwrap_or_default()
}

pub fn shelves(v: &Value) -> Vec<Shelf> {
    let Some(list) = v.get("shelves").and_then(Value::as_array) else { return vec![] };
    list.iter()
        .filter_map(|s| {
            let places = place_list(s);
            if places.is_empty() {
                return None;
            }
            let subtitle = text(s, "subtitle");
            Some(Shelf { id: text(s, "id"), title: text(s, "title"), subtitle: (!subtitle.is_empty()).then_some(subtitle), places })
        })
        .collect()
}

pub fn user(v: &Value) -> Option<User> {
    let v = v.get("user").filter(|u| u.is_object()).unwrap_or(v);
    let id = v.get("id").and_then(Value::as_u64)?;
    let name = text(v, "username");
    let shown = text(v, "display_name");
    Some(User {
        id,
        display_name: if shown.is_empty() { name.clone() } else { shown },
        name,
        balance: v.get("balance").and_then(Value::as_i64),
    })
}

/// The extra facts the game page shows, on top of what a tile has.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlaceDetails {
    pub place: Place,
    /// Unix seconds.
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
    /// `all_ages`, `ages_9_plus`, `ages_13_plus` or `ages_17_plus`.
    pub age_rating: Option<String>,
    pub content_warnings: Vec<String>,
    /// The group that owns the game, if any.
    pub group_id: Option<u64>,
    pub group_name: Option<String>,
}

pub fn details(v: &Value) -> Option<PlaceDetails> {
    let place = place(v)?;
    let rating = text(v, "age_rating");
    Some(PlaceDetails {
        place,
        created_at: v.get("created_at").and_then(Value::as_u64),
        updated_at: v.get("updated_at").and_then(Value::as_u64),
        age_rating: (!rating.is_empty()).then_some(rating),
        content_warnings: v
            .get("content_warnings")
            .and_then(Value::as_array)
            .map(|w| w.iter().filter_map(Value::as_str).map(|s| s.replace('_', " ")).collect())
            .unwrap_or_default(),
        group_id: v.get("group_id").and_then(Value::as_u64),
        group_name: Some(text(v, "group_name")).filter(|g| !g.is_empty()),
    })
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rating {
    pub likes: u64,
    pub dislikes: u64,
    /// 1 liked, -1 disliked, 0 not rated.
    pub vote: i64,
}

pub fn rating(v: &Value) -> Rating {
    Rating { likes: num(v, "thumbs_up"), dislikes: num(v, "thumbs_down"), vote: v.get("user_vote").and_then(Value::as_i64).unwrap_or(0).signum() }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerPlayer {
    pub id: u64,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub playing: u64,
    pub max_players: u64,
    pub players: Vec<ServerPlayer>,
}

/// A server id we are willing to put in a URL: Luduvo uses UUIDs.
pub fn valid_server_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

/// The code out of whatever was pasted: a bare code, or a link that ends in one (`.../abc` or `...=abc`).
pub fn private_code(pasted: &str) -> Option<String> {
    let t = pasted.trim().trim_end_matches('/');
    let code = t.rsplit(['/', '=']).next()?.trim();
    ((4..=128).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')).then(|| code.to_owned())
}

/// A small headshot from `headshot_key` (`headshots/<user>/<hash>.png`) on Luduvo's asset host.
fn headshot(v: &Value) -> Option<String> {
    let key = v.get("headshot_key")?.as_str()?;
    let safe = key.starts_with("headshots/") && key.ends_with(".png") && !key.contains("..") && key.bytes().all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(&b));
    if !safe {
        return None;
    }
    let sized = v.get("headshot_has_variants").and_then(Value::as_bool).unwrap_or(false);
    let path = if sized { format!("{}/96.png", &key[..key.len() - 4]) } else { key.to_owned() };
    asset_url(Some(&Value::String(format!("https://assets.luduvo.com/{path}"))))
}

pub fn servers(v: &Value) -> Vec<Server> {
    let list = v.as_array().or_else(|| v.get("servers").and_then(Value::as_array));
    list.into_iter()
        .flatten()
        .filter_map(|s| {
            let id = text(s, "id");
            if !valid_server_id(&id) {
                return None;
            }
            let players = s
                .get("players")
                .and_then(Value::as_array)
                .map(|p| {
                    p.iter()
                        .filter_map(|p| Some(ServerPlayer { id: p.get("user_id")?.as_u64()?, name: text(p, "username"), avatar_url: headshot(p) }))
                        .collect()
                })
                .unwrap_or_default();
            Some(Server { id, playing: num(s, "player_count"), max_players: num(s, "max_players"), players })
        })
        .collect()
}

/// Someone on your friends list (or asking to be), with what Luduvo says they're doing.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    /// `online`, `in_game` or `offline`.
    pub state: String,
    pub place_id: Option<u64>,
    pub place_title: Option<String>,
    /// Luduvo lets you follow them into their server.
    pub joinable: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Presence {
    pub state: String,
    pub place_id: Option<u64>,
    pub place_title: Option<String>,
    pub joinable: bool,
}

/// A person from any friends or requests list. Entries may be wrapped (`{"user": {...}}`) and name the id
/// `user_id`, `friend_id` or `id`.
pub fn friend(v: &Value) -> Option<Friend> {
    let v = ["user", "friend", "requester", "sender"].iter().find_map(|k| v.get(*k).filter(|u| u.is_object())).unwrap_or(v);
    let id = ["user_id", "friend_id", "id"].iter().find_map(|k| v.get(*k).and_then(Value::as_u64)).filter(|id| *id > 0)?;
    let name = text(v, "username");
    let shown = text(v, "display_name");
    let avatar = asset_url(v.get("headshot_url")).or_else(|| asset_url(v.get("avatar_url")));
    Some(Friend {
        id,
        display_name: if shown.is_empty() { name.clone() } else { shown },
        name,
        avatar_url: avatar,
        state: "offline".into(),
        place_id: None,
        place_title: None,
        joinable: false,
    })
}

pub fn friend_list(v: &Value) -> Vec<Friend> {
    let list = ["friends", "requests", "users", "items", "data", "pending"].iter().find_map(|k| v.get(*k).and_then(Value::as_array)).or_else(|| v.as_array());
    let mut seen = std::collections::HashSet::new();
    list.map(|l| l.iter().filter_map(friend).filter(|f| seen.insert(f.id)).collect()).unwrap_or_default()
}

/// `{"presence": [{"user_id": 1, "state": "in_game", "joinable": true, ...}]}` by user id.
pub fn presence(v: &Value) -> std::collections::HashMap<u64, Presence> {
    let list = v.get("presence").and_then(Value::as_array).or_else(|| v.as_array());
    list.into_iter()
        .flatten()
        .filter_map(|p| {
            let id = p.get("user_id").and_then(Value::as_u64)?;
            let state = ["state", "status"].iter().map(|k| text(p, k)).find(|s| !s.is_empty()).unwrap_or_else(|| "offline".into());
            let place = p.get("place").filter(|x| x.is_object());
            let place_id = p.get("place_id").and_then(Value::as_u64).or_else(|| place.and_then(|x| x.get("id")).and_then(Value::as_u64));
            let title = [text(p, "place_title"), text(p, "game_title"), place.map(|x| text(x, "title")).unwrap_or_default()].into_iter().find(|t| !t.is_empty());
            Some((id, Presence { state, place_id, place_title: title, joinable: p.get("joinable").and_then(Value::as_bool).unwrap_or(false) }))
        })
        .collect()
}

/// Your balance from `/me/balance`: `{"balance": 40}` or a bare number.
pub fn balance(v: &Value) -> Option<i64> {
    v.as_i64().or_else(|| v.get("balance").and_then(Value::as_i64))
}

/// Field names and types only, never values: for working out an unfamiliar answer in development logs.
#[cfg(debug_assertions)]
pub fn shape(v: &Value) -> String {
    match v {
        Value::Object(m) => format!("{{{}}}", m.iter().map(|(k, v)| format!("{k}: {}", shape(v))).collect::<Vec<_>>().join(", ")),
        Value::Array(a) => format!("[{} x {}]", a.len(), a.first().map(shape).unwrap_or_default()),
        Value::String(_) => "str".into(),
        Value::Number(_) => "num".into(),
        Value::Bool(_) => "bool".into(),
        Value::Null => "null".into(),
    }
}

/// `(access token, refresh token)` from a sign-in or refresh answer.
pub fn tokens(body: &str) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(body).ok()?;
    let access = v.get("token")?.as_str()?.to_owned();
    let refresh = v.get("refresh_token")?.as_str()?.to_owned();
    (!access.is_empty() && !refresh.is_empty()).then_some((access, refresh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        // Trimmed from a real /places/discovery answer.
        json!({
            "surface": "discover",
            "shelves": [
                { "id": "recommended", "title": "Recommended For You", "subtitle": "Curated for your play style", "places": [
                    { "id": 30, "owner_id": 55, "owner_username": "SAD", "title": "snow battle", "description": "its winter time!!!",
                      "max_players": 20, "visit_count": 188, "thumbs_up": 4, "thumbs_down": 0, "active_players": 0,
                      "thumbnail_url": "https://assets.luduvo.com/place-thumbnails/wide.png",
                      "square_thumbnail_url": "https://assets.luduvo.com/place-thumbnails/square.png" }
                ]},
                { "id": "empty", "title": "Nothing", "places": [] }
            ]
        })
    }

    #[test]
    fn reads_discovery_shelves() {
        let s = shelves(&sample());
        assert_eq!(s.len(), 1, "empty shelves are dropped");
        assert_eq!(s[0].title, "Recommended For You");
        let p = &s[0].places[0];
        assert_eq!((p.id, p.title.as_str(), p.creator.as_str(), p.max_players, p.visits), (30, "snow battle", "SAD", 20, 188));
        assert_eq!(p.icon_url.as_deref(), Some("https://assets.luduvo.com/place-thumbnails/square.png"));
        assert_eq!(p.banner_url.as_deref(), Some("https://assets.luduvo.com/place-thumbnails/wide.png"));
    }

    #[test]
    fn reads_wrapped_and_bare_lists_once_each() {
        let wrapped = json!({ "items": [{ "place": { "id": 7, "title": "A" } }, { "place": { "id": 7, "title": "A" } }, { "id": 8, "title": "B" }] });
        assert_eq!(place_list(&wrapped).iter().map(|p| p.id).collect::<Vec<_>>(), vec![7, 8]);
        assert_eq!(place_list(&json!([{ "id": 1 }]))[0].title, "Game 1");
        assert!(place_list(&json!({ "error": "nope" })).is_empty());
    }

    #[test]
    fn reads_recently_played() {
        // The shape of a real /me/recently-played answer.
        let recent = json!({ "limit": 30, "offset": 0, "places": [{
            "access": "public", "active_players": 3, "last_played_at": 1794838800, "owner_id": 5, "owner_username": "foren",
            "place_id": 1988, "playtime_seconds": 420, "sessions": 2, "title": "Murder Mayhem",
            "square_thumbnail_url": "https://assets.luduvo.com/place-thumbnails/s.png", "thumbnail_url": "https://assets.luduvo.com/place-thumbnails/w.png",
            "thumbs_up": 9, "thumbs_down": 1
        }]});
        let list = place_list(&recent);
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].id, list[0].title.as_str(), list[0].playing, list[0].playtime), (1988, "Murder Mayhem", 3, Some(420)));
    }

    #[test]
    fn reads_friends_in_any_wrapping() {
        let list = json!({ "friends": [
            { "user_id": 3, "username": "nikhil", "display_name": "Nikhil", "headshot_url": "https://assets.luduvo.com/h/3.png" },
            { "user": { "id": 4, "username": "poptart" } },
            { "id": 3, "username": "nikhil" }
        ]});
        let f = friend_list(&list);
        assert_eq!(f.iter().map(|f| f.id).collect::<Vec<_>>(), vec![3, 4]);
        assert_eq!((f[0].display_name.as_str(), f[1].display_name.as_str()), ("Nikhil", "poptart"));
        assert!(f[0].avatar_url.is_some());
        assert_eq!(f[1].state, "offline");
    }

    #[test]
    fn reads_presence() {
        let p = presence(&json!({ "presence": [
            { "user_id": 3, "state": "in_game", "joinable": true, "place_id": 2869, "place_title": "BoomBox!" },
            { "user_id": 4, "state": "online" }
        ]}));
        assert_eq!(p[&3], Presence { state: "in_game".into(), place_id: Some(2869), place_title: Some("BoomBox!".into()), joinable: true });
        assert_eq!(p[&4].state, "online");
        assert!(!p[&4].joinable);
    }

    #[test]
    fn reads_servers_with_their_players() {
        // The shape of a real /places/{id}/servers answer.
        let list = json!([{ "id": "8513af4f-3721-43ed-97ff-2479392b7f5c", "player_count": 2, "max_players": 16, "players": [
            { "user_id": 12117, "username": "EyeLash", "headshot_key": "headshots/12117/1ed4.png", "headshot_has_variants": true },
            { "user_id": 3, "username": "x", "headshot_key": "../../evil.png" }
        ]}, { "id": "../../me", "player_count": 1 }]);
        let s = servers(&list);
        assert_eq!(s.len(), 1, "a server id that isn't a UUID is dropped");
        assert_eq!((s[0].playing, s[0].max_players, s[0].players.len()), (2, 16, 2));
        assert_eq!(s[0].players[0].avatar_url.as_deref(), Some("https://assets.luduvo.com/headshots/12117/1ed4/96.png"));
        assert_eq!(s[0].players[1].avatar_url, None);
    }

    #[test]
    fn finds_the_private_server_code_in_what_was_pasted() {
        assert_eq!(private_code(" Ab12-cd_9 ").as_deref(), Some("Ab12-cd_9"));
        assert_eq!(private_code("https://luduvo.com/games/30?private=Ab12cd").as_deref(), Some("Ab12cd"));
        assert_eq!(private_code("https://luduvo.com/private/Ab12cd/").as_deref(), Some("Ab12cd"));
        assert_eq!(private_code("../../me"), None);
        assert_eq!(private_code("a b"), None);
    }

    #[test]
    fn reads_rating_and_details() {
        assert_eq!(rating(&json!({ "thumbs_up": 4, "thumbs_down": 1, "user_vote": -1 })), Rating { likes: 4, dislikes: 1, vote: -1 });
        let d = details(&json!({ "id": 30, "title": "snow battle", "updated_at": 1794441417, "age_rating": "all_ages", "content_warnings": ["online_interactions"] })).unwrap();
        assert_eq!((d.place.id, d.updated_at, d.age_rating.as_deref()), (30, Some(1794441417), Some("all_ages")));
        assert_eq!(d.content_warnings, vec!["online interactions"]);
        let grouped = details(&json!({ "id": 31, "title": "x", "group_id": 9, "group_name": "Builders" })).unwrap();
        assert_eq!((grouped.group_id, grouped.group_name.as_deref()), (Some(9), Some("Builders")));
    }

    #[test]
    fn reads_balance() {
        assert_eq!(balance(&json!({ "balance": 40 })), Some(40));
        assert_eq!(balance(&json!(12)), Some(12));
        assert_eq!(balance(&json!({})), None);
    }

    #[test]
    fn refuses_pictures_from_other_hosts() {
        assert_eq!(asset_url(Some(&json!("https://evil.test/x.png"))), None);
        assert_eq!(asset_url(Some(&json!("http://assets.luduvo.com/x.png"))), None);
        assert!(asset_url(Some(&json!("https://assets.luduvo.com/x.png"))).is_some());
    }

    #[test]
    fn reads_the_account() {
        let u = user(&json!({ "id": 12, "username": "puppy", "display_name": null, "balance": 40 })).unwrap();
        assert_eq!((u.id, u.name.as_str(), u.display_name.as_str(), u.balance), (12, "puppy", "puppy", Some(40)));
        assert!(user(&json!({ "username": "x" })).is_none());
    }

    #[test]
    fn reads_tokens() {
        assert_eq!(tokens(r#"{"token":"a","refresh_token":"b"}"#), Some(("a".into(), "b".into())));
        assert_eq!(tokens(r#"{"token":"a"}"#), None);
    }
}
