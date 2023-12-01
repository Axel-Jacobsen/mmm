mod market_handler;

fn main() -> Result<(), reqwest::Error> {
    let market_handler = market_handler::MarketHandler::new();
    println!(
        "{:?}",
        market_handler.market_search(
            "(M1000 subsidy) Will GPT-4 solve any freshly-generated Sudoku puzzle? (2023)"
                .to_string()
        )
    );
    Ok(())
}
