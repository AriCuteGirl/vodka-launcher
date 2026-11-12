//! Pure parsing of Roblox web API responses. No network in here.

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub universe_id: u64,
    pub place_id: u64,
    pub name: String,
    pub playing: Option<u64>,
    pub icon_url: Option<String>,
}

fn bad(what: &str, e: impl std::fmt::Display) -> AppError {
    AppError::Api(format!("{what}: {e}"))
}

#[derive(Deserialize)]
struct Page<T> {
    data: Vec<T>,
}

pub fn authenticated_user(body: &str) -> Result<User> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        id: u64,
        name: String,
        display_name: String,
    }
    let r: Raw = serde_json::from_str(body).map_err(|e| bad("user", e))?;
    Ok(User { id: r.id, name: r.name, display_name: r.display_name, avatar_url: None })
}

pub fn games(body: &str) -> Result<Vec<Game>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        id: u64,
        root_place_id: u64,
        name: String,
        playing: Option<u64>,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("games", e))?;
    Ok(page
        .data
        .into_iter()
        .map(|g| Game {
            universe_id: g.id,
            place_id: g.root_place_id,
            name: g.name,
            playing: g.playing,
            icon_url: None,
        })
        .collect())
}

pub fn favorites(body: &str) -> Result<Vec<Game>> {
    #[derive(Deserialize)]
    struct Place {
        id: u64,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        id: u64,
        name: String,
        root_place: Option<Place>,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("favorites", e))?;
    Ok(page
        .data
        .into_iter()
        .filter_map(|g| {
            Some(Game {
                universe_id: g.id,
                place_id: g.root_place?.id,
                name: g.name,
                playing: None,
                icon_url: None,
            })
        })
        .collect())
}

/// Thumbnail responses: target id -> image URL, finished images only.
pub fn thumbnails(body: &str) -> Result<HashMap<u64, String>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        target_id: u64,
        state: String,
        image_url: Option<String>,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("thumbnails", e))?;
    Ok(page
        .data
        .into_iter()
        .filter(|t| t.state == "Completed")
        .filter_map(|t| Some((t.target_id, t.image_url.filter(|u| u.starts_with("https://"))?)))
        .collect())
}

/// A public server of a game.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub playing: u32,
    pub max_players: u32,
    pub ping: Option<u32>,
}

/// A private server the signed-in account can join.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrivateServer {
    pub name: String,
    pub owner: Option<String>,
    pub playing: u32,
    pub max_players: u32,
    pub access_code: String,
}

/// Private-server access codes are GUID-like. Anything else must never reach a launch link.
pub fn valid_access_code(code: &str) -> bool {
    (1..=128).contains(&code.len()) && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Public servers with room, emptiest first.
pub fn public_servers(body: &str) -> Result<Vec<Server>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        id: String,
        max_players: Option<u32>,
        playing: Option<u32>,
        ping: Option<u32>,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("servers", e))?;
    let mut servers: Vec<Server> = page
        .data
        .into_iter()
        .filter(|s| valid_game_id(&s.id))
        .map(|s| Server { id: s.id, playing: s.playing.unwrap_or(0), max_players: s.max_players.unwrap_or(0), ping: s.ping })
        .filter(|s| s.max_players == 0 || s.playing < s.max_players)
        .collect();
    servers.sort_by_key(|s| s.playing);
    Ok(servers)
}

/// Private servers that come with an access code (the ones this account may join).
pub fn private_servers(body: &str) -> Result<Vec<PrivateServer>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Owner {
        display_name: Option<String>,
        name: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        name: Option<String>,
        access_code: Option<String>,
        max_players: Option<u32>,
        playing: Option<u32>,
        #[serde(default)]
        players: Vec<serde_json::Value>,
        owner: Option<Owner>,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("private servers", e))?;
    Ok(page
        .data
        .into_iter()
        .filter_map(|s| {
            let access_code = s.access_code.filter(|c| valid_access_code(c))?;
            Some(PrivateServer {
                name: s.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Private server".into()),
                owner: s.owner.and_then(|o| o.display_name.or(o.name)),
                playing: s.playing.unwrap_or(s.players.len() as u32),
                max_players: s.max_players.unwrap_or(0),
                access_code,
            })
        })
        .collect())
}

/// One row of the charts page, for example "Top Trending".
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Chart {
    pub name: String,
    pub games: Vec<Game>,
}

/// A game as the search and charts endpoints describe it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    universe_id: Option<u64>,
    root_place_id: Option<u64>,
    name: Option<String>,
    player_count: Option<u64>,
    #[serde(default)]
    is_sponsored: Option<bool>,
}

impl Listed {
    /// Ads and entries without a place to join are dropped.
    fn into_game(self) -> Option<Game> {
        if self.is_sponsored == Some(true) {
            return None;
        }
        Some(Game {
            universe_id: self.universe_id?,
            place_id: self.root_place_id.filter(|id| *id > 0)?,
            name: self.name.filter(|n| !n.trim().is_empty())?,
            playing: self.player_count,
            icon_url: None,
        })
    }
}

pub fn search_results(body: &str) -> Result<Vec<Game>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Group {
        content_group_type: Option<String>,
        #[serde(default)]
        contents: Vec<Listed>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Body {
        search_results: Vec<Group>,
    }
    let body: Body = serde_json::from_str(body).map_err(|e| bad("search", e))?;
    let mut seen = std::collections::HashSet::new();
    Ok(body
        .search_results
        .into_iter()
        .filter(|g| g.content_group_type.as_deref() == Some("Game"))
        .flat_map(|g| g.contents)
        .filter_map(Listed::into_game)
        .filter(|g| seen.insert(g.universe_id))
        .collect())
}

pub fn charts(body: &str, per_chart: usize) -> Result<Vec<Chart>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Sort {
        sort_display_name: Option<String>,
        content_type: Option<String>,
        #[serde(default)]
        games: Vec<Listed>,
    }
    #[derive(Deserialize)]
    struct Body {
        sorts: Vec<Sort>,
    }
    let body: Body = serde_json::from_str(body).map_err(|e| bad("charts", e))?;
    Ok(body
        .sorts
        .into_iter()
        .filter(|s| s.content_type.as_deref() == Some("Games"))
        .filter_map(|s| {
            let games: Vec<Game> = s.games.into_iter().filter_map(Listed::into_game).take(per_chart).collect();
            (!games.is_empty()).then(|| Chart { name: s.sort_display_name.unwrap_or_else(|| "Games".into()), games })
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Presence {
    /// "offline", "online", "in_game" or "studio".
    pub kind: &'static str,
    pub location: Option<String>,
    /// Where they are, when their privacy settings let us see it.
    pub place_id: Option<u64>,
    pub root_place_id: Option<u64>,
    pub universe_id: Option<u64>,
    /// The server they are on. Needed to join them.
    pub game_id: Option<String>,
}

impl Presence {
    pub fn offline() -> Self {
        Presence { kind: "offline", location: None, place_id: None, root_place_id: None, universe_id: None, game_id: None }
    }

    /// The (place, server) to join this person, if Roblox shared it.
    pub fn join_target(&self) -> Option<(u64, &str)> {
        if self.kind != "in_game" {
            return None;
        }
        Some((self.place_id?, self.game_id.as_deref()?))
    }
}

/// Someone on Roblox: a friend, or a person who sent a friend request.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub presence: Presence,
}

/// The ids in a friends or friend-requests page. Roblox no longer includes names there.
pub fn id_list(body: &str) -> Result<Vec<u64>> {
    #[derive(Deserialize)]
    struct Raw {
        id: u64,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("friends", e))?;
    Ok(page.data.into_iter().map(|r| r.id).collect())
}

/// (username, display name) by user id, from the bulk user lookup.
pub fn user_names(body: &str) -> Result<HashMap<u64, (String, String)>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        id: u64,
        name: String,
        display_name: String,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("users", e))?;
    Ok(page.data.into_iter().map(|r| (r.id, (r.name, r.display_name))).collect())
}

/// Server ids are GUIDs. Anything else must never reach a launch link.
pub fn valid_game_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// People in the order a friends list should show them: in a game, online, in Studio, offline; then by name.
pub fn sort_people(people: &mut [Person]) {
    let rank = |p: &Person| match p.presence.kind {
        "in_game" => 0,
        "online" => 1,
        "studio" => 2,
        _ => 3,
    };
    people.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase())));
}

pub fn presences(body: &str) -> Result<HashMap<u64, Presence>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        user_id: u64,
        user_presence_type: u8,
        last_location: Option<String>,
        place_id: Option<u64>,
        root_place_id: Option<u64>,
        universe_id: Option<u64>,
        game_id: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Body {
        user_presences: Vec<Raw>,
    }
    let body: Body = serde_json::from_str(body).map_err(|e| bad("presence", e))?;
    Ok(body
        .user_presences
        .into_iter()
        .map(|p| {
            let kind = match p.user_presence_type {
                1 => "online",
                2 => "in_game",
                3 => "studio",
                _ => "offline",
            };
            let visible = kind != "offline";
            let presence = Presence {
                kind,
                location: p.last_location.filter(|l| !l.is_empty() && visible),
                place_id: p.place_id.filter(|_| visible),
                root_place_id: p.root_place_id.filter(|_| visible),
                universe_id: p.universe_id.filter(|_| visible),
                game_id: p.game_id.filter(|id| visible && valid_game_id(id)),
            };
            (p.user_id, presence)
        })
        .collect())
}

/// Universe ids of the home page's "Continue" row, in order.
pub fn continue_universe_ids(body: &str) -> Result<Vec<u64>> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Item {
        content_type: Option<String>,
        content_id: Option<u64>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Sort {
        topic: Option<String>,
        #[serde(default)]
        recommendation_list: Option<Vec<Item>>,
    }
    #[derive(Deserialize)]
    struct Raw {
        sorts: Vec<Sort>,
    }
    let raw: Raw = serde_json::from_str(body).map_err(|e| bad("home", e))?;
    let row = raw
        .sorts
        .into_iter()
        .find(|s| s.topic.as_deref().is_some_and(|t| t.to_lowercase().starts_with("continue")));
    Ok(row
        .and_then(|s| s.recommendation_list)
        .unwrap_or_default()
        .into_iter()
        .filter(|i| i.content_type.as_deref().is_none_or(|t| t == "Game"))
        .filter_map(|i| i.content_id)
        .collect())
}

/// Puts `games` in the order of `ids` and attaches icons.
pub fn assemble(ids: &[u64], games: Vec<Game>, icons: &HashMap<u64, String>) -> Vec<Game> {
    let mut by_id: HashMap<u64, Game> = games.into_iter().map(|g| (g.universe_id, g)).collect();
    ids.iter()
        .filter_map(|id| by_id.remove(id))
        .map(|mut g| {
            g.icon_url = icons.get(&g.universe_id).cloned();
            g
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAMES: &str = include_str!("../../tests/fixtures/games.json");
    const ICONS: &str = include_str!("../../tests/fixtures/game_icons.json");
    const HEADSHOT: &str = include_str!("../../tests/fixtures/headshot.json");
    const FAVORITES: &str = include_str!("../../tests/fixtures/favorites.json");
    const OMNI: &str = include_str!("../../tests/fixtures/omni.json");

    #[test]
    fn parses_live_games_response() {
        let g = games(GAMES).unwrap();
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].universe_id, 6035872082);
        assert_eq!(g[0].place_id, 17625359962);
        assert_eq!(g[0].name, "RIVALS");
        assert!(g[0].playing.is_some());
    }

    #[test]
    fn parses_live_thumbnails() {
        let icons = thumbnails(ICONS).unwrap();
        assert!(icons[&6035872082].starts_with("https://tr.rbxcdn.com/"));
        assert!(thumbnails(HEADSHOT).unwrap().contains_key(&1));
    }

    #[test]
    fn skips_unfinished_or_unsafe_thumbnails() {
        let body = r#"{"data":[
            {"targetId":1,"state":"Pending","imageUrl":null},
            {"targetId":2,"state":"Blocked","imageUrl":""},
            {"targetId":3,"state":"Completed","imageUrl":"http://plain.example/x"},
            {"targetId":4,"state":"Completed","imageUrl":"https://tr.rbxcdn.com/x"}]}"#;
        let icons = thumbnails(body).unwrap();
        assert_eq!(icons.len(), 1);
        assert!(icons.contains_key(&4));
    }

    #[test]
    fn parses_favorites() {
        let f = favorites(FAVORITES).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[1].place_id, 2753915549);
    }

    #[test]
    fn finds_the_continue_row() {
        assert_eq!(continue_universe_ids(OMNI).unwrap(), vec![6035872082, 994732206]);
        assert!(continue_universe_ids(r#"{"sorts":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn public_servers_are_sorted_emptiest_first_and_skip_full_or_odd_ones() {
        // Field names captured live on 2026-11-12.
        let body = r#"{"previousPageCursor":null,"nextPageCursor":"x","data":[
            {"id":"aaaa-1","maxPlayers":40,"playing":12,"playerTokens":[],"players":[],"fps":59.9,"ping":80},
            {"id":"bbbb-2","maxPlayers":40,"playing":1,"playerTokens":[],"players":[],"fps":60.0,"ping":120},
            {"id":"cccc-3","maxPlayers":40,"playing":40,"fps":60.0,"ping":50},
            {"id":"bad id&x=1","maxPlayers":40,"playing":0}]}"#;
        let servers = public_servers(body).unwrap();
        assert_eq!(servers.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["bbbb-2", "aaaa-1"]);
        assert_eq!((servers[0].playing, servers[0].max_players, servers[0].ping), (1, 40, Some(120)));
        assert!(public_servers(r#"{"data":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn private_servers_need_a_usable_access_code() {
        let body = r#"{"gameJoinRestricted":false,"data":[
            {"name":"Farm","accessCode":"3f2a-9c1e_7b","maxPlayers":20,"players":[{},{}],"owner":{"displayName":"Ari","name":"ari"}},
            {"name":"No code","maxPlayers":20},
            {"name":"Bad","accessCode":"x&placeId=1","maxPlayers":20}]}"#;
        let servers = private_servers(body).unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!((servers[0].name.as_str(), servers[0].owner.as_deref(), servers[0].playing), ("Farm", Some("Ari"), 2));
        assert!(private_servers(r#"{"gameJoinRestricted":false,"data":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn parses_live_search_results() {
        let games = search_results(include_str!("../../tests/fixtures/search.json")).unwrap();
        assert_eq!(games.len(), 4);
        assert!(games.iter().all(|g| g.place_id > 0 && !g.name.is_empty() && g.playing.is_some()));
    }

    #[test]
    fn search_drops_ads_duplicates_and_non_games() {
        let body = r#"{"searchResults":[
            {"contentGroupType":"Game","contents":[{"universeId":1,"rootPlaceId":10,"name":"A","playerCount":5}]},
            {"contentGroupType":"Game","contents":[{"universeId":1,"rootPlaceId":10,"name":"A","playerCount":5}]},
            {"contentGroupType":"Game","contents":[{"universeId":2,"rootPlaceId":20,"name":"Ad","isSponsored":true}]},
            {"contentGroupType":"Game","contents":[{"universeId":3,"rootPlaceId":0,"name":"No place"}]},
            {"contentGroupType":"User","contents":[{"universeId":4,"rootPlaceId":40,"name":"Not a game"}]}]}"#;
        let games = search_results(body).unwrap();
        assert_eq!(games.iter().map(|g| g.universe_id).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn parses_live_charts_and_skips_the_filter_row() {
        let rows = charts(include_str!("../../tests/fixtures/charts.json"), 2).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Top Trending");
        assert!(rows.iter().all(|r| r.games.len() == 2));
    }

    #[test]
    fn parses_live_presence_response() {
        // Captured from presence.roblox.com on 2026-11-11.
        let body = r#"{"userPresences":[{"userPresenceType":0,"lastLocation":"Website","placeId":null,"rootPlaceId":null,"gameId":null,"universeId":null,"userId":1},{"userPresenceType":2,"lastLocation":"RIVALS","placeId":5,"rootPlaceId":5,"gameId":"x","universeId":6,"userId":2}]}"#;
        let p = presences(body).unwrap();
        assert_eq!(p[&1], Presence::offline());
        assert_eq!(p[&2].kind, "in_game");
        assert_eq!(p[&2].location.as_deref(), Some("RIVALS"));
        assert_eq!(p[&2].join_target(), Some((5, "x")));
        assert_eq!(p[&1].join_target(), None);
    }

    #[test]
    fn a_hidden_or_malformed_server_cannot_be_joined() {
        let body = r#"{"userPresences":[
            {"userPresenceType":2,"lastLocation":"A","placeId":5,"gameId":null,"userId":1},
            {"userPresenceType":2,"lastLocation":"B","placeId":5,"gameId":"abc&placeId=666","userId":2},
            {"userPresenceType":2,"lastLocation":"C","placeId":null,"gameId":"0f0e-11","userId":3},
            {"userPresenceType":1,"lastLocation":"Website","placeId":5,"gameId":"0f0e-11","userId":4}]}"#;
        let p = presences(body).unwrap();
        for id in 1..=4 {
            assert_eq!(p[&id].join_target(), None, "user {id}");
        }
        assert!(valid_game_id("3f2a9c1e-7b1d-4c5e-9a10-aa00bb11cc22"));
        assert!(!valid_game_id(""));
    }

    #[test]
    fn friends_pages_give_ids_and_the_lookup_gives_names() {
        // Shape captured live on 2026-11-11: names come back empty in the friends list.
        let ids = id_list(r#"{"data":[{"id":11,"name":"","displayName":""},{"id":12,"name":"","displayName":""}]}"#).unwrap();
        assert_eq!(ids, vec![11, 12]);
        let names = user_names(r#"{"data":[{"hasVerifiedBadge":false,"id":11,"name":"snow","displayName":"Snow"}]}"#).unwrap();
        assert_eq!(names[&11], ("snow".into(), "Snow".into()));
        assert!(id_list(r#"{"data":[]}"#).unwrap().is_empty());
    }

    #[test]
    fn friends_sort_by_what_they_are_doing_then_name() {
        let person = |id, name: &str, kind| Person {
            id,
            name: name.to_lowercase(),
            display_name: name.into(),
            avatar_url: None,
            presence: Presence { kind, ..Presence::offline() },
        };
        let mut people = vec![person(1, "zed", "offline"), person(2, "Bea", "online"), person(3, "amy", "online"), person(4, "Kit", "in_game"), person(5, "Dev", "studio")];
        sort_people(&mut people);
        assert_eq!(people.iter().map(|p| p.id).collect::<Vec<_>>(), vec![4, 3, 2, 5, 1]);
    }

    #[test]
    fn parses_the_signed_in_user() {
        let u = authenticated_user(r#"{"id":7,"name":"ari","displayName":"Ari"}"#).unwrap();
        assert_eq!((u.id, u.display_name.as_str()), (7, "Ari"));
    }

    #[test]
    fn garbage_is_an_api_error_not_a_panic() {
        assert_eq!(games("<html>").unwrap_err().kind(), "api");
        assert_eq!(continue_universe_ids("{}").unwrap_err().kind(), "api");
    }

    #[test]
    fn assemble_keeps_order_and_adds_icons() {
        let icons = thumbnails(ICONS).unwrap();
        let out = assemble(&[994732206, 6035872082, 5], games(GAMES).unwrap(), &icons);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].universe_id, 994732206);
        assert!(out[0].icon_url.is_some());
    }
}
