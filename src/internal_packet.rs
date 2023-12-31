use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Method {
    Get,
    Post,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InternalPacket {
    pub bot_id: String,
    pub method: Method,
    pub endpoint: String,
    pub query_params: Vec<(String, String)>,
    pub data: Option<Value>,
    pub response: Option<String>,
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
