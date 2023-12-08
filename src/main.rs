mod bots;
mod market_handler;

#[tokio::main]
async fn main() {
    let market_handler = market_handler::MarketHandler::new();
    assert!(market_handler.check_alive().await);

    let maybe_some_market = market_handler.market_search(
        "(M25000 subsidy!) Will a prompt that enables GPT-4 to solve easy Sudoku puzzles be found? (2023)");

    let mut rx = match maybe_some_market.await {
        Ok(Some(m)) => {
            market_handler
                .get_bet_stream_for_market_id(m.id.to_string())
                .await
        }
        Ok(None) => panic!("No markets found"),
        Err(e) => panic!("{:?}", e),
    };

    loop {
        println!("{:?}", rx.recv().await);
    }
}
