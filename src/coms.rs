use std::env;

use log::{debug, error};

use serde_json::Value;

use tokio::time::Duration;

use crate::rate_limiter;

use crate::errors;
use crate::internal_packet as ip;

fn get_api_url() -> String {
    if env::var("MMM_BACKTEST").is_err() {
        "https://api.manifold.markets/v0".to_string()
    } else {
        "http://127.0.0.1:3030/v0".to_string()
    }
}

fn get_env_key(key: &str) -> Result<String, String> {
    match env::var(key) {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(format!("couldn't find Manifold API key: {e}")),
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
        .get(format!("{}/{endpoint}", get_api_url()))
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
    let mut req = client
        .post(format!("{}/{endpoint}", get_api_url()))
        .query(&query_params)
        .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

    if let Some(data) = data {
        req = req.json(&data);
    };

    let resp = req.send().await?;

    if resp.status().is_success() {
        Ok(resp)
    } else {
        error!("api error (bad status code) {resp:?} {query_params:?}");
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
    internal_coms_packet: &ip::InternalPacket,
) -> Result<reqwest::Response, reqwest::Error> {
    match internal_coms_packet.method {
        ip::Method::Get => {
            rate_limited_get_endpoint(
                read_rate_limiter.clone(),
                internal_coms_packet.endpoint.clone(),
                &internal_coms_packet.query_params,
            )
            .await
        }
        ip::Method::Post => {
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
