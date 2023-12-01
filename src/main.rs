mod market_handler;

fn main() -> Result<(), reqwest::Error> {
    let _market_handler =
        market_handler::MarketHandler::<market_handler::manifold_types::Market>::new();
    Ok(())
}
