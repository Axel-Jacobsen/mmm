use clap::Parser;
use log::{error, info};

use crate::bots::{ArbitrageBot, Bot};
use crate::cli::{Args, Commands};

mod bots;
mod cli;
mod errors;
mod manifold_types;
mod market_handler;
mod rate_limiter;
mod utils;

async fn run() {
    info!("Starting!");

    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await, "Manifold API is down");

    let me = market_handler.whoami().await;

    info!("Logged in as {} (balance {})", me.name, me.balance);

    let arb_market = {
        let market = market_handler.market_search(
            "Which video game confirmed for released in Q1 2024 will \
                average the highest score on Opencritic.com by 4/1/24?"
                .to_string(),
        );

        match market.await {
            Ok(market) => market,
            Err(e) => {
                error!("market is err {e}");
                return;
            }
        }
    };

    info!("Found market {}", arb_market.lite_market.question);

    let (bot_to_mh_tx, rx_bot) = market_handler.posty_init("bawt".to_string()).await.unwrap();
    let mut bot = ArbitrageBot::new("bawt".to_string(), arb_market.clone(), bot_to_mh_tx, rx_bot);

    let rx = market_handler
        .get_bet_stream_for_market_id(arb_market.lite_market.id)
        .await;

    bot.run(rx).await;
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();

    match args.command {
        Commands::Run => run().await,
        Commands::Liquidate => {
            let market_handler = market_handler::MarketHandler::new();
            match market_handler.liquidate_all_positions().await {
                Ok(_) => info!("All positions liquidated"),
                Err(e) => error!("{e}"),
            };
        }
    }
}
