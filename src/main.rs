use log::{debug, info, warn};

mod bots;
mod market_handler;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Starting!");

    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await, "Manifold API is down");

    let mut rx = market_handler
        .get_bet_stream("all_bets".to_string(), vec![])
        .await;

    let mut i: u64 = 0;
    loop {
        match rx.recv().await {
            Ok(bet) => {
                debug!("{i} {:?}", bet);
                i += 1;
            }
            Err(e) => {
                warn!("{e}");
            }
        }
    }
}
