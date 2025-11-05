use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// `Player` represents each participant in the team being managed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub number: u32,
    pub play_count: u32,
    pub play_start_time: Option<DateTime<Utc>>,
    pub play_duration: Duration,
}

impl Player {
    pub const fn new(id: u32, number: u32, name: String) -> Self {
        Self {
            id,
            name,
            number,
            play_count: 0,
            play_start_time: None,
            play_duration: Duration::zero(),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.play_start_time.is_some()
    }

    pub fn sub_on(&mut self) {
        // can't sub on someone already on court
        if self.is_playing() {
            return;
        }

        self.play_count += 1;
        self.play_start_time = Some(Utc::now());
    }

    pub fn sub_off(&mut self) {
        // can't sub off someone already on bench
        if !self.is_playing() {
            return;
        }

        if let Some(st) = self.play_start_time {
            self.play_duration += Utc::now() - st;
        }

        self.play_start_time = None;
    }

    pub fn add_stats(&mut self, play_count: u32, play_duration: Duration) {
        self.play_count += play_count;
        self.play_duration += play_duration;
    }

    pub fn reset_stats(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            number: self.number,
            play_count: 0,
            play_duration: Duration::zero(),
            play_start_time: None,
        }
    }
}
