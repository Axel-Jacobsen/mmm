use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ReqwestResponseParsing {
    ReqwestError(reqwest::Error),
    SerdeError(serde_json::Error),
}

impl fmt::Display for ReqwestResponseParsing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReqwestResponseParsing::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            ReqwestResponseParsing::SerdeError(e) => write!(f, "Serde JSON error: {}", e),
        }
    }
}

impl Error for ReqwestResponseParsing {}

impl From<reqwest::Error> for ReqwestResponseParsing {
    fn from(error: reqwest::Error) -> Self {
        ReqwestResponseParsing::ReqwestError(error)
    }
}

impl From<serde_json::Error> for ReqwestResponseParsing {
    fn from(error: serde_json::Error) -> Self {
        ReqwestResponseParsing::SerdeError(error)
    }
}

async fn response_into<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T, ReqwestResponseParsing> {
    let body = resp.text().await?;
    let parsed = serde_json::from_str::<T>(&body);
    parsed.map_err(ReqwestResponseParsing::from)
}
