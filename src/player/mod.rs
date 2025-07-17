//! `player` contains the main `Player` struct, events and other items that can be interacted with to manage a sports game player.

mod core;
mod view;

// re-export some objects to reduce use import stuttering.
pub use core::Player;
pub use view::{PlayerView, into_player_views};
