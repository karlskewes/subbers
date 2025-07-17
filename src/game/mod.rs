//! `game` contains the main `Game` struct, events and other items that can be interacted with to manage a sports game.

mod core;
mod data;
mod event;
mod state;
mod view;

// re-export some objects to reduce use import stuttering.
pub use core::Game;
pub use data::Data;
pub use event::{Event, EventError};
pub use state::GameState;
pub use state::State;
pub use view::{GameView, into_game_views};
