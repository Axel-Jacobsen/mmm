mod bots;
mod market_handler;

fn main() {
    let market_handler = market_handler::MarketHandler::new();
    assert!(market_handler.check_alive());

    let market = market_handler.market_search(
        "(M25000 subsidy!) Will a prompt that enables GPT-4 to solve easy Sudoku puzzles be found? (2023)");

    match market {
        Some(m) => {
            market_handler.get_bet_stream_for_market_id(m.id.to_string());
        }
        None => println!("No markets found"),
    }
}
