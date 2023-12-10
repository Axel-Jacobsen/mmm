use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ReqwestResponseParsing {
    APIGeneric(String),
    ReqwestError(reqwest::Error),
    SerdeError(serde_json::Error),
}

impl fmt::Display for ReqwestResponseParsing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReqwestResponseParsing::APIGeneric(e) => write!(f, "APIGeneric error: {}", e),
            ReqwestResponseParsing::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            ReqwestResponseParsing::SerdeError(e) => write!(f, "Serde JSON error: {}", e),
        }
    }
}

impl Error for ReqwestResponseParsing {}

impl From<String> for ReqwestResponseParsing {
    fn from(error: String) -> Self {
        ReqwestResponseParsing::APIGeneric(error)
    }
}

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
