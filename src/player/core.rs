use std::str::FromStr;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, ErrorKind};

/// PlayerId represents the players id.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct PlayerId(pub u32);

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
enum JerseyNumberRepr {
    DoubleZero,
    Number(u8),
}

/// JerseyNumber represents the players jersey number.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct JerseyNumber(JerseyNumberRepr);

impl std::fmt::Display for JerseyNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            JerseyNumberRepr::DoubleZero => write!(f, "00"),
            JerseyNumberRepr::Number(v) => write!(f, "{v}"),
        }
    }
}

impl JerseyNumber {
    pub fn new(s: &str) -> Result<JerseyNumber, Error> {
        JerseyNumber::from_str(s)
    }
}

impl std::str::FromStr for JerseyNumber {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let repr = if s == "00" {
            Ok(JerseyNumberRepr::DoubleZero)
        } else {
            s.parse::<u8>().map_or_else(
                |e| Err(Error::new(ErrorKind::InvalidInput(e.to_string()))),
                |v| {
                    if v < 100 {
                        Ok(JerseyNumberRepr::Number(v))
                    } else {
                        Err(Error::new(ErrorKind::InvalidInput(
                            "jersey number must be between 0 and 99 inclusive, got: {v}"
                                .to_string(),
                        )))
                    }
                },
            )
        }?;

        Ok(JerseyNumber(repr))
    }
}

/// `Player` represents each participant in the team being managed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub number: JerseyNumber,
    pub play_count: u32,
    pub play_start_time: Option<DateTime<Utc>>,
    pub play_duration: Duration,
}

impl Player {
    pub const fn new(id: PlayerId, number: JerseyNumber, name: String) -> Self {
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
