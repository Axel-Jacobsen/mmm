use clap::Parser;
use log::{error, info};

use crate::cli::{Args, Commands};

mod bots;
mod cli;
mod coms;
mod errors;
mod manifold_types;
mod market_handler;
mod rate_limiter;

use crate::bots::arb_bot::ArbitrageBot;
use crate::bots::ewma_bot::EWMABot;
use crate::bots::Bot;

async fn run() {
    info!("Starting!");

    let mut market_handler = market_handler::MarketHandler::new();

    assert!(market_handler.check_alive().await, "Manifold API is down");

    let me = market_handler.whoami().await.expect("Failed to get me");

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
                error!("couldn't find arb market {e}");
                return;
            }
        }
    };

    let sudoku_market = {
        let market = market_handler.market_search(
            "Will a prompt that enables GPT-4 to solve easy Sudoku \
            puzzles be found? (2023)"
                .to_string(),
        );

        match market.await {
            Ok(market) => market,
            Err(e) => {
                error!("couldn't find ewma market {e}");
                return;
            }
        }
    };

    info!("Found market {}", arb_market.lite_market.question);
    info!("Found market {}", sudoku_market.lite_market.question);

    let (bot_to_mh_tx, arb_rx_bot) = market_handler
        .internal_coms_init("bawt".to_string())
        .await
        .unwrap();

    let (_, ewma_rx_bot) = market_handler
        .internal_coms_init("ewma_bawt".to_string())
        .await
        .unwrap();

    let mut arb_bot = ArbitrageBot::new(
        "bawt".to_string(),
        arb_market.clone(),
        bot_to_mh_tx.clone(),
        arb_rx_bot,
    );
    let mut ewma_bot = EWMABot::new(
        "ewma_bawt".to_string(),
        bot_to_mh_tx.clone(),
        ewma_rx_bot,
        0.4,
        0.7,
    );

    let arb_rx = market_handler
        .get_bet_stream_for_market_id(arb_market.lite_market.id)
        .await;

    let ewma_rx = market_handler
        .get_bet_stream_for_market_id(sudoku_market.lite_market.id)
        .await;

    arb_bot.run(arb_rx).await;
    ewma_bot.run(ewma_rx).await;
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
        Commands::Positions => {
            let market_handler = market_handler::MarketHandler::new();
            let all_my_bets = market_handler
                .get_all_my_positions()
                .await
                .expect("couldn't get positions");

            if all_my_bets.is_empty() {
                println!("no positions found");
                return;
            }

            let mut active_positions =
                market_handler::MarketHandler::get_active_positions(all_my_bets).await;

            active_positions.sort_unstable_by_key(|position| {
                (position.contract_id.clone(), position.answer_id.clone())
            });

            for position in active_positions {
                println!("{position}");
            }
        }
    }
}
