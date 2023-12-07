mod bots;
mod market_handler;

fn main() {
    let market_handler = market_handler::MarketHandler::new();
    assert!(market_handler.check_alive());

    let markets = market_handler.market_search(
        "(M25000 subsidy!) Will a prompt that enables GPT-4 to solve easy Sudoku puzzles be found? (2023)".to_string());

    assert!(markets.len() == 1);

    market_handler.get_bet_stream_for_market_id(markets[0].id.to_string());
}
