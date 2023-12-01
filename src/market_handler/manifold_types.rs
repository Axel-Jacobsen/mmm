use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct LiteUsers {
    users: Vec<LiteUser>,
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct LiteUser {
    /// from https://docs.manifold.markets/api#get-v0users
    id: String,

    #[serde(rename = "createdTime")]
    created_time: u64,

    name: String,
    username: String,
    url: Option<String>, // not an option in the docs, but it should be?

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
