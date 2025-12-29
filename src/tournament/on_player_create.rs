use crate::tournament::TournamentStatistics;

pub fn on_player_create(id: usize, tournament_stats: &mut TournamentStatistics, is_verbose: bool) {
    if is_verbose {
        println!("Create player: {}", id);
    }
    tournament_stats.players_created += 1;
}
