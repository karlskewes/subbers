use crate::player::Player;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `Period` represents time sections of a `Game`. In football/soccer 'half' might be
/// the official term but here we use `Period` for all.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Period {
    pub start_time: DateTime<Utc>,       // TODO: time.Time{} equivalent?
    pub end_time: Option<DateTime<Utc>>, // TODO: time.Time{} equivalent?
}

/// `Data` represents the unique data per game.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Data {
    pub periods: Vec<Period>,
    pub players: Vec<Player>,
    pub mvp: Option<u32>, // player_id
}

impl Period {
    pub const fn new(start_time: DateTime<Utc>) -> Self {
        Self {
            start_time,
            end_time: None,
        }
    }

    pub const fn finish(&mut self, end_time: DateTime<Utc>) {
        self.end_time = Some(end_time);
    }
}

impl Default for Period {
    fn default() -> Self {
        Self {
            start_time: Utc::now(),
            end_time: None,
        }
    }
}
