/// This does a lot of work!
/// The job of this file is to interact w/ the Manifold API,
/// keep track of information that the bots want, make bets
/// that the bots want, and make sure limits (api limits, risk
/// limits) are within bounds.
use std::env;
use std::marker::PhantomData;
use std::thread::sleep;
use std::time::Duration;

use serde::de::DeserializeOwned;

pub mod manifold_types;

fn get_env_key(key: &str) -> Result<String, String> {
    match env::var(key) {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(format!("couldn't find Manifold API key: {e}")),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ApiEndpoint<'a, T>
where
    T: DeserializeOwned + std::fmt::Debug,
{
    endpoint: String,
    query_params: Vec<(&'a str, &'a str)>,
    response_type: PhantomData<T>,
}

#[allow(dead_code)]
impl<'a, T> ApiEndpoint<'a, T>
where
    T: DeserializeOwned + std::fmt::Debug,
{
    pub fn new(
        endpoint: String,
        query_params: Vec<(&'a str, &'a str)>,
    ) -> Self {
        Self {
            endpoint,
            query_params,
            response_type: PhantomData,
        }
    }

    pub fn hit(&self) -> Result<T, reqwest::Error> {
        let client = reqwest::blocking::Client::new();

        let req = client
            .get(format!("https://manifold.markets/api/v0/{}", self.endpoint))
            .query(&self.query_params)
            .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

        let resp = req.send().unwrap();

        resp.json::<T>()
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketHandler<T> {
    api_key: String,
    api_url: String,
    api_read_limit_per_s: u32,
    api_write_limit_per_min: u32,
    _response_type: PhantomData<T>,
}

#[allow(dead_code)]
impl<T> MarketHandler<T>
where
    T: DeserializeOwned + std::fmt::Debug,
{
    pub fn new() -> Self {
        let api_key = get_env_key("MANIFOLD_KEY").unwrap();

        Self {
            api_key,
            api_url: String::from("https://api.manifold.markets"),
            api_read_limit_per_s: 100,
            api_write_limit_per_min: 10,
            _response_type: PhantomData,
        }
    }

    fn get_endpoint(
        &self,
        endpoint: String,
        query_params: &[(&str, &str)],
    ) -> Result<reqwest::blocking::Response, reqwest::Error> {
        let client = reqwest::blocking::Client::new();

        let req = client
            .get(format!("https://manifold.markets/api/v0/{}", endpoint))
            .query(&query_params)
            .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

        req.send()
    }

    fn read_sleep(&self) {
        sleep(Duration::from_secs(1) / self.api_read_limit_per_s);
    }

    fn write_sleep(&self) {
        sleep(Duration::from_secs(1) / self.api_write_limit_per_min);
    }

    pub fn check_alive(&self) -> bool {
        let resp = self.get_endpoint(String::from("me"), &[]).unwrap();

        resp.json::<manifold_types::User>().is_ok()
    }

    pub fn market_search(&self, term: String) -> Vec<manifold_types::Market> {
        let resp = self
            .get_endpoint(String::from("search-markets"), &[("term", term.as_str())])
            .unwrap();

        resp.json::<Vec<manifold_types::Market>>().unwrap()
    }

    pub fn run(&self, endpoints: Vec<ApiEndpoint<T>>) {
        loop {
            for endpoint in &endpoints {
                self.read_sleep();

                match endpoint.hit() {
                    Ok(resp) => println!("{:?}", resp),
                    Err(e) => println!("endpoint {:?} failed {:?}", endpoint, e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::market_handler::MarketHandler;

    #[test]
    fn build_a_market() {
        let market_handler = MarketHandler::new();
        assert!(market_handler.check_alive());
    }
}
