use super::data::{Data, Period};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// `State` represents the `Game` state or 'phase' and is implemented using the "typestate pattern",
/// with compile time safety enforced by GamePhase marker structs.
// A simple enum and some judicious match statements would have been simpler and enough for this
// simple app.
#[derive(Clone, Serialize, Deserialize)]
pub enum State {
    NotStarted(GamePhase<NotStartedState>),
    InProgress(GamePhase<InProgressState>),
    Paused(GamePhase<PausedState>),
    Finished(GamePhase<FinishedState>),
}

pub enum GameState {
    NotStarted,
    InProgress,
    Paused,
    Finished,
}

impl State {
    pub fn kind(&self) -> GameState {
        match self {
            Self::NotStarted(_) => GameState::NotStarted,
            Self::InProgress(_) => GameState::InProgress,
            Self::Paused(_) => GameState::Paused,
            Self::Finished(_) => GameState::Finished,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NotStartedState {}

#[derive(Clone, Serialize, Deserialize)]
pub struct InProgressState {
    pub start_time: DateTime<Utc>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PausedState {
    pub start_time: DateTime<Utc>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct FinishedState {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

/// `GamePhase` is a marker struct representing the phase or state a game is in and enforces
/// compile time safety of phase/state transitions.
#[derive(Clone, Serialize, Deserialize)]
pub struct GamePhase<S> {
    pub state: S,
}

impl Default for GamePhase<NotStartedState> {
    fn default() -> Self {
        Self {
            state: NotStartedState {},
        }
    }
}

impl GamePhase<NotStartedState> {
    pub fn start_game(self, mut shared: Data) -> (GamePhase<InProgressState>, Data) {
        let start_time = Utc::now();
        shared.periods.push(Period::new(start_time));

        let next = GamePhase {
            state: InProgressState { start_time },
        };

        (next, shared)
    }
}

impl GamePhase<InProgressState> {
    pub fn end_period(self, mut shared: Data) -> (GamePhase<PausedState>, Data) {
        let end_time = Utc::now();

        if let Some(mut period) = shared.periods.pop() {
            period.finish(end_time);
            shared.periods.push(period);
        }

        let next = GamePhase {
            state: PausedState {
                start_time: self.state.start_time,
            },
        };

        (next, shared)
    }

    pub fn end_game(self, mut shared: Data) -> (GamePhase<FinishedState>, Data) {
        let end_time = Utc::now();
        if let Some(mut period) = shared.periods.pop() {
            period.finish(end_time);
            shared.periods.push(period);
        }

        let next = GamePhase {
            state: FinishedState {
                start_time: self.state.start_time,
                end_time: Utc::now(),
            },
        };

        (next, shared)
    }
}

impl GamePhase<PausedState> {
    pub fn start_period(self, mut shared: Data) -> (GamePhase<InProgressState>, Data) {
        shared.periods.push(Period::new(Utc::now()));

        let next = GamePhase {
            state: InProgressState {
                start_time: self.state.start_time,
            },
        };

        (next, shared)
    }

    pub fn end_game(self, mut shared: Data) -> (GamePhase<FinishedState>, Data) {
        let end_time = Utc::now();
        if let Some(mut period) = shared.periods.pop() {
            period.finish(end_time);
            shared.periods.push(period);
        }

        let next = GamePhase {
            state: FinishedState {
                start_time: self.state.start_time,
                end_time: Utc::now(),
            },
        };

        (next, shared)
    }
}

impl GamePhase<FinishedState> {}

// Support `next.into()` usage in ./event.rs, unwrapping our phase marker struct into the
// Game struct embedded enum State.
impl<S> GamePhase<S> {
    pub fn into_state(self) -> State
    where
        S: Into<State>,
    {
        self.state.into()
    }
}

impl From<GamePhase<NotStartedState>> for State {
    fn from(phase: GamePhase<NotStartedState>) -> Self {
        Self::NotStarted(phase)
    }
}

impl From<GamePhase<InProgressState>> for State {
    fn from(phase: GamePhase<InProgressState>) -> Self {
        Self::InProgress(phase)
    }
}

impl From<GamePhase<PausedState>> for State {
    fn from(phase: GamePhase<PausedState>) -> Self {
        Self::Paused(phase)
    }
}

impl From<GamePhase<FinishedState>> for State {
    fn from(phase: GamePhase<FinishedState>) -> Self {
        Self::Finished(phase)
    }
}
