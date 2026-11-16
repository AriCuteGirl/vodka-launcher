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

/// One thing in Luduvo's store.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreItem {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub creator: String,
    /// C$; `None` when it isn't for sale.
    pub price: Option<u64>,
    pub limited: bool,
    pub asset_type: String,
    pub slot: String,
    pub picture_url: Option<String>,
    pub sales: u64,
    pub likes: u64,
    pub dislikes: u64,
    /// Only on the item's own page.
    pub owners: Option<u64>,
    /// 1 liked, -1 disliked, 0 not rated; only on the item's own page when signed in.
    pub my_vote: i64,
    /// Ends sale at this Unix time, for timed items.
    pub expires_at: Option<u64>,
}

pub fn store_item(v: &Value) -> Option<StoreItem> {
    let id = v.get("id").and_then(Value::as_u64)?;
    let on_sale = v.get("is_on_sale").map(|x| x.as_bool().unwrap_or(x.as_u64() == Some(1))).unwrap_or(true);
    let rating = v.get("rating");
    Some(StoreItem {
        id,
        name: { let n = text(v, "name"); if n.is_empty() { format!("Item {id}") } else { n } },
        description: text(v, "description"),
        creator: text(v, "creator_username"),
        price: if on_sale { v.get("price").and_then(Value::as_u64) } else { None },
        limited: v.get("is_limited").map(|x| x.as_bool().unwrap_or(x.as_u64() == Some(1))).unwrap_or(false),
        asset_type: text(v, "asset_type"),
        slot: text(v, "slot"),
        picture_url: asset_url(v.get("thumbnail_url")),
        sales: num(v, "sales_count"),
        likes: rating.map(|r| num(r, "thumbs_up")).unwrap_or_else(|| num(v, "thumbs_up")),
        dislikes: rating.map(|r| num(r, "thumbs_down")).unwrap_or_else(|| num(v, "thumbs_down")),
        owners: v.get("owners_count").and_then(Value::as_u64),
        my_vote: rating.and_then(|r| r.get("user_vote")).and_then(Value::as_i64).unwrap_or(0).signum(),
        expires_at: v.get("expires_at").and_then(Value::as_u64),
    })
}

/// A page of the store: `{"items": [...], "total": n}`.
pub fn store_page(v: &Value) -> (Vec<StoreItem>, u64) {
    let items = v.get("items").and_then(Value::as_array).map(|l| l.iter().filter_map(store_item).collect()).unwrap_or_default();
    (items, num(v, "total"))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoreCategory {
    pub asset_type: String,
    pub slot: String,
    pub name: String,
}

pub fn store_categories(v: &Value) -> Vec<StoreCategory> {
    v.as_array()
        .or_else(|| v.get("categories").and_then(Value::as_array))
        .map(|l| {
            l.iter()
                .map(|c| StoreCategory { asset_type: text(c, "asset_type"), slot: text(c, "slot"), name: text(c, "display_name") })
                .filter(|c| !c.name.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Something you own or wear: an item and which variant of it.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnedItem {
    pub id: u64,
    pub variant: u64,
    pub name: String,
    pub asset_type: String,
    pub slot: String,
    pub picture_url: Option<String>,
}

/// An entry from the inventory or the equipped list. Entries may be flat or wrap the item (`{"item": {...}}`),
/// and name the item `item_id` or `id`.
pub fn owned_item(v: &Value) -> Option<OwnedItem> {
    let inner = v.get("item").filter(|i| i.is_object()).unwrap_or(v);
    let id = v.get("item_id").and_then(Value::as_u64).or_else(|| inner.get("id").and_then(Value::as_u64)).filter(|id| *id > 0)?;
    let pick = |k: &str| {
        let t = text(inner, k);
        if t.is_empty() { text(v, k) } else { t }
    };
    // Inventory entries call it `item_name`; profile and store entries `name`.
    let name = Some(pick("name")).filter(|n| !n.is_empty()).unwrap_or_else(|| pick("item_name"));
    Some(OwnedItem {
        id,
        variant: v.get("variant_index").and_then(Value::as_u64).unwrap_or(0),
        name: if name.is_empty() { format!("Item {id}") } else { name },
        asset_type: pick("asset_type"),
        // Profile entries list `slots` (a body covers six); the first one names it.
        slot: {
            let one = pick("slot");
            if one.is_empty() { v.get("slots").and_then(Value::as_array).and_then(|s| s.first()).and_then(Value::as_str).unwrap_or_default().to_owned() } else { one }
        },
        picture_url: asset_url(inner.get("thumbnail_url")).or_else(|| asset_url(v.get("thumbnail_url"))),
    })
}

/// Any list of owned or worn items: `{"items": [...]}`, `{"equipped": [...]}`, `{"inventory": [...]}` or a bare array.
pub fn owned_list(v: &Value) -> Vec<OwnedItem> {
    let list = ["equipped_items", "items", "equipped", "inventory", "data"].iter().find_map(|k| v.get(*k).and_then(Value::as_array)).or_else(|| v.as_array());
    let mut seen = std::collections::HashSet::new();
    list.map(|l| l.iter().filter_map(owned_item).filter(|i| seen.insert((i.id, i.variant))).collect()).unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outfit {
    pub id: u64,
    pub name: String,
    pub picture_url: Option<String>,
}

pub fn outfits(v: &Value) -> Vec<Outfit> {
    v.get("outfits")
        .and_then(Value::as_array)
        .map(|l| {
            l.iter()
                .filter_map(|o| Some(Outfit { id: o.get("id")?.as_u64()?, name: text(o, "name"), picture_url: headshot(o) }))
                .collect()
        })
        .unwrap_or_default()
}

/// Someone's Luduvo profile.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub bio: String,
    /// Their short status line, when set.
    pub status: String,
    pub tier: String,
    /// Unix seconds.
    pub member_since: Option<u64>,
    pub last_active: Option<u64>,
    pub friends: Option<u64>,
    pub places: Option<u64>,
    pub items: Option<u64>,
    pub friends_hidden: bool,
    pub badges: Vec<String>,
    pub wearing: Vec<OwnedItem>,
    pub headshot_url: Option<String>,
    /// Filled in separately: the full-body picture and what they're doing.
    pub picture_url: Option<String>,
    pub state: String,
    pub place_id: Option<u64>,
    pub place_title: Option<String>,
    pub joinable: bool,
}

pub fn profile(v: &Value) -> Option<Profile> {
    let id = v.get("user_id").or_else(|| v.get("id")).and_then(Value::as_u64)?;
    let name = text(v, "username");
    let shown = text(v, "display_name");
    Some(Profile {
        id,
        display_name: if shown.is_empty() { name.clone() } else { shown },
        name,
        bio: text(v, "bio"),
        status: text(v, "status"),
        tier: text(v, "tier"),
        member_since: v.get("member_since").and_then(Value::as_u64),
        last_active: v.get("last_active").and_then(Value::as_u64),
        friends: v.get("friend_count").and_then(Value::as_u64),
        places: v.get("place_count").and_then(Value::as_u64),
        items: v.get("item_count").and_then(Value::as_u64),
        friends_hidden: v.get("friends_hidden").and_then(Value::as_bool).unwrap_or(false),
        badges: v.get("badges").and_then(Value::as_array).map(|b| b.iter().map(|x| text(x, "name")).filter(|n| !n.is_empty()).collect()).unwrap_or_default(),
        wearing: owned_list(v),
        headshot_url: asset_url(v.get("headshot_url")),
        picture_url: None,
        state: "offline".into(),
        place_id: None,
        place_title: None,
        joinable: false,
    })
}

/// `/users/{id}/presence`: `{"online": true, "state": "in_game", "joinable": true, "place": {...}}`.
pub fn single_presence(v: &Value) -> Presence {
    let state = ["state", "status"].iter().map(|k| text(v, k)).find(|s| !s.is_empty()).unwrap_or_else(|| {
        if v.get("online").and_then(Value::as_bool).unwrap_or(false) { "online".into() } else { "offline".into() }
    });
    let place = v.get("place").filter(|x| x.is_object());
    let place_id = v.get("place_id").and_then(Value::as_u64).or_else(|| place.and_then(|x| x.get("id")).and_then(Value::as_u64));
    let title = [text(v, "place_title"), place.map(|x| text(x, "title")).unwrap_or_default()].into_iter().find(|t| !t.is_empty());
    let joinable = v.get("joinable").and_then(Value::as_bool).unwrap_or(state == "in_game" && place_id.is_some());
    Presence { state, place_id, place_title: title, joinable }
}

/// A group someone is in.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub members: u64,
    pub role: String,
    pub icon_url: Option<String>,
}

pub fn groups(v: &Value) -> Vec<Group> {
    v.get("groups")
        .and_then(Value::as_array)
        .map(|l| {
            l.iter()
                .filter_map(|m| {
                    let g = m.get("group").filter(|g| g.is_object()).unwrap_or(m);
                    Some(Group {
                        id: g.get("id").or_else(|| m.get("group_id")).and_then(Value::as_u64)?,
                        name: text(g, "name"),
                        members: g.get("member_count").or_else(|| g.get("members")).and_then(Value::as_u64).unwrap_or(0),
                        role: [text(m, "role_name"), m.get("role").map(|r| if r.is_object() { text(r, "name") } else { r.as_str().unwrap_or_default().to_owned() }).unwrap_or_default()]
                            .into_iter()
                            .find(|r| !r.is_empty())
                            .unwrap_or_default(),
                        icon_url: asset_url(g.get("icon_url")).or_else(|| asset_url(g.get("thumbnail_url"))),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Where a starting game is, read from Luduvo's launcher log.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStep {
    /// `checking`, `updating`, `content`, `starting`.
    pub stage: &'static str,
    pub done: Option<u64>,
    pub total: Option<u64>,
}

/// `2026-11-16T13:47:55.861Z` as Unix milliseconds.
fn log_time_ms(stamp: &str) -> Option<u64> {
    let b = stamp.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[10] != b'T' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| stamp.get(r)?.parse::<i64>().ok();
    let (y, m, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (h, mi, sec) = (n(11..13)?, n(14..16)?, n(17..19)?);
    let ms = if b.get(19) == Some(&b'.') { n(20..23).unwrap_or(0) } else { 0 };
    // Days from 1970-01-01 to y-m-d (proleptic Gregorian).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(((days * 24 + h) * 60 + mi) * 60_000 + sec * 1000 + ms).ok()
}

/// The latest step in Luduvo's launcher log written at or after `since_ms` (Unix milliseconds); `None` before it
/// wrote anything for this start.
pub fn launch_step(log: &str, since_ms: u64) -> Option<LaunchStep> {
    let mut step = None;
    for line in log.lines() {
        let Some(when) = line.split_whitespace().next().and_then(log_time_ms) else { continue };
        // A little slack for the two clocks.
        if when + 2000 < since_ms {
            continue;
        }
        let text = line.split_once(" | ").map(|(_, t)| t).unwrap_or(line);
        let next = if text.starts_with("update available") || text.starts_with("downloading") {
            Some(LaunchStep { stage: "updating", done: None, total: None })
        } else if text.starts_with("update applied") || text.contains("up to date") || text.contains("update applied") {
            Some(LaunchStep { stage: "checking", done: None, total: None })
        } else if let Some(rest) = text.strip_prefix("launcher content: warmed ") {
            let mut words = rest.split_whitespace();
            let done = words.next().and_then(|w| w.parse().ok());
            let total = words.nth(1).and_then(|w| w.parse().ok());
            Some(LaunchStep { stage: "content", done, total })
        } else if text.contains(" ready, ") || text.contains("handler registered") {
            Some(LaunchStep { stage: "starting", done: None, total: None })
        } else {
            None
        };
        if next.is_some() {
            step = next;
        }
    }
    step
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
            // The friends-list presence doesn't say `joinable`; someone in a game is, and Luduvo refuses the join if not.
            let joinable = p.get("joinable").and_then(Value::as_bool).unwrap_or(state == "in_game" && place_id.is_some());
            Some((id, Presence { state, place_id, place_title: title, joinable }))
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
        // The real friends-list answer: a place, no joinable field.
        let real = presence(&json!({ "presence": [{ "user_id": 9, "state": "in_game", "place": { "id": 77, "title": "Tower Defense Simulator!" } }] }));
        assert_eq!((real[&9].place_id, real[&9].place_title.as_deref(), real[&9].joinable), (Some(77), Some("Tower Defense Simulator!"), true));
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
    fn reads_the_store() {
        // Trimmed from real /store and /store/{id} answers.
        let page = json!({ "items": [
            { "id": 1996, "creator_username": "Luduvo", "name": "???", "price": 42, "is_on_sale": 1, "is_limited": 0, "asset_type": "accessory", "slot": "hat",
              "thumbnail_url": "https://assets.luduvo.com/items/1996/th.png", "sales_count": 371, "thumbs_up": 62, "thumbs_down": 20, "expires_at": 1821510000 },
            { "id": 7, "name": "Old hat", "price": 10, "is_on_sale": 0 }
        ], "total": 1803 });
        let (items, total) = store_page(&page);
        assert_eq!(total, 1803);
        assert_eq!((items[0].id, items[0].price, items[0].creator.as_str(), items[0].sales, items[0].slot.as_str()), (1996, Some(42), "Luduvo", 371, "hat"));
        assert_eq!(items[1].price, None, "off sale has no price");
        let one = store_item(&json!({ "id": 1996, "price": 42, "is_on_sale": 1, "owners_count": 370, "rating": { "thumbs_up": 62, "thumbs_down": 20, "user_vote": 1 } })).unwrap();
        assert_eq!((one.owners, one.my_vote, one.likes), (Some(370), 1, 62));
        let cats = store_categories(&json!([{ "asset_type": "accessory", "slot": "hat", "display_name": "Hats" }]));
        assert_eq!(cats[0].name, "Hats");
    }

    #[test]
    fn reads_owned_items_in_any_wrapping() {
        let list = json!({ "items": [
            { "item_id": 1996, "variant_index": 0, "name": "???", "asset_type": "accessory", "slot": "hat", "thumbnail_url": "https://assets.luduvo.com/items/1996/th.png" },
            { "item": { "id": 17, "name": "Body", "asset_type": "bundle" }, "variant_index": 2 },
            { "item_id": 1996, "variant_index": 0 }
        ]});
        let items = owned_list(&list);
        assert_eq!(items.iter().map(|i| (i.id, i.variant)).collect::<Vec<_>>(), vec![(1996, 0), (17, 2)]);
        assert_eq!((items[1].name.as_str(), items[1].asset_type.as_str()), ("Body", "bundle"));
        // The profile's `equipped_items`, as the real answer has them.
        let worn = owned_list(&json!({ "user_id": 1, "equipped_items": [
            { "item_id": 486, "name": "Spark Cap", "asset_type": "accessory", "thumbnail_url": "https://assets.luduvo.com/item_previews/486/a.png", "slots": ["hat"], "variant_index": 0 },
            { "item_id": 17, "name": "Luduvo Default Bundle", "asset_type": "avatar_bundle", "slots": ["head", "torso"], "variant_index": 0 }
        ]}));
        assert_eq!((worn[0].id, worn[0].slot.as_str(), worn[1].asset_type.as_str()), (486, "hat", "avatar_bundle"));
        assert!(worn[0].picture_url.is_some());
        // An inventory entry, as the real answer has it: its own `id`, the item's `item_id` and `item_name`.
        let inv = owned_list(&json!({ "hidden": false, "items": [{ "id": 9001, "item_id": 486, "item_name": "Spark Cap", "asset_type": "accessory", "variant_index": 0 }] }));
        assert_eq!((inv[0].id, inv[0].name.as_str()), (486, "Spark Cap"));
        let o = outfits(&json!({ "outfits": [{ "id": 3, "name": "Winter", "headshot_key": "headshots/1/ab.png", "headshot_has_variants": false }] }));
        assert_eq!((o[0].id, o[0].name.as_str(), o[0].picture_url.as_deref()), (3, "Winter", Some("https://assets.luduvo.com/headshots/1/ab.png")));
    }

    #[test]
    fn reads_a_profile_and_presence() {
        let p = profile(&json!({ "user_id": 1, "username": "luduvo", "display_name": "Luduvo", "bio": "hi", "status": "", "tier": "free",
            "member_since": 1766534400, "friend_count": 2, "place_count": 1, "friends_hidden": false,
            "badges": [{ "name": "Administrator" }],
            "equipped_items": [{ "item_id": 486, "name": "Spark Cap", "asset_type": "accessory", "slots": ["hat"], "variant_index": 0 }] })).unwrap();
        assert_eq!((p.id, p.display_name.as_str(), p.friends, p.badges.clone(), p.wearing.len()), (1, "Luduvo", Some(2), vec!["Administrator".to_string()], 1));
        let pr = single_presence(&json!({ "online": true, "state": "in_game", "joinable": true, "place": { "id": 30, "title": "snow battle" } }));
        assert_eq!((pr.state.as_str(), pr.place_id, pr.joinable), ("in_game", Some(30), true));
        assert_eq!(single_presence(&json!({ "online": true })).state, "online");
        let g = groups(&json!({ "groups": [{ "group": { "id": 9, "name": "Builders", "member_count": 40 }, "role": { "name": "Owner" } }] }));
        assert_eq!((g[0].id, g[0].name.as_str(), g[0].members, g[0].role.as_str()), (9, "Builders", 40, "Owner"));
    }

    #[test]
    fn follows_the_launcher_log() {
        // Real lines from a first start.
        let log = "2026-11-16T13:47:55.861Z I 421989 | update available: seq 0 -> 2000054000, version <none> -> 54\n\
                   2026-11-16T13:47:58.214Z I 421989 | update applied: version=54 seq=2000054000\n\
                   2026-11-16T13:48:03.185Z I 422142 | launcher content: warmed 50 of 1922 objects\n\
                   2026-11-16T13:49:00.645Z W 422139 | launcher content: download failed for shaders/d3d12/x.json\n\
                   2026-11-16T13:49:41.952Z I 422144 | launcher content: warmed 850 of 1922 objects\n";
        let start = log_time_ms("2026-11-16T13:47:55.000Z").unwrap();
        assert_eq!(launch_step(log, start), Some(LaunchStep { stage: "content", done: Some(850), total: Some(1922) }));
        assert_eq!(launch_step(&log[..log.find("2026-11-16T13:48").unwrap()], start).unwrap().stage, "checking");
        let ready = "2026-11-16T18:58:55.645Z I 821788 | launcher content: version 14 ready, 1922 names in /x\n";
        assert_eq!(launch_step(ready, log_time_ms("2026-11-16T18:58:50.000Z").unwrap()).unwrap().stage, "starting");
        // Lines from an earlier start don't count.
        assert_eq!(launch_step(log, start + 10 * 60_000), None);
        assert_eq!(log_time_ms("1970-01-01T00:00:01.500Z"), Some(1500));
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
