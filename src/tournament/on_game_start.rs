pub fn on_game_start(id: usize, fen: &str, is_verbose: bool) {
    if is_verbose {
        println!("<Event::GameStart> Start game {} fen {}", id, fen);
    }
}
