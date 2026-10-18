use std::collections::VecDeque;

enum Pairing {
    RoundRobin,
}

pub struct Generator {
    pairing: Pairing,
    pub id: usize,
    pub num_players: usize,
    pub num_openings: usize,
    pub num_games: Option<usize>,
    // Games before `id` that a resumed match still has to play, handed out
    // first
    replay: VecDeque<usize>,
}

pub struct Work {
    pub game_id: usize,
    pub player1: usize,
    pub player2: usize,
    pub fen_idx: usize,
}

impl Generator {
    #[must_use]
    pub fn new(num_players: usize, num_openings: usize, num_games: Option<usize>) -> Self {
        Self {
            pairing: Pairing::RoundRobin,
            id: 0,
            num_players,
            num_openings,
            num_games,
            replay: VecDeque::new(),
        }
    }

    // Continues from `next_id`, after playing `replay`
    #[must_use]
    pub fn resume(mut self, next_id: usize, replay: Vec<usize>) -> Self {
        self.id = next_id;
        self.replay = replay.into();
        self
    }

    #[must_use]
    pub fn next(&mut self) -> Option<Work> {
        // Nothing to play from
        if self.num_openings == 0 || self.num_players < 2 {
            return None;
        }

        if let Some(id) = self.replay.pop_front() {
            return Some(self.work(id));
        }

        // Number of games limit reached
        if let Some(max) = self.num_games
            && self.id >= max
        {
            return None;
        }

        let work = self.work(self.id);
        self.id += 1;

        Some(work)
    }

    // A game's players and opening depend only on its id, so a replayed game
    // is the same game
    fn work(&self, id: usize) -> Work {
        match self.pairing {
            Pairing::RoundRobin => Work {
                game_id: id,
                player1: id % 2,
                player2: 1 - (id % 2),
                fen_idx: (id / 2) % self.num_openings,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_replays_then_continues() {
        let mut generator = Generator::new(2, 3, Some(8)).resume(5, vec![1, 4]);
        let ids: Vec<_> = std::iter::from_fn(|| generator.next())
            .map(|w| (w.game_id, w.player1, w.fen_idx))
            .collect();
        assert_eq!(ids, [(1, 1, 0), (4, 0, 2), (5, 1, 2), (6, 0, 0), (7, 1, 0)]);
    }
}
