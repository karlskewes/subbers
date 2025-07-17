use super::data::Data;
use super::state::{
    FinishedState, GamePhase, InProgressState, NotStartedState, PausedState, State,
};

/// `Event` represents an event that has happened affecting the game state.
pub enum Event {
    StartGame,
    EndGame,
    StartPeriod,
    EndPeriod,
    // SubPlayer, // SubPlayer(player_id: u3) ?
}

/// `EventError` represents errors that can occur when processing events for a game.
pub enum EventError {
    NoOp,
    Invalid,
}

/// `EventHandler` defines how game events are handled with each game phase (state of the game)
/// performing different transitions based on the event type.
pub trait EventHandler {
    /// `on_event` processes the incoming event for the game.
    /// # Errors
    ///
    /// `EventError` will be returned when an invalid `Event` is provided
    /// or the wrong `Event` for the current game state.
    fn on_event(self, event: Event, shared: Data) -> Result<(State, Data), EventError>;
}

impl EventHandler for GamePhase<NotStartedState> {
    fn on_event(self, event: Event, shared: Data) -> Result<(State, Data), EventError> {
        match event {
            Event::StartGame => {
                let (next, updated) = self.start_game(shared);

                Ok((next.into(), updated))
            }
            _ => Err(EventError::Invalid),
        }
    }
}

impl EventHandler for GamePhase<InProgressState> {
    fn on_event(self, event: Event, shared: Data) -> Result<(State, Data), EventError> {
        match event {
            Event::EndPeriod => {
                let (next, updated) = self.end_period(shared);
                Ok((next.into(), updated))
            }
            Event::EndGame => {
                let (next, updated) = self.end_game(shared);
                Ok((next.into(), updated))
            }
            _ => Err(EventError::Invalid),
        }
    }
}

impl EventHandler for GamePhase<PausedState> {
    fn on_event(self, event: Event, shared: Data) -> Result<(State, Data), EventError> {
        match event {
            Event::StartPeriod => {
                let (next, updated) = self.start_period(shared);
                Ok((next.into(), updated))
            }
            Event::EndGame => {
                let (next, updated) = self.end_game(shared);
                Ok((next.into(), updated))
            }
            _ => Err(EventError::Invalid),
        }
    }
}

impl EventHandler for GamePhase<FinishedState> {
    fn on_event(self, _event: Event, _shared: Data) -> Result<(State, Data), EventError> {
        Err(EventError::Invalid)
    }
}
