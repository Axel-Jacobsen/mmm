/// This does a lot of work!
/// The job of this file is to interact w/ the Manifold API,
/// keep track of information that the bots want, make bets
/// that the bots want, and make sure limits (api limits, risk
/// limits) are within bounds.
use std::env;
use std::thread::sleep;
use std::time::Duration;

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

        resp.json::<Vec<manifold_types::LiteMarket>>().unwrap()
    }

    pub fn get_bet_stream_for_market_id(
        &self,
        market_id: String,
    ) {

        let resp = self
            .get_endpoint(format!("bets"), &[("contractId", market_id.as_str())])
            .unwrap();

        println!("respP: {:?}", resp.json::<Vec<manifold_types::Bet>>());
    }

    pub fn run(&self, endpoints: Vec<String>) {
        loop {
            for endpoint in &endpoints {
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
    use crate::market_handler::LiteMarketHandler;

    #[test]
    fn build_a_market() {
        let market_handler = MarketHandler::new();
        assert!(market_handler.check_alive());
    }

    #[test]
    fn search_for_market() {
        let market_handler = MarketHandler::new();
        println!(
            "{:?}",
            market_handler.market_search(
                "(M1000 subsidy) Will GPT-4 solve any freshly-generated Sudoku puzzle? (2023)"
                    .to_string()
            )
        );
    }

    #[test]
    fn what_are_groups() {
        let market_handler = MarketHandler::new();
        println!(
            "{:?}",
            market_handler
                .get_endpoint("groups".to_string(), &[])
                .unwrap()
        );
    }
}
