use log::{debug, error, info, warn};

use crate::bots::{ArbitrageBot, Bot};

mod bots;
mod manifold_types;
mod market_handler;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Starting!");

    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await, "Manifold API is down");

    let arb_market = {
        let market = market_handler.market_search(
            "Will Republicans win Pennsylvania, Georgia in the 2024 Presidential?".to_string(),
        );

        match market.await {
            Ok(market) => market,
            Err(e) => {
                error!("market is err {e}");
                return;
            }
        }
    };

    info!("Found market {:?}", arb_market);

    let bot = ArbitrageBot::new(arb_market.clone());

    let rx = market_handler
        .get_bet_stream_for_market_id(arb_market.lite_market.id)
        .await;

    bot.run(rx).await;
}
