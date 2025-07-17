use super::core::Player;
use chrono::{DateTime, Duration, TimeDelta, Utc};

fn duration(delta: TimeDelta) -> String {
    let total_seconds = delta.num_seconds();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{}m {}s", minutes, seconds)
}

/// `PlayerView` is a read-only view of a `Player` with useful data provided as struct fields and via
/// helper methods. It is intended for use in HTML and other presentation layers.
pub struct PlayerView {
    pub id: u32,
    pub number: u32,
    pub name: String,
    pub play_count: u32,
    pub playing: bool,
    pub play_start_time: Option<DateTime<Utc>>,
    pub play_duration: Duration,
}

impl PlayerView {
    pub fn total_duration(&self) -> String {
        duration(self.play_duration)
    }

    pub fn current_period_duration(&self) -> String {
        if let Some(st) = self.play_start_time {
            return duration(Utc::now() - st);
        }

        "-".to_string()
    }
}

/// `into_player_views` is a helper function to simplify converting a vector of Player's into
/// PlayerView's.
pub fn into_player_views(players: Vec<Player>) -> Vec<PlayerView> {
    players.iter().map(PlayerView::from).collect()
}

// Convert from an owned `Player`, avoiding clone in caller.
impl From<Player> for PlayerView {
    fn from(player: Player) -> PlayerView {
        PlayerView::from(&player)
    }
}

impl From<&Player> for PlayerView {
    fn from(player: &Player) -> PlayerView {
        Self {
            id: player.id,
            number: player.number,
            name: player.name.clone(),
            play_count: player.play_count,
            play_duration: player.play_duration,
            play_start_time: player.play_start_time,
            playing: player.is_playing(),
        }
    }
}
