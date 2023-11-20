use std::env;



fn get_api_key() -> Result<String, env::VarError> {
    match env::var("MANIFOLD_KEY") {
        Ok(key) => Ok(format!("Key {key}")),
        Err(e) => Err(e),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();

    let req = client
        .get("https://manifold.markets/api/v0/me")
        .header("Authorization", get_api_key()?);

    let resp = req.send()?;

    println!("{:#?}", resp);
    Ok(())
}
