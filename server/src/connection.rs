use crate::queue::JobQueue;
use protocol::{ClientMessage, ServerMessage, validate_job};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

fn send(stream: &mut TcpStream, msg: &ServerMessage) -> std::io::Result<()> {
    let mut line = serde_json::to_string(msg).expect("ServerMessage always serialises");
    line.push('\n');
    stream.write_all(line.as_bytes())
}

// Runs for the lifetime of one client connection. Any games it was holding
// when the connection drops are put back in the queue for someone else.
pub fn handle_client(mut stream: TcpStream, queue: Arc<Mutex<JobQueue>>) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());
    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone TcpStream"));
    // (job_id, game_id) pairs handed to this client that haven't been
    // reported back yet.
    let mut holding: Vec<(u64, usize)> = Vec::new();
    let mut name = peer.clone();

    if send(&mut stream, &ServerMessage::Welcome).is_err() {
        return;
    }

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // client disconnected
            Ok(_) => {}
            Err(err) => {
                println!("<Client {}> Read error: {}", name, err);
                break;
            }
        }

        let msg: ClientMessage = match serde_json::from_str(line.trim_end()) {
            Ok(msg) => msg,
            Err(err) => {
                println!("<Client {}> Bad message: {}", name, err);
                continue;
            }
        };

        match msg {
            ClientMessage::Hello { name: client_name } => {
                println!("<Client {}> Connected as \"{}\"", peer, client_name);
                name = client_name;
            }
            ClientMessage::RequestTask => {
                let task = queue.lock().expect("JobQueue lock poisoned").next_task();
                let reply = match &task {
                    Some(task) => {
                        holding.push((task.job_id, task.game_id));
                        ServerMessage::Task(Box::new(task.clone()))
                    }
                    None => ServerMessage::NoWork,
                };
                if send(&mut stream, &reply).is_err() {
                    break;
                }
            }
            ClientMessage::TaskResult(result) => {
                holding.retain(|&(job_id, game_id)| {
                    !(job_id == result.job_id && game_id == result.game_id)
                });
                println!(
                    "<Client {}> Job #{} game #{} finished: {:?} ({} ply)",
                    name, result.job_id, result.game_id, result.outcome, result.ply
                );

                let job_id = result.job_id;
                let mut q = queue.lock().expect("JobQueue lock poisoned");
                q.record_result(result);
                if q.is_job_finished(job_id)
                    && let Some(stats) = q.stats(job_id)
                {
                    println!(
                        "<Job #{}> Finished: {} - {} - {} ({} no result)",
                        job_id,
                        stats.player1_wins,
                        stats.player2_wins,
                        stats.draws,
                        stats.no_result
                    );
                }
            }
            ClientMessage::SubmitJob(spec) => {
                let reply = match validate_job(&spec) {
                    Ok(()) => {
                        let id = queue.lock().expect("JobQueue lock poisoned").add_job(*spec);
                        println!("<Job #{}> Submitted by {}", id, name);
                        ServerMessage::JobSubmitted(id)
                    }
                    Err(err) => {
                        println!("<Client {}> Rejected a job: {}", name, err);
                        ServerMessage::Error(err)
                    }
                };
                if send(&mut stream, &reply).is_err() {
                    break;
                }
            }
        }
    }

    if !holding.is_empty() {
        println!(
            "<Client {}> Disconnected with {} game(s) in flight, requeuing",
            name,
            holding.len()
        );
        let mut queue = queue.lock().expect("JobQueue lock poisoned");
        for (job_id, game_id) in holding {
            queue.requeue(job_id, game_id);
        }
    } else {
        println!("<Client {}> Disconnected", name);
    }
}
