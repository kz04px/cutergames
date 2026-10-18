mod connection;
mod http;
mod queue;

use clap::{Arg, ArgAction, ArgMatches, Command};
use protocol::{ClientMessage, JobSpec, ServerMessage, validate_job};
use queue::JobQueue;
use std::fs::read_to_string;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn cli() -> Command {
    Command::new("server")
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(Arg::new("port").long("port").default_value("9080"))
        .arg(
            Arg::new("http-port")
                .long("http-port")
                .default_value("8080"),
        )
        .arg(Arg::new("jobs").long("jobs"))
        .arg(Arg::new("help").long("help").action(ArgAction::SetTrue))
        .subcommand(
            Command::new("submit")
                .about("Submit jobs to an already-running server")
                .disable_help_flag(true)
                .arg(Arg::new("host").long("host").default_value("127.0.0.1"))
                .arg(Arg::new("port").long("port").default_value("9080"))
                .arg(
                    Arg::new("jobs")
                        .long("jobs")
                        .required_unless_present("help"),
                )
                .arg(Arg::new("help").long("help").action(ArgAction::SetTrue)),
        )
}

fn load_jobs(path: &str) -> Result<Vec<JobSpec>, String> {
    let content =
        read_to_string(path).map_err(|err| format!("Failed to read \"{}\": {}", path, err))?;
    let specs: Vec<JobSpec> = serde_json::from_str(&content)
        .map_err(|err| format!("Failed to parse \"{}\": {}", path, err))?;

    for spec in &specs {
        validate_job(spec)?;
    }

    Ok(specs)
}

const HELP: &str = "\
Usage: server [options]
       server submit [options]

Runs a server that stays open and waits for jobs to come in, either
loaded from a file at startup or submitted at any time by a connected
client (see the `submit` subcommand, or send a SubmitJob message
yourself).

Options:
  --port <n>      Port to listen on for workers/submitters. Default 9080.
  --http-port <n> Port to serve a browser status page on. Default 8080.
  --jobs <path>   JSON file of jobs to load at startup, an array of
                  {game, player1, player2, fens, num_games}.
  --help          Print this message.

submit options:
  --host <addr> Server address to connect to. Default 127.0.0.1.
  --port <n>    Server port to connect to. Default 9080.
  --jobs <path> JSON file of jobs to submit, same format as above.";

fn send(stream: &mut TcpStream, msg: &ClientMessage) -> std::io::Result<()> {
    let mut line = serde_json::to_string(msg).expect("ClientMessage always serialises");
    line.push('\n');
    stream.write_all(line.as_bytes())
}

fn recv(reader: &mut BufReader<TcpStream>) -> Result<ServerMessage, String> {
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|err| format!("Failed to read from the server: {}", err))?;
    serde_json::from_str(line.trim_end())
        .map_err(|err| format!("Failed to parse the server's reply: {}", err))
}

// Connects to a running server and submits every job in `--jobs`, printing
// the id it was assigned or why it was rejected.
fn run_submit(matches: &ArgMatches) -> Result<(), String> {
    let host = matches.get_one::<String>("host").expect("has a default");
    let port: u16 = matches
        .get_one::<String>("port")
        .expect("has a default")
        .parse()
        .map_err(|_| "--port must be a number".to_string())?;
    let path = matches.get_one::<String>("jobs").expect("required");

    let specs = load_jobs(path)?;

    let stream = TcpStream::connect((host.as_str(), port))
        .map_err(|err| format!("Failed to connect to {}:{}: {}", host, port, err))?;
    let mut writer = stream
        .try_clone()
        .map_err(|err| format!("Failed to clone the connection: {}", err))?;
    let mut reader = BufReader::new(stream);

    // The server greets every connection with a Welcome first.
    recv(&mut reader)?;

    send(
        &mut writer,
        &ClientMessage::Hello {
            name: "submit".to_string(),
        },
    )
    .map_err(|err| format!("Failed to send to the server: {}", err))?;

    for spec in specs {
        let game = spec.game.clone();
        send(&mut writer, &ClientMessage::SubmitJob(Box::new(spec)))
            .map_err(|err| format!("Failed to send to the server: {}", err))?;

        match recv(&mut reader)? {
            ServerMessage::JobSubmitted(id) => println!("<Job #{}> Submitted ({})", id, game),
            ServerMessage::Error(err) => println!("Rejected ({}): {}", game, err),
            other => println!("Unexpected reply from the server: {:?}", other),
        }
    }

    Ok(())
}

fn run_serve(matches: &ArgMatches) -> Result<(), String> {
    let port: u16 = matches
        .get_one::<String>("port")
        .expect("has a default")
        .parse()
        .map_err(|_| "--port must be a number".to_string())?;
    let http_port: u16 = matches
        .get_one::<String>("http-port")
        .expect("has a default")
        .parse()
        .map_err(|_| "--http-port must be a number".to_string())?;

    let queue = Arc::new(Mutex::new(JobQueue::new()));

    if let Some(path) = matches.get_one::<String>("jobs") {
        let specs = load_jobs(path)?;
        let mut q = queue.lock().expect("JobQueue lock poisoned");
        for spec in specs {
            let id = q.add_job(spec);
            println!("<Job #{}> Loaded", id);
        }
    }

    // Games run on clients, not here, so there's no in-progress local state
    // to protect -- just stop accepting connections and exit. Clients will
    // see their connection drop and can reconnect.
    ctrlc::set_handler(|| {
        println!("\n<Ctrl-C> Shutting down.");
        std::process::exit(0);
    })
    .map_err(|err| format!("Failed to set Ctrl-C handler: {}", err))?;

    // Periodically prints progress. Runs for as long as the server does --
    // jobs can be submitted at any time, so "no work left" is never final.
    {
        let queue = queue.clone();
        thread::spawn(move || {
            let mut was_finished = false;
            loop {
                thread::sleep(Duration::from_secs(10));

                let snapshot = queue.lock().expect("JobQueue lock poisoned").snapshot();
                for job in &snapshot {
                    println!(
                        "<Job #{}> {} vs {} -- {}/{} -- {} - {} - {} ({} no result)",
                        job.id,
                        job.player1,
                        job.player2,
                        job.stats.completed,
                        job.total,
                        job.stats.player1_wins,
                        job.stats.player2_wins,
                        job.stats.draws,
                        job.stats.no_result
                    );
                }

                let now_finished = !snapshot.is_empty() && snapshot.iter().all(|j| j.finished);
                if now_finished && !was_finished {
                    println!("All jobs finished. Waiting for more.");
                }
                was_finished = now_finished;
            }
        });
    }

    // Browser status page.
    {
        let queue = queue.clone();
        thread::spawn(move || {
            if let Err(err) = http::serve(queue, http_port) {
                println!("Status page failed to start: {}", err);
            }
        });
    }

    let listener = TcpListener::bind(("0.0.0.0", port))
        .map_err(|err| format!("Failed to bind to port {}: {}", port, err))?;
    println!("Listening on port {}", port);

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(err) => {
                println!("Failed to accept a connection: {}", err);
                continue;
            }
        };

        let queue = queue.clone();
        thread::spawn(move || connection::handle_client(stream, queue));
    }

    Ok(())
}

fn run() -> Result<(), String> {
    let matches = cli().get_matches();

    if matches.get_flag("help") {
        println!("{}", HELP);
        return Ok(());
    }

    if let Some(sub) = matches.subcommand_matches("submit") {
        if sub.get_flag("help") {
            println!("{}", HELP);
            return Ok(());
        }
        return run_submit(sub);
    }

    run_serve(&matches)
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
