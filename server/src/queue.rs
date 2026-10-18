use protocol::{GameOutcome, Job, JobSpec, Task, TaskResult};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};

// Turns a job and a game index into the task a client actually plays.
// Mirrors the runner's own Generator: which spec player is "player1"
// alternates with the game's parity, and the opening cycles through fens
// two games at a time so both colours see the same opening.
fn build_task(job: &Job, game_id: usize) -> Task {
    let (player1, player2) = if game_id.is_multiple_of(2) {
        (job.spec.player1.clone(), job.spec.player2.clone())
    } else {
        (job.spec.player2.clone(), job.spec.player1.clone())
    };
    let fen_idx = (game_id / 2) % job.spec.fens.len();

    Task {
        job_id: job.id,
        game_id,
        game: job.spec.game.clone(),
        player1,
        player2,
        fen: job.spec.fens[fen_idx].clone(),
    }
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
pub struct JobStats {
    pub player1_wins: usize,
    pub player2_wins: usize,
    pub draws: usize,
    pub no_result: usize,
    pub completed: usize,
}

// A point-in-time view of one job's state, e.g. for a status page.
#[derive(Debug, Clone, Serialize)]
pub struct JobSnapshot {
    pub id: u64,
    pub game: String,
    pub player1: String,
    pub player2: String,
    pub total: usize,
    pub finished: bool,
    pub stats: JobStats,
}

struct JobState {
    job: Job,
    // Games not yet handed out for the first time.
    next_game_id: usize,
    // Games handed out but lost to a disconnect, to be retried before any
    // fresh game is handed out.
    requeued: VecDeque<usize>,
    in_flight: usize,
    stats: JobStats,
}

impl JobState {
    fn is_finished(&self) -> bool {
        self.next_game_id >= self.job.spec.num_games
            && self.requeued.is_empty()
            && self.in_flight == 0
    }
}

#[derive(Default)]
pub struct JobQueue {
    // Insertion order, so jobs are drained roughly first-in-first-out.
    order: Vec<u64>,
    jobs: HashMap<u64, JobState>,
    next_id: u64,
}

impl JobQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_job(&mut self, spec: JobSpec) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.order.push(id);
        self.jobs.insert(
            id,
            JobState {
                job: Job { id, spec },
                next_game_id: 0,
                requeued: VecDeque::new(),
                in_flight: 0,
                stats: JobStats::default(),
            },
        );

        id
    }

    // Hands out the next available game across all jobs, preferring a
    // requeued (previously lost) game over a fresh one.
    pub fn next_task(&mut self) -> Option<Task> {
        for &job_id in &self.order {
            let Some(state) = self.jobs.get_mut(&job_id) else {
                continue;
            };

            let game_id = if let Some(game_id) = state.requeued.pop_front() {
                game_id
            } else if state.next_game_id < state.job.spec.num_games {
                let game_id = state.next_game_id;
                state.next_game_id += 1;
                game_id
            } else {
                continue;
            };

            state.in_flight += 1;
            return Some(build_task(&state.job, game_id));
        }

        None
    }

    // Puts a handed-out game back in the queue, e.g. because the client
    // that had it disconnected before reporting a result.
    pub fn requeue(&mut self, job_id: u64, game_id: usize) {
        if let Some(state) = self.jobs.get_mut(&job_id) {
            state.in_flight = state.in_flight.saturating_sub(1);
            state.requeued.push_back(game_id);
        }
    }

    pub fn record_result(&mut self, result: TaskResult) {
        let Some(state) = self.jobs.get_mut(&result.job_id) else {
            return;
        };

        state.in_flight = state.in_flight.saturating_sub(1);
        state.stats.completed += 1;

        let is_first = result.game_id.is_multiple_of(2);
        match result.outcome {
            Some(GameOutcome::P1win) => {
                if is_first {
                    state.stats.player1_wins += 1;
                } else {
                    state.stats.player2_wins += 1;
                }
            }
            Some(GameOutcome::P2win) => {
                if is_first {
                    state.stats.player2_wins += 1;
                } else {
                    state.stats.player1_wins += 1;
                }
            }
            Some(GameOutcome::Draw) => state.stats.draws += 1,
            None => state.stats.no_result += 1,
        }
    }

    #[must_use]
    pub fn stats(&self, job_id: u64) -> Option<JobStats> {
        self.jobs.get(&job_id).map(|state| state.stats)
    }

    #[must_use]
    pub fn is_job_finished(&self, job_id: u64) -> bool {
        self.jobs.get(&job_id).is_some_and(JobState::is_finished)
    }

    // A snapshot of every job's state, in insertion order.
    #[must_use]
    pub fn snapshot(&self) -> Vec<JobSnapshot> {
        self.order
            .iter()
            .filter_map(|&id| {
                let state = self.jobs.get(&id)?;
                Some(JobSnapshot {
                    id,
                    game: state.job.spec.game.clone(),
                    player1: state.job.spec.player1.name.clone(),
                    player2: state.job.spec.player2.name.clone(),
                    total: state.job.spec.num_games,
                    finished: state.is_finished(),
                    stats: state.stats,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::EngineSpec;

    fn spec(num_games: usize) -> JobSpec {
        JobSpec {
            game: "ataxx".to_string(),
            player1: EngineSpec {
                name: "A".to_string(),
                path: "./a".to_string(),
                proto: "ugi".to_string(),
                parameters: String::new(),
                tc: "st=10".to_string(),
                options: Default::default(),
            },
            player2: EngineSpec {
                name: "B".to_string(),
                path: "./b".to_string(),
                proto: "ugi".to_string(),
                parameters: String::new(),
                tc: "st=10".to_string(),
                options: Default::default(),
            },
            fens: vec!["startpos".to_string(), "startpos2".to_string()],
            num_games,
        }
    }

    #[test]
    fn hands_out_every_game_once() {
        let mut queue = JobQueue::new();
        let id = queue.add_job(spec(4));

        let mut game_ids: Vec<usize> = (0..4).map(|_| queue.next_task().unwrap().game_id).collect();
        game_ids.sort_unstable();
        assert_eq!(game_ids, vec![0, 1, 2, 3]);
        assert!(queue.next_task().is_none());
        assert!(!queue.is_job_finished(id));
    }

    #[test]
    fn colours_alternate_by_parity() {
        let mut queue = JobQueue::new();
        queue.add_job(spec(2));

        let first = queue.next_task().unwrap();
        let second = queue.next_task().unwrap();
        assert_eq!(first.player1.name, "A");
        assert_eq!(second.player1.name, "B");
    }

    #[test]
    fn requeue_is_handed_out_before_fresh_games() {
        let mut queue = JobQueue::new();
        let id = queue.add_job(spec(2));

        let lost = queue.next_task().unwrap();
        queue.requeue(id, lost.game_id);

        let retried = queue.next_task().unwrap();
        assert_eq!(retried.game_id, lost.game_id);
    }

    #[test]
    fn finishes_once_every_result_is_in() {
        let mut queue = JobQueue::new();
        let id = queue.add_job(spec(1));

        let task = queue.next_task().unwrap();
        assert!(!queue.is_job_finished(id));

        queue.record_result(TaskResult {
            job_id: id,
            game_id: task.game_id,
            outcome: Some(GameOutcome::Draw),
            ply: 10,
        });

        assert!(queue.is_job_finished(id));
        assert_eq!(queue.stats(id).unwrap().draws, 1);
    }

    #[test]
    fn result_attributed_to_spec_players_not_task_slots() {
        let mut queue = JobQueue::new();
        let id = queue.add_job(spec(2));

        // game_id 0: task.player1 == spec.player1 == "A"
        let first = queue.next_task().unwrap();
        assert_eq!(first.game_id, 0);
        queue.record_result(TaskResult {
            job_id: id,
            game_id: first.game_id,
            outcome: Some(GameOutcome::P1win),
            ply: 5,
        });

        // game_id 1: task.player1 == spec.player2 == "B", so a P1win here
        // is a win for spec.player2 ("B")
        let second = queue.next_task().unwrap();
        assert_eq!(second.game_id, 1);
        queue.record_result(TaskResult {
            job_id: id,
            game_id: second.game_id,
            outcome: Some(GameOutcome::P1win),
            ply: 5,
        });

        let stats = queue.stats(id).unwrap();
        assert_eq!(stats.player1_wins, 1);
        assert_eq!(stats.player2_wins, 1);
    }

    #[test]
    fn snapshot_reflects_progress_and_names() {
        let mut queue = JobQueue::new();
        let id = queue.add_job(spec(1));

        let before = queue.snapshot();
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].id, id);
        assert_eq!(before[0].player1, "A");
        assert_eq!(before[0].player2, "B");
        assert_eq!(before[0].total, 1);
        assert!(!before[0].finished);

        let task = queue.next_task().unwrap();
        queue.record_result(TaskResult {
            job_id: id,
            game_id: task.game_id,
            outcome: Some(GameOutcome::Draw),
            ply: 1,
        });

        let after = queue.snapshot();
        assert!(after[0].finished);
        assert_eq!(after[0].stats.draws, 1);
    }
}
