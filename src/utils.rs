use std::env;

use log::{debug, error};
use serde_json::Value;

use crate::errors;

fn get_env_key(key: &str) -> Result<String, String> {
    match env::var(key) {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(format!("couldn't find Manifold API key: {e}")),
    }
}

async fn get_endpoint(
    endpoint: String,
    query_params: &[(String, String)],
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();

    debug!("endpoint '{endpoint}'");
    debug!("query params '{query_params:?}'");

    let req = client
        .get(format!("https://manifold.markets/api/v0/{endpoint}"))
        .query(&query_params)
        .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

    let resp = req.send().await?;
    if resp.status().is_success() {
        Ok(resp)
    } else {
        error!("api error (bad status code) {resp:?}");
        Err(resp.error_for_status().unwrap_err())
    }
}

async fn post_endpoint(
    endpoint: String,
    query_params: &[(String, String)],
    data: Option<Value>,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    let req = client
        .post(format!("https://manifold.markets/api/v0/{endpoint}"))
        .query(&query_params)
        .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

    let resp = if let Some(data) = data {
        req.json(&data).send().await?
    } else {
        req.send().await?
    };

    if resp.status().is_success() {
        Ok(resp)
    } else {
        error!("api error (bad status code) {resp:?}");
        Err(resp.error_for_status().unwrap_err())
    }
}

async fn response_into<T: serde::de::DeserializeOwned>(
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
