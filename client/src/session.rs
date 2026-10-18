use crate::runner;
use protocol::{ClientMessage, ServerMessage};
use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

// How long to wait before asking again after being told there's no work.
const NO_WORK_RETRY: Duration = Duration::from_secs(2);

fn send(stream: &mut TcpStream, msg: &ClientMessage) -> io::Result<()> {
    let mut line = serde_json::to_string(msg).expect("ClientMessage always serialises");
    line.push('\n');
    stream.write_all(line.as_bytes())
}

fn recv(reader: &mut BufReader<TcpStream>) -> io::Result<Option<ServerMessage>> {
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(None); // server closed the connection
    }
    serde_json::from_str(line.trim_end())
        .map(Some)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

// Runs one connection to completion: hello, then repeatedly request and
// play tasks until the server closes the connection or asks us to stop.
// Returns once there's nothing more to do here; the caller decides whether
// to reconnect.
pub fn run(mut stream: TcpStream, name: &str, runner_path: &str) -> io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);

    send(
        &mut stream,
        &ClientMessage::Hello {
            name: name.to_string(),
        },
    )?;

    match recv(&mut reader)? {
        Some(ServerMessage::Welcome) => {}
        Some(other) => println!(
            "Expected a welcome from the server, got {:?} instead",
            other
        ),
        None => return Ok(()),
    }

    loop {
        send(&mut stream, &ClientMessage::RequestTask)?;

        let Some(msg) = recv(&mut reader)? else {
            println!("Server closed the connection");
            return Ok(());
        };

        match msg {
            ServerMessage::Task(task) => {
                println!(
                    "<Job #{}> Playing game #{}: {} vs {}",
                    task.job_id, task.game_id, task.player1.name, task.player2.name
                );
                let result = runner::run_task(runner_path, &task);
                send(&mut stream, &ClientMessage::TaskResult(result))?;
            }
            ServerMessage::NoWork => thread::sleep(NO_WORK_RETRY),
            ServerMessage::Shutdown => {
                println!("Server is shutting down");
                return Ok(());
            }
            // Only ever sent in reply to a SubmitJob, which a worker never sends.
            ServerMessage::Welcome | ServerMessage::JobSubmitted(_) | ServerMessage::Error(_) => {}
        }
    }
}
