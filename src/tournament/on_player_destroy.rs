use crate::tournament::TournamentStatistics;

pub fn on_player_destroy(tournament_stats: &mut TournamentStatistics, is_verbose: bool) {
    if is_verbose {
        println!("<Event::PlayerDestroy> Destroy player");
    }
    tournament_stats.players_destroyed += 1;
}
