//! `Repo` contains the main `Repo` trait which describes how different data stores for `game` and `player` can be interacted with.
//! `Repo` also contains the different concrete data store implementations.

use crate::Error;
use crate::Game;
use crate::Player;

/// `Repo` describes the methods required for a Service repository.
pub trait Repo: Send + Sync {
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn count_players(&self) -> Result<usize, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn list_players(&self) -> Result<Vec<Player>, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn create_player(&self, number: u32, name: String) -> Result<Player, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn get_player(&self, player_id: &u32) -> Result<Player, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn update_player(&self, player: Player) -> Result<(), Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn delete_player(&self, player_id: &u32) -> Result<(), Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn count_games(&self) -> Result<usize, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn list_games(&self) -> Result<Vec<Game>, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn create_game(&self, game: Game) -> Result<(), Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn get_game(&self, game_id: &u32) -> Result<Game, Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn update_game(&self, game: Game) -> Result<(), Error>;
    /// # Errors
    ///
    /// `Error` will be returned when a value can't be found or there was an
    /// internal error processing the request.
    fn delete_game(&self, game_id: &u32) -> Result<(), Error>;
}
