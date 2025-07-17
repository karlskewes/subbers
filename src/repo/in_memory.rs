use super::Repo;
use crate::Error;
use crate::Game;
use crate::Player;

use std::cmp::Reverse;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct InMemoryRepo {
    games: Arc<RwLock<HashMap<u32, Game>>>,     // TODO: Arc<Game>
    players: Arc<RwLock<HashMap<u32, Player>>>, // TODO: Arc<Player>
}

/// `InMemoryRepo` provides an in-memory `Repo` implementation using hash map for storage and
/// a single read-write lock per games and players hash maps. Returned values are clones.
// TODO: Consider returning Arc<Game> or Arc<Player> to reduce copying.
// Overkill for this tiny app, but thread safe and least memory cost.
impl InMemoryRepo {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Repo for InMemoryRepo {
    fn count_players(&self) -> Result<usize, Error> {
        let store = self
            .players
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(store.len())
    }

    fn list_players(&self) -> Result<Vec<Player>, Error> {
        let store = self
            .players
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let mut players: Vec<Player> = store.values().cloned().collect();
        players.sort_by_key(|p| p.name.to_lowercase());

        Ok(players)
    }

    fn create_player(&self, number: u32, name: String) -> Result<Player, Error> {
        let mut store = self
            .players
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let id: u32 = u32::try_from(store.len()).map_err(|e| Error::Internal(e.to_string()))?;

        if store.contains_key(&id) {
            return Err(Error::Conflict);
        }

        let player = Player::new(id, number, name);

        _ = store.insert(player.id, player.clone());

        Ok(player)
    }

    fn get_player(&self, player_id: &u32) -> Result<Player, Error> {
        let store = self
            .players
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;

        store
            .get(player_id)
            .map_or_else(|| Err(Error::NotFound), |p| Ok(p.clone()))
    }

    fn update_player(&self, player: Player) -> Result<(), Error> {
        {
            let mut store = self
                .players
                .write()
                .map_err(|e| Error::Internal(e.to_string()))?;

            if !store.contains_key(&player.number) {
                return Err(Error::NotFound);
            }

            _ = store.insert(player.number, player);
        }

        Ok(())
    }

    fn delete_player(&self, player_id: &u32) -> Result<(), Error> {
        let mut store = self
            .players
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        store
            .remove(player_id)
            .map_or_else(|| Err(Error::NotFound), |_p| Ok(()))
    }

    fn count_games(&self) -> Result<usize, Error> {
        let store = self
            .games
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;

        Ok(store.len())
    }

    fn list_games(&self) -> Result<Vec<Game>, Error> {
        let store = self
            .games
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;

        let mut games: Vec<Game> = store.values().cloned().collect();
        games.sort_by_key(|g| Reverse(g.id)); // Descending (newest).
        Ok(games)
    }

    fn create_game(&self, game: Game) -> Result<(), Error> {
        {
            let mut store = self
                .games
                .write()
                .map_err(|e| Error::Internal(e.to_string()))?;

            if store.contains_key(&game.id) {
                return Err(Error::Conflict);
            }

            _ = store.insert(game.id, game);
        }

        Ok(())
    }

    fn get_game(&self, game_id: &u32) -> Result<Game, Error> {
        let store = self
            .games
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?;

        store
            .get(game_id)
            .map_or_else(|| Err(Error::NotFound), |g| Ok(g.clone()))
    }

    fn update_game(&self, game: Game) -> Result<(), Error> {
        {
            let mut store = self
                .games
                .write()
                .map_err(|e| Error::Internal(e.to_string()))?;

            if !store.contains_key(&game.id) {
                return Err(Error::NotFound);
            }

            _ = store.insert(game.id, game);
        }

        Ok(())
    }

    fn delete_game(&self, game_id: &u32) -> Result<(), Error> {
        let mut store = self
            .games
            .write()
            .map_err(|e| Error::Internal(e.to_string()))?;

        store
            .remove(game_id)
            .map_or_else(|| Err(Error::NotFound), |_g| Ok(()))
    }
}
