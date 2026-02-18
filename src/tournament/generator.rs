enum Pairing {
    RoundRobin,
}

pub struct Generator {
    asd: Pairing,
    pub id: usize,
    pub num_players: usize,
    pub num_openings: usize,
    pub num_games: Option<usize>,
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
            asd: Pairing::RoundRobin,
            id: 0,
            num_players,
            num_openings,
            num_games,
        }
    }

    #[must_use]
    pub fn next(&mut self) -> Option<Work> {
        // Number of games limit reached
        if let Some(max) = self.num_games
            && self.id >= max
        {
            return None;
        }

        // Create work
        let work = match self.asd {
            Pairing::RoundRobin => Some(Work {
                game_id: self.id,
                player1: self.id % 2,
                player2: 1 - (self.id % 2),
                fen_idx: (self.id / 2) % self.num_openings,
            }),
        };

        // Update state
        self.id += 1;

        work
    }
}
