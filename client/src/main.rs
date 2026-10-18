mod runner;
mod session;

use clap::{Arg, Command};
use std::net::TcpStream;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

// How long to wait before retrying a connection the server refused or that
// dropped mid-session.
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

fn cli() -> Command {
    Command::new("client")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(Arg::new("host").long("host").default_value("127.0.0.1"))
        .arg(Arg::new("port").long("port").default_value("9080"))
        .arg(Arg::new("name").long("name").default_value("worker"))
        .arg(
            Arg::new("runner")
                .long("runner")
                .default_value("cutergames"),
        )
        .arg(
            Arg::new("help")
                .long("help")
                .action(clap::ArgAction::SetTrue),
        )
}

fn run() -> Result<(), String> {
    let matches = cli().get_matches();

    if matches.get_flag("help") {
        println!(
            "Usage: client [options]\n\n\
             Options:\n  \
             --host <addr>   Server address to connect to. Default 127.0.0.1.\n  \
             --port <n>      Server port to connect to. Default 9080.\n  \
             --name <name>   Name this client identifies itself with. Default \"worker\".\n  \
             --runner <path> Path to the cutergames runner binary to play tasks with.\n                  \
             Default \"cutergames\" (must be on PATH).\n  \
             --help          Print this message."
        );
        return Ok(());
    }

    let host = matches.get_one::<String>("host").expect("has a default");
    let port: u16 = matches
        .get_one::<String>("port")
        .expect("has a default")
        .parse()
        .map_err(|_| "--port must be a number".to_string())?;
    let name = matches.get_one::<String>("name").expect("has a default");
    let runner_path = matches.get_one::<String>("runner").expect("has a default");

    loop {
        match TcpStream::connect((host.as_str(), port)) {
            Ok(stream) => {
                println!("Connected to {}:{}", host, port);
                if let Err(err) = session::run(stream, name, runner_path) {
                    println!("Lost connection to the server: {}", err);
                }
            }
            Err(err) => println!("Failed to connect to {}:{}: {}", host, port, err),
        }

        thread::sleep(RECONNECT_DELAY);
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("Error: {}", msg);
            eprintln!("Try --help for a list of options.");
            ExitCode::FAILURE
        }
    }
}
