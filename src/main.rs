use crate::parse::parse_commandline;

mod elo;
mod parse;
mod players;
mod tournament;

fn main() {
    let settings = parse_commandline().unwrap();
    println!("CuterGames {}", env!("CARGO_PKG_VERSION"));
    println!(env!("CARGO_PKG_DESCRIPTION"));
    println!(env!("CARGO_PKG_HOMEPAGE"));

    if settings.verbose {
        println!("{:#?}", settings);
    }

    tournament::run::run(&settings);
}
