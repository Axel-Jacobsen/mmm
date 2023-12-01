use std::collections::HashMap;

use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
enum MarketOutcome {
    YES,
    NO,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct UserList {
    users: Vec<User>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    /// from <https://docs.manifold.markets/api#get-v0users>
    id: String,
    name: String,
    username: String,
    url: Option<String>,

    #[serde(rename="createdTime")]
    created_time: f64,

    #[serde(rename = "avatarUrl")]
    avatar_url: String,

    bio: Option<String>,

    #[serde(rename = "bannerUrl")]
    banner_url: Option<String>,
    website: Option<String>,

    #[serde(rename = "twitterHandle")]
    twitter_handle: Option<String>,

    #[serde(rename = "discordHandle")]
    discord_handle: Option<String>,

    balance: f64,

    #[serde(rename = "totalDeposits")]
    total_deposits: f64,

    #[serde(rename = "totalPnLCached")]
    total_pnl_cached: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MarketList {
    pub markets: Vec<Market>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Market {
    /// from <https://docs.manifold.markets/api#get-v0markets>

    /// Unique identifer for this market
    id: String,

    /// Attributes about the creator
    #[serde(rename = "creatorUsername")]
    creator_username: String,

    /// The name of the creator
    #[serde(rename = "creatorName")]
    creator_name: String,

    #[serde(rename = "creatorAvatarUrl")]
    creator_avatar_url: Option<String>,

    /// Market attributes. All times are in milliseconds since epoch
    /// Min of creator's chosen date, and resolutionTime
    #[serde(rename = "closeTime")]
    close_time: Option<u64>,

    /// milliseconds since epoch
    #[serde(rename="createdTime")]
    created_time: f64,

    /// The question!
    question: String,

    /// Note: This url always points to <https://manifold.markets>, regardless of what instance the api is running on.
    /// This url includes the creator's username, but this doesn't need to be correct when constructing valid URLs.
    ///   i.e. <https://manifold.markets/Austin/test-market> is the same as <https://manifold.markets/foo/test-market>
    url: String,

    #[serde(rename = "outcomeType")]
    /// BINARY, FREE_RESPONSE, MULTIPLE_CHOICE, NUMERIC, or PSEUDO_NUMERIC
    outcome_type: String,

    /// dpm-2 or cpmm-1
    mechanism: String,

    /// current probability of the market
    probability: f64,

    /// For CPMM markets, the number of shares in the liquidity pool. For DPM markets,
    /// the amount of mana invested in each answer.
    pool: HashMap<MarketOutcome, f64>,

    /// CPMM markets only, probability constant in y^p * n^(1-p) = k
    p: Option<f64>,

    /// CPMM markets only, the amount of mana deposited into the liquidity pool
    #[serde(rename = "total_liquidity")]
    total_liquidity: Option<f64>,

    /// PSEUDO_NUMERIC markets only, the current market value, which is mapped from
    /// probability using min, max, and isLogScale.
    value: Option<f64>,

    /// PSEUDO_NUMERIC markets only, the minimum resolvable value
    min: Option<f64>,

    /// PSEUDO_NUMERIC markets only, the maximum resolvable value
    max: Option<f64>,

    /// PSEUDO_NUMERIC markets only, if true `number = (max - min + 1)^probability + minstart - 1`,
    /// otherwise `number = min + (max - min) * probability`
    #[serde(rename = "isLogScale")]
    is_log_scale: Option<bool>,

    volume: f64,
    #[serde(rename = "volume24Hours")]
    volume_24_hours: f64,

    #[serde(rename = "isResolved")]
    is_resolved: bool,

    #[serde(rename = "resolutionTime")]
    resolution_time: Option<f64>,
    resolution: Option<String>,

    /// Used for BINARY markets resolved to MKT
    #[serde(rename = "resolutionProbability")]
    resolution_probability: Option<f64>,

    #[serde(rename = "uniqueBettorCount")]
    unique_bettor_count: u64,

    #[serde(rename = "lastUpdatedTime")]
    last_updated_time: Option<f64>,

    #[serde(rename = "lastBetTime")]
    last_bet_time: Option<f64>,
}
