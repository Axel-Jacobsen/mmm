use std::env;

mod manifold_types;

fn get_api_key() -> Result<String, String> {
    match env::var("MANIFOLD_KEY") {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(format!("couldn't find Manifold API key: {e}")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();

    let req = client
        .get("https://manifold.markets/api/v0/me")
        .header("Authorization", get_api_key()?);

    println!("REQ {req:?}\n");

    let resp = req.send()?;

    match resp.json::<manifold_types::LiteUser>() {
        Ok(user) => println!("{user:?}"),
        Err(e) => {
            let req2 = client
                .get("https://manifold.markets/api/v0/me")
                .header("Authorization", get_api_key()?);
            println!("{e} for text {:#?}", req2.send()?.text())
        }
    }

    Ok(())
}
