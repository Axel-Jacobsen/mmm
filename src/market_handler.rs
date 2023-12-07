/// This does a lot of work!
/// The job of this file is to interact w/ the Manifold API,
/// keep track of information that the bots want, make bets
/// that the bots want, and make sure limits (api limits, risk
/// limits) are within bounds.
use std::env;
use std::thread::sleep;
use std::time::Duration;

use serde_json::{self, Value};

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
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new() -> Self {
        let api_key = get_env_key("MANIFOLD_KEY").unwrap();

        Self {
            api_key,
            api_url: String::from("https://api.manifold.markets"),
            api_read_limit_per_s: 100,
            api_write_limit_per_min: 10,
        }
    }

    pub fn get_endpoint(
        &self,
        endpoint: String,
        query_params: &[(&str, &str)],
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let client = reqwest::blocking::Client::new();

        let req = client
            .get(format!("https://manifold.markets/api/v0/{endpoint}"))
            .query(&query_params)
            .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

        req.send()
    }

    pub fn check_alive(&self) -> bool {
        let resp = self.get_endpoint("me".to_string(), &[]).unwrap();

        resp.json::<manifold_types::User>().is_ok()
    }

    pub fn market_search(&self, term: String) -> Vec<manifold_types::LiteMarket> {
        let resp = self
            .get_endpoint(String::from("search-markets"), &[("term", term.as_str())])
            .unwrap();

        match resp.json::<Vec<manifold_types::LiteMarket>>() {
            Ok(markets) => markets,
            Err(e) => {
                let resp = self
                    .get_endpoint(String::from("search-markets"), &[("term", term.as_str())])
                    .unwrap();

                let json_array = serde_json::from_str::<Vec<Value>>(&resp.text().unwrap());

                let mut markets = Vec::new();

                for item in json_array.unwrap() {
                    match serde_json::from_value::<manifold_types::LiteMarket>(item.clone()) {
                        Ok(market) => markets.push(market),
                        Err(_) => {
                            println!("Failed to decode: {:?}", item);
                        }
                    }
                }
                panic!("Failed to decode: {:?}", e);
            }
        }
    }

    pub fn get_bet_stream_for_market_id(&self, market_id: String) {
        let resp = self
            .get_endpoint(format!("bets"), &[("contractId", market_id.as_str())])
            .unwrap();

        let bets = resp.json::<Vec<manifold_types::Bet>>().unwrap();
        for bet in bets {
            assert!(bet.contract_id == market_id);
        }
    }

    pub fn run(&self, endpoints: Vec<String>) {
        loop {
            for endpoint in &endpoints {
                // crummy way to avoid api lims
                sleep(Duration::from_secs(1) / self.api_read_limit_per_s);

                let resp = self
                    .get_endpoint(endpoint.to_string(), &[("limit", "1")])
                    .unwrap();

                if resp.status().is_success() {
                    println!("{}", resp.text().unwrap());
                } else {
                    println!("endpoint {endpoint} failed {:?}", resp);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::market_handler::manifold_types;
    use crate::market_handler::MarketHandler;

    #[test]
    fn build_a_market() {
        let market_handler = MarketHandler::new();
        assert!(market_handler.check_alive());
    }

    #[test]
    fn test_getting_bets() {
        let market_handler = MarketHandler::new();
        let all_markets = market_handler.market_search("".to_string());

        let market = &all_markets[0];
        let market_id = market.id.clone();

        let resp = market_handler
            .get_endpoint(format!("bets"), &[("contractId", market_id.as_str())])
            .unwrap();

        let bets = resp.json::<Vec<manifold_types::Bet>>().unwrap();
        for bet in bets {
            assert!(bet.contract_id == market_id);
        }
    }
}
