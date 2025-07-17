use super::data::Data;
use super::event::{Event, EventError, EventHandler};
use super::state::{GamePhase, State};
use crate::player::Player;
use serde::{Deserialize, Serialize};

/// `Game` represents a sports game, complete with data like periods, game phase, etc.
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: u32,
    pub shared: Data,
    pub state: State,
}

impl Game {
    pub fn new(id: u32, players: Vec<Player>) -> Self {
        Self {
            id,
            shared: Data {
                periods: vec![],
                players,
                mvp: None,
            },
            state: State::NotStarted(GamePhase::default()),
        }
    }
    /// `on_event` processes the incoming event for the game.
    /// # Errors
    ///
    /// `EventError` will be returned when an invalid `Event` is provided
    /// or the wrong `Event` for the current game state.
    pub fn on_event(self, event: Event) -> Result<Self, EventError> {
        match self.state {
            State::NotStarted(phase) => phase.on_event(event, self.shared),
            State::InProgress(phase) => phase.on_event(event, self.shared),
            State::Paused(phase) => phase.on_event(event, self.shared),
            State::Finished(phase) => phase.on_event(event, self.shared),
            // _ => Err(EventError::Invalid), // unnecessary
        }
        .map_or_else(Err, |v| {
            Ok(Self {
                id: self.id,
                state: v.0,
                shared: v.1,
            })
        })
    }
}
