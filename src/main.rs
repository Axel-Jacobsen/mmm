mod market_handler;

fn main() -> Result<(), reqwest::Error> {
    let market_handler = market_handler::MarketHandler::new();
    Ok(())
}
