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

use tokio::sync::broadcast::{channel, Receiver, Sender};

mod manifold_types;

fn get_env_key(key: &str) -> Result<String, String> {
    match env::var(key) {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(format!("couldn't find Manifold API key: {e}")),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketHandler {
    api_key: String,
    api_url: String,

    api_read_limit_per_s: u32,
    api_write_limit_per_min: u32,

    halt_flag: Arc<AtomicBool>,

    bet_channels: HashMap<String, Sender<manifold_types::Bet>>,
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new() -> Self {
        let api_key = get_env_key("MANIFOLD_KEY").unwrap();
        let halt_flag = Arc::new(AtomicBool::new(false));

        Self {
            api_key,
            api_url: String::from("https://api.manifold.markets"),
            api_read_limit_per_s: 100,
            api_write_limit_per_min: 10,
            halt_flag: halt_flag,
            bet_channels: HashMap::new(),
        }
    }

    pub fn halt(self) {
        self.halt_flag.store(true, Ordering::SeqCst);
    }

    pub async fn get_endpoint(
        &self,
        endpoint: String,
        query_params: &[(&str, &str)],
    ) -> Result<reqwest::Response, reqwest::Error> {
        let client = reqwest::Client::new();

        let req = client
            .get(format!("https://manifold.markets/api/v0/{endpoint}"))
            .query(&query_params)
            .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

        req.send().await
    }

    pub async fn check_alive(&self) -> bool {
        let resp = self.get_endpoint("me".to_string(), &[]).await.unwrap();

        resp.json::<manifold_types::User>().await.is_ok()
    }

    pub async fn market_search(
        &self,
        term: &str,
    ) -> Result<Option<manifold_types::LiteMarket>, String> {
        let resp = self
            .get_endpoint(
                String::from("search-markets"),
                &[("term", term), ("limit", "1")],
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

    pub async fn get_bet_stream_for_market_id(&self, market_id: String) {
        let resp = self
            .get_endpoint("bets".to_string(), &[("contractId", market_id.as_str())])
            .await
            .unwrap();

        let bets = resp.json::<Vec<manifold_types::Bet>>().await.unwrap();
        for bet in bets {
            assert!(bet.contract_id == market_id);
        }
    }

    pub async fn get_bets_stream_for_market_id(
        mut self,
        market_id: String,
    ) -> Receiver<manifold_types::Bet> {
        let rx;
        if self.bet_channels.contains_key(&market_id) {
            rx = self.bet_channels[&market_id].subscribe();
        } else {
            let (tx, rx_inner) = channel::<manifold_types::Bet>(4);
            self.bet_channels.entry(market_id.clone()).or_insert(tx);
            rx = rx_inner;
        }

        // Spawn the task that gets messages from the api and
        // sends them to the channell
        let tx_clone = self.bet_channels[&market_id].clone();
        tokio::spawn(async move {
            while !self.halt_flag.load(Ordering::SeqCst) {
                let params = &[("contractId", market_id.as_str())];
                let resp = self.get_endpoint("bets".to_string(), params);

                for bet in resp
                    .await
                    .unwrap()
                    .json::<Vec<manifold_types::Bet>>()
                    .await
                    .unwrap()
                {
                    tx_clone.send(bet).unwrap();
                }
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
