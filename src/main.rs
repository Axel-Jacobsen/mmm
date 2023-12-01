mod market_handler;

fn main() -> Result<(), reqwest::Error> {
    let market_handler = market_handler::MarketHandler::new(vec![String::from("markets")]);
    println!("{:?}", market_handler);
    market_handler.run();
    Ok(())
}
