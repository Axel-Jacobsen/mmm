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

// let client = reqwest::blocking::Client::new();

// let req = client
//     .get("https://manifold.markets/api/v0/me")
//     .header("Authorization", market_handler::get_api_key()?);

// println!("REQ {req:?}\n");

// let resp = req.send()?;

// match resp.json::<manifold_types::LiteUser>() {
//     Ok(user) => println!("{user:?}"),
//     Err(e) => {
//         let req2 = client
//             .get("https://manifold.markets/api/v0/me")
//             .header("Authorization", market_handler::get_api_key()?);
//         println!("{e} for text {:#?}", req2.send()?.text())
//     }
// }

#[allow(dead_code)]
#[derive(Debug)]
pub struct MarketHandler {
    api_key: String,
    endpoints: Vec<String>,
    api_url: String,
}

#[allow(dead_code)]
impl MarketHandler {
    pub fn new(endpoints: Vec<String>) -> Self {
        let api_key = get_env_key("MANIFOLD_KEY").unwrap();

        Self {
            api_key,
            endpoints,
            api_url: String::from("https://api.manifold.markets"),
        }
    }

    fn check_alive(&self) -> bool {
        let client = reqwest::blocking::Client::new();

        let req = client
            .get("https://manifold.markets/api/v0/me")
            .header("Authorization", get_env_key("MANIFOLD_KEY").unwrap());

        let resp = req.send().unwrap();

        resp.json::<manifold_types::LiteUser>().is_ok()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn build_a_market_0() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
