use crate::parse::parse;
use std::env;

mod elo;
mod parse;
mod players;
mod sprt;
mod stats;
mod store;
mod tournament;

fn main() -> Result<(), ()> {
    let args = env::args().collect::<Vec<String>>();

    if args.contains(&"--help".to_string()) {
        Ok(())
    } else if args.contains(&"--version".to_string()) {
        println!("CuterGames {}", env!("CARGO_PKG_VERSION"));
        Ok(())
    } else if args.contains(&"--about".to_string()) {
        println!("CuterGames {}", env!("CARGO_PKG_VERSION"));
        println!(env!("CARGO_PKG_DESCRIPTION"));
        println!(env!("CARGO_PKG_HOMEPAGE"));
        Ok(())
    } else if let Ok(settings) = parse(args.into_iter()) {
        if settings.verbose {
            println!("{:#?}", settings);
        }

        tournament::run::run(&settings);
        Ok(())
    } else {
        Err(())
    }
}
