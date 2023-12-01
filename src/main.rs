mod market_handler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let market_handler = market_handler::MarketHandler::new(vec![String::from("/v0/bets")]);
    println!("{:?}", market_handler);
    Ok(())
}
