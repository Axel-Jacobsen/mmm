use std::collections::HashMap;
/// This does a lot of work!
/// The job of this file is to interact w/ the Manifold API,
/// keep track of information that the bots want, make bets
/// that the bots want, and make sure limits (api limits, risk
/// limits) are within bounds.
use std::env;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use log::{debug, error, info, log_enabled, Level};
use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::time::{sleep, Duration};

mod errors;
mod manifold_types;

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
    let client = reqwest::Client::new();

    let req = client
        .get(format!("https://manifold.markets/api/v0/{endpoint}"))
        .query(&query_params)
        .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

    req.send().await
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketHandler {
    api_read_limit_per_s: u32,
    api_write_limit_per_min: u32,
    halt_flag: Arc<AtomicBool>,
    bet_channels: HashMap<String, Sender<manifold_types::Bet>>,
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new() -> Self {
        let halt_flag = Arc::new(AtomicBool::new(false));

        Self {
            api_read_limit_per_s: 100,
            api_write_limit_per_min: 10,
            halt_flag,
            bet_channels: HashMap::new(),
        }
    }

    pub fn halt(&self) {
        self.halt_flag.store(true, Ordering::SeqCst);
    }

    pub async fn check_alive(&self) -> bool {
        let resp = get_endpoint("me".to_string(), &[]).await.unwrap();

        resp.json::<manifold_types::User>().await.is_ok()
    }

    pub async fn market_search(
        &self,
        term: String,
    ) -> Result<Option<manifold_types::LiteMarket>, String> {
        let resp = get_endpoint(
            "search-markets".to_string(),
            &[
                ("term".to_string(), term),
                ("limit".to_string(), "1".to_string()),
            ],
        )
        .await
        .unwrap();

        match resp.json::<Vec<manifold_types::LiteMarket>>().await {
            Ok(mut markets) => {
                if markets.len() == 1 {
                    Ok(markets.pop())
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(format!("{e}")),
        }
    }

    pub async fn get_bet_stream_for_market_id(
        &mut self,
        market_id: String,
    ) -> Receiver<manifold_types::Bet> {
        self.get_bet_stream(
            market_id.clone(),
            vec![("contractId".to_string(), market_id)],
        )
        .await
    }

    pub async fn get_bet_stream(
        &mut self,
        stream_key: String,
        query_params: Vec<(String, String)>,
    ) -> Receiver<manifold_types::Bet> {
        info!(
            "Getting bet stream for {stream_key} params {:?}",
            query_params
        );

        let rx = if self.bet_channels.contains_key(&stream_key) {
            self.bet_channels[&stream_key].subscribe()
        } else {
            let (tx, rx) = channel::<manifold_types::Bet>(4);
            self.bet_channels
                .entry(stream_key.to_string())
                .or_insert(tx);
            rx
        };

        let mut base_query = query_params.to_vec();
        base_query.push(("limit".to_string(), "1".to_string()));

        let response = get_endpoint("bets".to_string(), &base_query)
            .await
            .expect("Couldn't get most recent bet from api");

        let mut most_recent_id = response_into::<Vec<manifold_types::Bet>>(response)
            .await
            .expect("Couldn't convert json into Bet")
            .pop()
            .expect("no bets placed yet")
            .id;

        // Spawn the task that gets messages from the api and
        // sends them to the channel
        let tx_clone = self.bet_channels[&stream_key].clone();
        let halt_flag_clone = self.halt_flag.clone();

        tokio::spawn(async move {
            while !halt_flag_clone.load(Ordering::SeqCst) {
                let mut params = query_params.clone();
                params.push(("after".to_string(), most_recent_id.clone()));

                let resp = get_endpoint("bets".to_string(), &params);

                let bets = resp
                    .await
                    .expect("Couldn't get bets from api")
                    .json::<Vec<manifold_types::Bet>>()
                    .await
                    .expect("Couldn't convert json into Bet");

                for bet in bets.iter() {
                    tx_clone.send(bet.clone()).expect("Couldn't send bet");
                }

                if bets.len() > 0 {
                    most_recent_id = bets.last().unwrap().id.clone();
                }

                sleep(Duration::from_secs(1)).await;
            }
        });

        rx
    }
}

#[cfg(test)]
mod tests {
    use crate::market_handler::manifold_types;
    use crate::market_handler::MarketHandler;

    use serde_json::{self, Value};

    #[tokio::test]
    async fn build_a_market_handler() {
        let market_handler = MarketHandler::new();
        assert!(market_handler.check_alive().await);
    }

    #[tokio::test]
    async fn test_parse_markets() {
        let market_handler = MarketHandler::new();
        let all_markets = market_handler
            .get_endpoint("markets".to_string(), &[("limit", "1000")])
            .await
            .unwrap();

        // testing that we can parse markets correctly
        match all_markets.json::<Vec<manifold_types::LiteMarket>>().await {
            Ok(_markets) => (),
            Err(e) => {
                // this code here is purely for debugging, and hopefully is like never called
                let resp = market_handler
                    .get_endpoint("markets".to_string(), &[("limit", "1000")])
                    .await
                    .unwrap();

                let json_array = serde_json::from_str::<Vec<Value>>(&resp.text().await.unwrap());

                let mut markets = Vec::new();

                for item in json_array.unwrap() {
                    match serde_json::from_value::<manifold_types::LiteMarket>(item.clone()) {
                        Ok(market) => markets.push(market),
                        Err(e) => {
                            println!("Failed to decode: {:?} due to {:?}\n", item, e);
                        }
                    }
                }
                panic!("Failed to decode: {:?}", e);
            }
        }
    }
}
