/// This does a lot of work!
/// The job of this file is to interact w/ the Manifold API,
/// keep track of information that the bots want, make bets
/// that the bots want, and make sure limits (api limits, risk
/// limits) are within bounds.
use std::env;

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
    endpoints: Vec<String>,

    api_read_limit_per_s: u32,
    api_write_limit_per_min: u32,
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new(endpoints: Vec<String>) -> Self {
        let api_key = get_env_key("MANIFOLD_KEY").unwrap();

        Self {
            api_key,
            endpoints,
            api_url: String::from("https://api.manifold.markets"),
            api_read_limit_per_s: 100,
            api_write_limit_per_min: 10,
        }
    }

    fn get_endpoint(
        &self,
        endpoint: String,
        query_params: Option<&[(&str, &str)]>,
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let client = reqwest::blocking::Client::new();

        let mut req = client
            .get(format!("https://manifold.markets/api/v0/{}", endpoint))
            .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

        if let Some(p) = query_params {
            req = req.query(&p);
        }

        req.send()
    }

    pub fn check_alive(&self) -> bool {
        let resp = self.get_endpoint(String::from("me"), None).unwrap();

        resp.json::<manifold_types::LiteUser>().is_ok()
    }

    pub fn run(&self) {
        loop {
            for endpoint in &self.endpoints {
                std::thread::sleep(std::time::Duration::from_secs(1) / self.api_read_limit_per_s);

                let resp = self
                    .get_endpoint(endpoint.to_string(), Some(&[("limit", "1")]))
                    .unwrap();

                if resp.status().is_success() {
                    println!("{:?}", resp.text());
                } else {
                    println!("endpoint {endpoint} failed {:?}", resp);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::market_handler::MarketHandler;

    #[test]
    fn build_a_market_0() {
        let market_handler = MarketHandler::new(vec![String::from("/v0/bets")]);
        assert!(market_handler.check_alive());
    }
}
