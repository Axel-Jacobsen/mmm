use std::env;

use log::{debug, error};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use tokio::time::Duration;

use crate::rate_limiter;

use crate::errors;

const MANIFOLD_API_URL: &str = "https://api.manifold.markets/v0";

fn get_env_key(key: &str) -> Result<String, String> {
    match env::var(key) {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(format!("couldn't find Manifold API key: {e}")),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Method {
    Get,
    Post,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalPacket {
    pub bot_id: String,
    method: Method,
    endpoint: String,
    query_params: Vec<(String, String)>,
    data: Option<Value>,
    response: Option<String>,
}

impl InternalPacket {
    pub fn new(
        bot_id: String,
        method: Method,
        endpoint: String,
        query_params: Vec<(String, String)>,
        data: Option<Value>,
    ) -> Self {
        Self {
            bot_id,
            method,
            endpoint,
            query_params,
            data,
            response: None,
        }
    }

    pub fn response_from_existing(packet: &InternalPacket, response: String) -> Self {
        Self {
            bot_id: packet.bot_id.clone(),
            method: packet.method.clone(),
            endpoint: packet.endpoint.clone(),
            query_params: packet.query_params.clone(),
            data: packet.data.clone(),
            response: Some(response),
        }
    }
}

pub async fn get_endpoint(
    endpoint: String,
    query_params: &[(String, String)],
) -> Result<reqwest::Response, reqwest::Error> {
    debug!(
        "get endpoint; endpoint '{endpoint}'; query params '{:?}'",
        query_params,
    );

    let client = reqwest::Client::new();

    let req = client
        .get(format!("{MANIFOLD_API_URL}/{endpoint}"))
        .query(&query_params)
        .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

    let resp = req.send().await?;

    if resp.status().is_success() {
        Ok(resp)
    } else {
        error!("api error (bad status code) {resp:?} {query_params:?}");
        Err(resp.error_for_status().unwrap_err())
    }
}

pub async fn post_endpoint(
    endpoint: String,
    query_params: &[(String, String)],
    data: Option<Value>,
) -> Result<reqwest::Response, reqwest::Error> {
    debug!(
        "post endpoint; endpoint '{endpoint}'; query params '{:?}'; data '{:?}'",
        query_params, data
    );

    let client = reqwest::Client::new();
    let req = client
        .post(format!("{MANIFOLD_API_URL}/{endpoint}"))
        .query(&query_params)
        .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

    let data_clone = data.clone();

    let resp = if let Some(data) = data {
        let reqq = req.json(&data);
        reqq.send().await?
    } else {
        req.send().await?
    };

    if resp.status().is_success() {
        Ok(resp)
    } else {
        error!("api error (bad status code) {resp:?} {query_params:?} {data_clone:?}");
        Err(resp.error_for_status().unwrap_err())
    }
}

pub async fn response_into<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, errors::ReqwestResponseParsing> {
    let body = resp.text().await?;
    let from_json = serde_json::from_str::<T>(&body);
    match from_json {
        Ok(t) => Ok(t),
        Err(e) => {
            error!("Couldn't parse response {body}");
            Err(errors::ReqwestResponseParsing::SerdeError(e))
        }
    }
}

pub async fn rate_limited_post_endpoint(
    mut write_rate_limiter: rate_limiter::RateLimiter,
    endpoint: String,
    query_params: &[(String, String)],
    data: Option<Value>,
) -> Result<reqwest::Response, reqwest::Error> {
    if write_rate_limiter.block_for_average_pace_then_commit(Duration::from_secs(60)) {
        post_endpoint(endpoint, query_params, data).await
    } else {
        panic!(
            "rate limiter timed out; this shouldn't be possible, \
            most likely rate limit is set wrong"
        );
    }
}

pub async fn rate_limited_get_endpoint(
    mut read_rate_limiter: rate_limiter::RateLimiter,
    endpoint: String,
    query_params: &[(String, String)],
) -> Result<reqwest::Response, reqwest::Error> {
    if read_rate_limiter.block_for_average_pace_then_commit(Duration::from_secs(1)) {
        get_endpoint(endpoint, query_params).await
    } else {
        panic!(
            "rate limiter timed out; this shouldn't be possible, \
            most likely rate limit is set wrong"
        );
    }
}

pub async fn send_internal_packet(
    read_rate_limiter: &rate_limiter::RateLimiter,
    write_rate_limiter: &rate_limiter::RateLimiter,
    internal_coms_packet: &InternalPacket,
) -> Result<reqwest::Response, reqwest::Error> {
    match internal_coms_packet.method {
        Method::Get => {
            rate_limited_get_endpoint(
                read_rate_limiter.clone(),
                internal_coms_packet.endpoint.clone(),
                &internal_coms_packet.query_params,
            )
            .await
        }
        Method::Post => {
            rate_limited_post_endpoint(
                write_rate_limiter.clone(),
                internal_coms_packet.endpoint.clone(),
                &internal_coms_packet.query_params,
                internal_coms_packet.data.clone(),
            )
            .await
        }
    }
}
