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

/// An avatar item: something worn, owned, or for sale.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u64,
    pub name: String,
    /// Roblox's type name, for example "Hat" or "Shirt". "Bundle" for bundles.
    pub kind: String,
    pub image_url: Option<String>,
    /// Robux price in the catalog; `None` when not for sale or not a catalog listing.
    pub price: Option<u64>,
    pub creator: Option<String>,
    pub description: Option<String>,
    /// One of the accessory types that Roblox's "Advanced" box accepts by id.
    pub accessory: bool,
    /// Layer order and fit for layered clothing; passed back unchanged when worn.
    #[serde(skip)]
    pub meta: Option<serde_json::Value>,
}

impl Item {
    fn basic(id: u64, name: String, kind: String) -> Self {
        let accessory = is_accessory(&kind);
        Item { id, name, kind, image_url: None, price: None, creator: None, description: None, accessory, meta: None }
    }
}

/// Accessories are the items Roblox's "Advanced" box takes by id (up to ten in total).
pub fn is_accessory(kind: &str) -> bool {
    kind == "Hat" || kind.ends_with("Accessory")
}

pub const MAX_ADVANCED_ACCESSORIES: usize = 10;

/// What the account is wearing right now, plus the raw avatar (colours, scale, rig) for saving as an outfit.
pub fn current_avatar(body: &str) -> Result<(Vec<Item>, serde_json::Value)> {
    let raw: serde_json::Value = serde_json::from_str(body).map_err(|e| bad("avatar", e))?;
    let assets = raw.get("assets").and_then(|a| a.as_array()).ok_or_else(|| bad("avatar", "no assets"))?;
    let items = assets
        .iter()
        .filter_map(|a| {
            let mut item = Item::basic(
                a.get("id")?.as_u64()?,
                a.get("name").and_then(|n| n.as_str()).unwrap_or("Item").to_owned(),
                a.get("assetType").and_then(|t| t.get("name")).and_then(|n| n.as_str()).unwrap_or("Item").to_owned(),
            );
            item.meta = a.get("meta").filter(|m| m.is_object()).cloned();
            Some(item)
        })
        .collect();
    Ok((items, raw))
}

/// One page of the account's own items, and the cursor for the next page.
pub fn inventory(body: &str) -> Result<(Vec<Item>, Option<String>)> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        asset_id: u64,
        name: Option<String>,
        asset_type: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Body {
        next_page_cursor: Option<String>,
        data: Vec<Raw>,
    }
    let page: Body = serde_json::from_str(body).map_err(|e| bad("inventory", e))?;
    let items = page
        .data
        .into_iter()
        .map(|r| Item::basic(r.asset_id, r.name.unwrap_or_else(|| "Item".into()), r.asset_type.unwrap_or_else(|| "Item".into())))
        .collect();
    Ok((items, page.next_page_cursor.filter(|c| !c.is_empty())))
}

/// One page of catalog search results, and the cursor for the next page.
pub fn catalog(body: &str) -> Result<(Vec<Item>, Option<String>)> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Raw {
        id: u64,
        item_type: Option<String>,
        name: Option<String>,
        description: Option<String>,
        creator_name: Option<String>,
        price: Option<u64>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Body {
        next_page_cursor: Option<String>,
        data: Vec<Raw>,
    }
    let page: Body = serde_json::from_str(body).map_err(|e| bad("catalog", e))?;
    let items = page
        .data
        .into_iter()
        .map(|r| Item {
            id: r.id,
            name: r.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Item".into()),
            kind: r.item_type.unwrap_or_else(|| "Asset".into()),
            image_url: None,
            price: r.price,
            creator: r.creator_name.filter(|c| !c.is_empty()),
            description: r.description.filter(|d| !d.trim().is_empty()),
            accessory: false,
            meta: None,
        })
        .collect();
    Ok((items, page.next_page_cursor.filter(|c| !c.is_empty())))
}

/// The body for "wear exactly these items". Items already worn keep their layering details.
pub fn wearing_body(ids: &[u64], worn: &[Item]) -> serde_json::Value {
    let assets: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| {
            let mut asset = serde_json::json!({ "id": id });
            if let Some(meta) = worn.iter().find(|w| w.id == *id).and_then(|w| w.meta.clone()) {
                asset["meta"] = meta;
            }
            asset
        })
        .collect();
    serde_json::json!({ "assets": assets })
}

/// The full list to wear when the Advanced box is saved: everything worn that isn't an accessory,
/// plus exactly the accessory ids the user entered.
pub fn with_accessories(worn: &[Item], accessory_ids: &[u64]) -> Vec<u64> {
    let mut ids: Vec<u64> = worn.iter().filter(|i| !i.accessory).map(|i| i.id).collect();
    for id in accessory_ids {
        if *id > 0 && !ids.contains(id) {
            ids.push(*id);
        }
    }
    ids
}

/// Ids Roblox refused to put on (not owned, not wearable, or over a limit).
pub fn refused_ids(body: &str) -> Vec<u64> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else { return vec![] };
    let mut ids: Vec<u64> = v.get("invalidAssetIds").and_then(|a| a.as_array()).into_iter().flatten().filter_map(|x| x.as_u64()).collect();
    ids.extend(v.get("invalidAssets").and_then(|a| a.as_array()).into_iter().flatten().filter_map(|x| x.get("id")?.as_u64()));
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The body for "save what I'm wearing as an outfit", built from the current avatar.
pub fn outfit_body(name: &str, avatar: &serde_json::Value) -> serde_json::Value {
    let assets: Vec<serde_json::Value> = avatar
        .get("assets")
        .and_then(|a| a.as_array())
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let mut asset = serde_json::json!({ "id": a.get("id")?.as_u64()? });
            if let Some(meta) = a.get("meta").filter(|m| m.is_object()) {
                asset["meta"] = meta.clone();
            }
            Some(asset)
        })
        .collect();
    serde_json::json!({
        "name": name,
        "bodyColor3s": avatar.get("bodyColor3s"),
        "assets": assets,
        "scale": avatar.get("scales"),
        "playerAvatarType": avatar.get("playerAvatarType"),
        "outfitType": "Avatar",
    })
}

/// What buying an item would involve, as shown to the user before they confirm.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub asset_id: u64,
    pub name: String,
    /// Robux. `None` when the item has no fixed price.
    pub price: Option<u64>,
    /// Vodka can buy it: on sale at a fixed price, not a limited or resale item.
    pub buyable: bool,
    /// Why not, in plain words, when it can't be bought here.
    pub why_not: Option<String>,
    pub owned: bool,
    pub balance: Option<u64>,
}

/// Reads an item's sale details and decides whether Vodka can buy it.
pub fn offer(details: &str, owned: bool, balance: Option<u64>) -> Result<Offer> {
    let d: serde_json::Value = serde_json::from_str(details).map_err(|e| bad("item", e))?;
    let asset_id = d.get("AssetId").and_then(|v| v.as_u64()).ok_or_else(|| bad("item", "no id"))?;
    let price = d.get("PriceInRobux").and_then(|v| v.as_u64());
    let flag = |k: &str| d.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
    let why_not = if owned {
        Some("You already own this.".to_owned())
    } else if !flag("IsForSale") || price.is_none() {
        Some("This item isn't on sale right now.".to_owned())
    } else if flag("IsLimited") || flag("IsLimitedUnique") {
        Some("Limited items are sold by other players at changing prices. Buy those on roblox.com.".to_owned())
    } else if balance.is_some_and(|b| b < price.unwrap_or(0)) {
        Some("You don't have enough Robux for this.".to_owned())
    } else {
        None
    };
    Ok(Offer {
        asset_id,
        name: d.get("Name").and_then(|v| v.as_str()).unwrap_or("Item").to_owned(),
        price,
        buyable: why_not.is_none(),
        why_not,
        owned,
        balance,
    })
}

/// The purchase request for an item: (url, body). Refuses if the price is no longer the one the user agreed to.
pub fn purchase_request(details: &str, agreed_price: u64, buyer_id: u64, idempotency_key: &str) -> Result<(String, serde_json::Value)> {
    let o = offer(details, false, None)?;
    if !o.buyable {
        return Err(AppError::Other(o.why_not.unwrap_or_else(|| "This item can't be bought here.".into())));
    }
    if o.price != Some(agreed_price) {
        return Err(AppError::Other(format!(
            "The price changed to {} Robux since you looked. Nothing was bought; open the item again to see the new price.",
            o.price.unwrap_or(0)
        )));
    }
    let d: serde_json::Value = serde_json::from_str(details).map_err(|e| bad("item", e))?;
    let creator = d.get("Creator");
    let seller_id = creator.and_then(|c| c.get("CreatorTargetId")).and_then(|v| v.as_u64()).ok_or_else(|| bad("item", "no seller"))?;
    let seller_type = creator.and_then(|c| c.get("CreatorType")).and_then(|v| v.as_str()).unwrap_or("User");
    let text = |k: &str| d.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'));
    // Newer items are sold through the "collectibles" system even at a fixed price; older ones through products.
    if let (Some(item), Some(product)) = (text("CollectibleItemId"), text("CollectibleProductId")) {
        return Ok((
            format!("https://apis.roblox.com/marketplace-sales/v1/item/{item}/purchase-item"),
            serde_json::json!({
                "collectibleItemId": item,
                "collectibleProductId": product,
                "expectedCurrency": 1,
                "expectedPrice": agreed_price,
                "expectedPurchaserId": buyer_id.to_string(),
                "expectedPurchaserType": "User",
                "expectedSellerId": seller_id.to_string(),
                "expectedSellerType": seller_type,
                "idempotencyKey": idempotency_key,
            }),
        ));
    }
    let product_id = d.get("ProductId").and_then(|v| v.as_u64()).filter(|id| *id > 0).ok_or_else(|| bad("item", "no product"))?;
    Ok((
        format!("https://economy.roblox.com/v1/purchases/products/{product_id}"),
        serde_json::json!({ "expectedCurrency": 1, "expectedPrice": agreed_price, "expectedSellerId": seller_id }),
    ))
}

/// Whether Roblox says the purchase went through; its own explanation otherwise.
pub fn purchase_outcome(body: &str) -> Result<()> {
    let v: serde_json::Value = serde_json::from_str(body).map_err(|e| bad("purchase", e))?;
    if v.get("purchased").and_then(|p| p.as_bool()) == Some(true) {
        return Ok(());
    }
    let reason = ["errorMessage", "errorMsg", "purchaseResult", "reason"]
        .iter()
        .find_map(|k| v.get(*k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()))
        .unwrap_or("Roblox didn't say why.");
    Err(AppError::Other(format!("Roblox didn't complete the purchase: {reason}")))
}

/// How the signed-in account stands with a game.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameStanding {
    pub favorited: bool,
    /// `Some(true)` liked, `Some(false)` disliked, `None` not rated.
    pub vote: Option<bool>,
    pub can_vote: bool,
    /// Why rating isn't allowed, in plain words, when it isn't.
    pub why_not: Option<String>,
    pub up_votes: Option<u64>,
    pub down_votes: Option<u64>,
}

pub fn game_standing(favorite: &str, mine: &str, totals: Option<&str>) -> Result<GameStanding> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Favorite {
        is_favorited: bool,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Mine {
        can_vote: bool,
        user_vote: Option<bool>,
        reason_for_not_voteable: Option<String>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Totals {
        up_votes: Option<u64>,
        down_votes: Option<u64>,
    }
    let favorite: Favorite = serde_json::from_str(favorite).map_err(|e| bad("favorite", e))?;
    let mine: Mine = serde_json::from_str(mine).map_err(|e| bad("vote", e))?;
    let totals: Option<Totals> = totals.and_then(|t| serde_json::from_str(t).ok());
    let why_not = (!mine.can_vote).then(|| match mine.reason_for_not_voteable.as_deref() {
        Some("PlayGame") => "Play this game first, then you can rate it.".to_owned(),
        Some("FloodCheckThresholdMet") => "You've rated a lot recently. Try again later.".to_owned(),
        Some("EmailIsNotVerified") => "Roblox needs a verified email before you can rate games.".to_owned(),
        Some("AccountNotOldEnough") => "This account is too new to rate games.".to_owned(),
        _ => "Roblox doesn't let this account rate this game right now.".to_owned(),
    });
    Ok(GameStanding {
        favorited: favorite.is_favorited,
        vote: mine.user_vote,
        can_vote: mine.can_vote,
        why_not,
        up_votes: totals.as_ref().and_then(|t| t.up_votes),
        down_votes: totals.as_ref().and_then(|t| t.down_votes),
    })
}

/// A saved avatar outfit.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outfit {
    pub id: u64,
    pub name: String,
    pub image_url: Option<String>,
}

pub fn outfits(body: &str) -> Result<Vec<Outfit>> {
    #[derive(Deserialize)]
    struct Raw {
        id: u64,
        name: Option<String>,
    }
    let page: Page<Raw> = serde_json::from_str(body).map_err(|e| bad("outfits", e))?;
    Ok(page
        .data
        .into_iter()
        .map(|o| Outfit { id: o.id, name: o.name.filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Outfit".into()), image_url: None })
        .collect())
}

/// The avatar-API requests that put an outfit on: (what it changes, path, body).
/// Built from the outfit's details so nothing but that outfit's own values is sent.
pub fn wear_requests(details: &str) -> Result<Vec<(&'static str, &'static str, serde_json::Value)>> {
    let d: serde_json::Value = serde_json::from_str(details).map_err(|e| bad("outfit", e))?;
    let assets = d.get("assets").and_then(|a| a.as_array()).ok_or_else(|| bad("outfit", "no assets"))?;
    let wearing: Vec<serde_json::Value> = assets
        .iter()
        .filter_map(|a| {
            let id = a.get("id")?.as_u64()?;
            let mut item = serde_json::json!({ "id": id });
            // Layered clothing carries its layer order here.
            if let Some(meta) = a.get("meta").filter(|m| m.is_object()) {
                item["meta"] = meta.clone();
            }
            Some(item)
        })
        .collect();
    let mut requests = vec![("items", "/v2/avatar/set-wearing-assets", serde_json::json!({ "assets": wearing }))];
    if let Some(colors) = d.get("bodyColor3s").filter(|c| c.is_object()) {
        requests.push(("body colors", "/v2/avatar/set-body-colors", colors.clone()));
    }
    if let Some(scale) = d.get("scale").filter(|c| c.is_object()) {
        requests.push(("body scale", "/v1/avatar/set-scales", scale.clone()));
    }
    if let Some(kind) = d.get("playerAvatarType").and_then(|k| k.as_str()).filter(|k| matches!(*k, "R6" | "R15")) {
        requests.push(("rig type", "/v1/avatar/set-player-avatar-type", serde_json::json!({ "playerAvatarType": kind })));
    }
    Ok(requests)
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

/// The server to gather `needed` accounts in: the emptiest one that has room for all of them,
/// or simply the emptiest if none does. `servers` must already be sorted emptiest first.
pub fn server_for_group(servers: &[Server], needed: u32) -> Option<&Server> {
    servers
        .iter()
        .find(|s| s.max_players == 0 || s.max_players.saturating_sub(s.playing) >= needed)
        .or_else(|| servers.first())
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

    const AVATAR: &str = r#"{"scales":{"height":1.0},"playerAvatarType":"R15","bodyColor3s":{"headColor3":"FFCC99"},
        "assets":[{"id":11,"name":"Cap","assetType":{"id":8,"name":"Hat"},"currentVersionId":1},
                  {"id":12,"name":"Shirt","assetType":{"id":11,"name":"Shirt"},"currentVersionId":2},
                  {"id":13,"name":"Jacket","assetType":{"id":67,"name":"JacketAccessory"},"meta":{"order":11,"version":1}}]}"#;

    #[test]
    fn reads_what_the_avatar_is_wearing() {
        let (items, raw) = current_avatar(AVATAR).unwrap();
        assert_eq!(items.iter().map(|i| (i.id, i.kind.as_str())).collect::<Vec<_>>(), vec![(11, "Hat"), (12, "Shirt"), (13, "JacketAccessory")]);
        assert!(items[2].meta.is_some() && items[0].meta.is_none());
        assert_eq!(raw["playerAvatarType"], "R15");
        assert!(is_accessory("Hat") && is_accessory("HairAccessory") && !is_accessory("Shirt") && !is_accessory("DynamicHead"));
    }

    #[test]
    fn wearing_keeps_layering_for_items_already_on() {
        let (worn, _) = current_avatar(AVATAR).unwrap();
        let body = wearing_body(&[12, 13, 99], &worn);
        assert_eq!(body, serde_json::json!({"assets":[{"id":12},{"id":13,"meta":{"order":11,"version":1}},{"id":99}]}));
    }

    #[test]
    fn advanced_replaces_accessories_and_keeps_everything_else() {
        let (worn, _) = current_avatar(AVATAR).unwrap();
        assert!(worn[0].accessory && !worn[1].accessory && worn[2].accessory);
        // Cap (11) and Jacket (13) are accessories; the shirt (12) stays whatever is entered.
        assert_eq!(with_accessories(&worn, &[13, 77, 0, 77]), vec![12, 13, 77]);
        assert_eq!(with_accessories(&worn, &[]), vec![12]);
        assert_eq!(MAX_ADVANCED_ACCESSORIES, 10);
    }

    #[test]
    fn reports_items_roblox_refused() {
        assert_eq!(refused_ids(r#"{"invalidAssets":[{"id":5,"name":"x"}],"invalidAssetIds":[9,5],"success":false}"#), vec![5, 9]);
        assert!(refused_ids(r#"{"invalidAssets":[],"invalidAssetIds":[],"success":true}"#).is_empty());
        assert!(refused_ids("not json").is_empty());
    }

    #[test]
    fn an_outfit_is_saved_from_the_current_look() {
        let (_, raw) = current_avatar(AVATAR).unwrap();
        let body = outfit_body("Winter", &raw);
        assert_eq!(body["name"], "Winter");
        assert_eq!(body["assets"].as_array().unwrap().len(), 3);
        assert_eq!(body["assets"][2]["meta"]["order"], 11);
        assert_eq!(body["scale"]["height"], 1.0);
        assert_eq!(body["playerAvatarType"], "R15");
    }

    #[test]
    fn parses_inventory_and_catalog_pages() {
        // Field names captured live on 2026-11-12.
        let (items, next) = inventory(r#"{"previousPageCursor":null,"nextPageCursor":"abc","data":[{"assetId":7,"name":"Bow","assetType":"HairAccessory","created":"2024"}]}"#).unwrap();
        assert_eq!((items[0].id, items[0].kind.as_str(), next.as_deref()), (7, "HairAccessory", Some("abc")));
        let (found, next) = catalog(r#"{"keyword":"cat","nextPageCursor":null,"data":[
            {"id":1,"itemType":"Asset","assetType":8,"name":"Cat Ears","description":"meow","creatorName":"UGC","price":40},
            {"id":2,"itemType":"Bundle","name":"","description":"","creatorName":""}]}"#).unwrap();
        assert_eq!(next, None);
        assert_eq!((found[0].price, found[0].creator.as_deref(), found[0].description.as_deref()), (Some(40), Some("UGC"), Some("meow")));
        assert_eq!((found[1].name.as_str(), found[1].kind.as_str(), found[1].price), ("Item", "Bundle", None));
    }

    const CLASSIC: &str = r#"{"AssetId":50,"ProductId":900,"Name":"Cap","PriceInRobux":15,"IsForSale":true,"IsLimited":false,"IsLimitedUnique":false,
        "CollectibleItemId":null,"CollectibleProductId":null,"Creator":{"Id":1,"Name":"Maker","CreatorType":"Group","CreatorTargetId":77}}"#;
    const COLLECTIBLE: &str = r#"{"AssetId":51,"ProductId":0,"Name":"Ears","PriceInRobux":40,"IsForSale":true,"IsLimited":false,"IsLimitedUnique":false,
        "CollectibleItemId":"aaaa-1111","CollectibleProductId":"bbbb-2222","Creator":{"CreatorType":"User","CreatorTargetId":88}}"#;

    #[test]
    fn an_offer_says_whether_and_why_not() {
        let ok = offer(CLASSIC, false, Some(100)).unwrap();
        assert!(ok.buyable && ok.why_not.is_none());
        assert_eq!((ok.price, ok.balance), (Some(15), Some(100)));
        assert!(offer(CLASSIC, true, Some(100)).unwrap().why_not.unwrap().contains("already own"));
        assert!(offer(CLASSIC, false, Some(14)).unwrap().why_not.unwrap().contains("enough Robux"));
        let off_sale = CLASSIC.replace(r#""IsForSale":true"#, r#""IsForSale":false"#);
        assert!(!offer(&off_sale, false, Some(100)).unwrap().buyable);
        let limited = CLASSIC.replace(r#""IsLimited":false"#, r#""IsLimited":true"#);
        assert!(offer(&limited, false, Some(100)).unwrap().why_not.unwrap().contains("roblox.com"));
    }

    #[test]
    fn a_purchase_states_the_exact_price_and_seller() {
        let (url, body) = purchase_request(CLASSIC, 15, 5, "key").unwrap();
        assert_eq!(url, "https://economy.roblox.com/v1/purchases/products/900");
        assert_eq!(body, serde_json::json!({"expectedCurrency":1,"expectedPrice":15,"expectedSellerId":77}));
        let (url, body) = purchase_request(COLLECTIBLE, 40, 5, "key").unwrap();
        assert_eq!(url, "https://apis.roblox.com/marketplace-sales/v1/item/aaaa-1111/purchase-item");
        assert_eq!((body["expectedPrice"].as_u64(), body["expectedPurchaserId"].as_str(), body["expectedSellerId"].as_str()), (Some(40), Some("5"), Some("88")));
        assert_eq!((body["collectibleProductId"].as_str(), body["idempotencyKey"].as_str()), (Some("bbbb-2222"), Some("key")));
    }

    #[test]
    fn nothing_is_bought_if_the_price_moved_or_the_item_is_not_plainly_for_sale() {
        let err = purchase_request(CLASSIC, 10, 5, "key").unwrap_err();
        assert!(err.to_string().contains("price changed to 15"));
        let limited = CLASSIC.replace(r#""IsLimited":false"#, r#""IsLimited":true"#);
        assert!(purchase_request(&limited, 15, 5, "key").is_err());
        let off_sale = COLLECTIBLE.replace(r#""IsForSale":true"#, r#""IsForSale":false"#);
        assert!(purchase_request(&off_sale, 40, 5, "key").is_err());
    }

    #[test]
    fn purchase_results_are_read_honestly() {
        assert!(purchase_outcome(r#"{"purchased":true,"reason":"Success"}"#).is_ok());
        assert!(purchase_outcome(r#"{"purchased":false,"reason":"InsufficientFunds","errorMsg":"You need more Robux."}"#).unwrap_err().to_string().contains("You need more Robux."));
        assert!(purchase_outcome(r#"{"purchaseResult":"Flooded","purchased":false}"#).unwrap_err().to_string().contains("Flooded"));
        assert!(purchase_outcome("{}").unwrap_err().to_string().contains("didn't say why"));
    }

    #[test]
    fn reads_favorite_and_vote_status() {
        // All three bodies captured live on 2026-11-12.
        let s = game_standing(
            r#"{"isFavorited": false}"#,
            r#"{"canVote": false, "userVote": null, "reasonForNotVoteable": "PlayGame"}"#,
            Some(r#"{"id": 6035872082, "upVotes": 10984750, "downVotes": 746771}"#),
        )
        .unwrap();
        assert!(!s.favorited && !s.can_vote && s.vote.is_none());
        assert_eq!(s.why_not.as_deref(), Some("Play this game first, then you can rate it."));
        assert_eq!((s.up_votes, s.down_votes), (Some(10984750), Some(746771)));

        let liked = game_standing(r#"{"isFavorited":true}"#, r#"{"canVote":true,"userVote":true,"reasonForNotVoteable":""}"#, None).unwrap();
        assert!(liked.favorited && liked.can_vote);
        assert_eq!((liked.vote, liked.why_not, liked.up_votes), (Some(true), None, None));
        assert!(game_standing("{}", "{}", None).is_err());
    }

    #[test]
    fn outfits_keep_their_ids_and_get_a_name() {
        // Field names captured live on 2026-11-12.
        let list = outfits(r#"{"data":[{"id":7,"name":"Winter","isEditable":true,"outfitType":"Avatar"},{"id":8,"name":"","isEditable":true}],"paginationToken":"x"}"#).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].id, list[0].name.as_str()), (7, "Winter"));
        assert_eq!(list[1].name, "Outfit");
    }

    #[test]
    fn wearing_an_outfit_sends_only_that_outfits_values() {
        let details = r#"{"id":7,"name":"Winter",
            "assets":[{"id":11,"name":"Hat","assetType":{"id":8,"name":"Hat"},"currentVersionId":1},
                      {"id":12,"name":"Jacket","assetType":{"id":67},"meta":{"order":11,"version":1}}],
            "bodyColor3s":{"headColor3":"FFCC99","torsoColor3":"FFCC99"},
            "scale":{"height":1.0,"width":1.0,"head":1.0,"depth":1.0,"proportion":0.0,"bodyType":0.0},
            "playerAvatarType":"R15","outfitType":"Avatar"}"#;
        let reqs = wear_requests(details).unwrap();
        assert_eq!(reqs.iter().map(|r| r.1).collect::<Vec<_>>(), vec![
            "/v2/avatar/set-wearing-assets", "/v2/avatar/set-body-colors", "/v1/avatar/set-scales", "/v1/avatar/set-player-avatar-type"]);
        assert_eq!(reqs[0].2, serde_json::json!({"assets":[{"id":11},{"id":12,"meta":{"order":11,"version":1}}]}));
        assert_eq!(reqs[3].2, serde_json::json!({"playerAvatarType":"R15"}));
        // An outfit without colours or scale only changes the items.
        assert_eq!(wear_requests(r#"{"assets":[],"playerAvatarType":"Weird"}"#).unwrap().len(), 1);
        assert!(wear_requests(r#"{"name":"no assets"}"#).is_err());
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
    fn a_group_goes_to_the_emptiest_server_with_room_for_everyone() {
        let server = |id: &str, playing, max_players| Server { id: id.into(), playing, max_players, ping: None };
        let list = [server("a", 38, 40), server("b", 39, 40), server("c", 30, 40)];
        // Sorted emptiest-first by the caller; "a" has 2 free, "c" has 10.
        assert_eq!(server_for_group(&list, 2).unwrap().id, "a");
        assert_eq!(server_for_group(&list, 4).unwrap().id, "c");
        // Nobody has room for 20: take the emptiest and let Roblox overflow the rest.
        assert_eq!(server_for_group(&list, 20).unwrap().id, "a");
        assert!(server_for_group(&[], 3).is_none());
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
