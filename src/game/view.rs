use super::core::Game;
use super::data::Period;
use super::state::GameState;
use crate::player::{PlayerView, into_player_views};
use chrono::{DateTime, TimeDelta, Utc};

const TIME_FORMAT_DIGITAL: &str = "%H:%M:%S";

fn digital_time(date: Option<DateTime<Utc>>) -> String {
    date.map(|d| format!("{}", d.format(TIME_FORMAT_DIGITAL)))
        .unwrap_or_else(|| "-".to_string())
}

fn duration(delta: TimeDelta) -> String {
    let total_seconds = delta.num_seconds();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;

    format!("{}m {}s", minutes, seconds)
}

/// `GameView` is a read-only view of a `Game` with useful data provided as struct fields and via
/// helper methods. It is intended for use in HTML and other presentation layers.
pub struct GameView {
    pub id: u32,
    // consider Option<string> for easier consumption
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: GameState,
    pub periods: Vec<Period>,
    pub players: Vec<PlayerView>,
    pub mvp: Option<u32>,
}

impl GameView {
    pub fn start_time_as_digital(&self) -> String {
        digital_time(self.start_time)
    }

    pub fn end_time_as_digital(&self) -> String {
        digital_time(self.end_time)
    }

    pub fn total_duration(&self) -> String {
        match (self.start_time, self.end_time) {
            (Some(st), None) => duration(Utc::now() - st),
            (Some(st), Some(et)) => duration(et - st),
            _ => "-".to_string(),
        }
    }

    pub fn current_period_duration(&self) -> String {
        if let Some(p) = self.periods.last()
            && p.end_time.is_none()
        {
            return duration(Utc::now() - p.start_time);
        }

        "-".to_string()
    }
}

/// `into_game_views` is a helper function to simplify converting a vector of Game's into
/// GameView's.
pub fn into_game_views(games: Vec<Game>) -> Vec<GameView> {
    games.iter().map(GameView::from).collect()
}

// Convert from an owned `Game`, avoiding clone in caller.
impl From<Game> for GameView {
    fn from(game: Game) -> GameView {
        GameView::from(&game)
    }
}

// Convert from a borrowed `Game`.
impl From<&Game> for GameView {
    fn from(game: &Game) -> GameView {
        let (start_time, end_time) = match &game.state {
            super::state::State::NotStarted(_) => (None, None),
            super::state::State::InProgress(p) => (Some(p.state.start_time), None),
            super::state::State::Paused(p) => (Some(p.state.start_time), None),
            super::state::State::Finished(p) => (Some(p.state.start_time), Some(p.state.end_time)),
        };

        Self {
            id: game.id,
            start_time,
            end_time,
            periods: game.shared.periods.clone(),
            players: into_player_views(game.shared.players.clone()),
            state: game.state.kind(),
            mvp: game.shared.mvp,
        }
    }
}
