use log::{info};

mod bots;
mod market_handler;

#[tokio::main]
async fn main() {
    env_logger::init();

    println!("Hello, world!");
    info!("Starting!");

    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await);

    let maybe_some_market = market_handler.market_search(
        String::from("(M25000 subsidy!) Will a prompt that enables GPT-4 to solve easy Sudoku puzzles be found? (2023)"));

    let mut rx = match maybe_some_market.await {
        Ok(Some(_)) => {
            let gg = market_handler.get_bet_stream("all_bets".to_string(), vec![]);
            gg.await
        }
        Ok(None) => panic!("No markets found"),
        Err(e) => panic!("{:?}", e),
    };

    let mut i: u64 = 0;
    loop {
        info!("{i} {:?}", rx.recv().await);
        i += 1;
    }
}
