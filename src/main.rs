use log::{error, info};

use crate::bots::{ArbitrageBot, Bot};

mod bots;
mod errors;
mod manifold_types;
mod market_handler;

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Starting!");

    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await, "Manifold API is down");

    let me = market_handler.whoami().await;

    info!("Logged in as {} (balance {})", me.name, me.balance);

    let arb_market = {
        let market = market_handler
            .market_search("Which video game confirmed for released in Q1 2024 will average the highest score on Opencritic.com by 4/1/24?".to_string());

        match market.await {
            Ok(market) => market,
            Err(e) => {
                error!("market is err {e}");
                return;
            }
        }
    };

    info!(
        "Found market {}",
        serde_json::to_string_pretty(&arb_market).unwrap()
    );

    let mut bot = ArbitrageBot::new(me.clone(), arb_market.clone());

    let rx = market_handler
        .get_bet_stream_for_market_id(arb_market.lite_market.id)
        .await;

    bot.run(rx).await;
}
