mod bots;
mod market_handler;

#[tokio::main]
async fn main() {
    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await);

    let maybe_some_market = market_handler.market_search(
        String::from("(M25000 subsidy!) Will a prompt that enables GPT-4 to solve easy Sudoku puzzles be found? (2023)"));

    let mut rx = match maybe_some_market.await {
        Ok(Some(m)) => {
            let gg = market_handler.get_bet_stream_for_market_id(m.id.to_string());
            gg.await
        }
        Ok(None) => panic!("No markets found"),
        Err(e) => panic!("{:?}", e),
    };

    let mut i = 0;
    loop {
        println!("{i} {:?}", rx.recv().await);
        i += 1;
        if i > 10 {
            market_handler.halt();
            break;
        }
    }
}
