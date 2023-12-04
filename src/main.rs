mod bots;
mod market_handler;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let market_handler = market_handler::MarketHandler::new();
    let b = bots::ArbitrageBot {};
    let hopefully_one_market = market_handler.market_search(
        "Long shot(ish) bets: How many of these 13 markets will resolve as expected?".to_string(),
    );

    let arb_endpoint_packet = market_handler::EndpointPacket {
        endpoint_url: String::from("bets"),
        endpoint_params: Vec::new(),
        sender: Sender::new(),
    };

    Ok(())
}
