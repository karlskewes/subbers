use super::Repo;
use crate::{Error, ErrorKind, JerseyNumber};
use crate::{Game, GameId};
use crate::{Player, PlayerId};

use std::cmp::Reverse;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct InMemoryRepo {
    games: Arc<RwLock<HashMap<GameId, Game>>>, // TODO: Arc<Game>
    players: Arc<RwLock<HashMap<PlayerId, Player>>>, // TODO: Arc<Player>
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
    #[tracing::instrument(name = "repo.inmemory.count_players", skip_all)]
    fn count_players(&self) -> Result<usize, Error> {
        let store = self
            .players
            .read()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        Ok(store.len())
    }

    #[tracing::instrument(name = "repo.inmemory.list_players", skip_all)]
    fn list_players(&self) -> Result<Vec<Player>, Error> {
        let store = self
            .players
            .read()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        let mut players: Vec<Player> = store.values().cloned().collect();
        players.sort_by_key(|p| p.name.to_lowercase());

        Ok(players)
    }

    #[tracing::instrument(name = "repo.inmemory.create_player", skip_all)]
    fn create_player(&self, number: JerseyNumber, name: String) -> Result<Player, Error> {
        let mut store = self
            .players
            .write()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        let id = PlayerId(
            u32::try_from(store.len())
                .map_err(|e| Error::new(ErrorKind::Internal).with_source(e))?,
        );

        if store.contains_key(&id) {
            return Err(Error::new(ErrorKind::Conflict));
        }

        let player = Player::new(id, number, name);

        _ = store.insert(player.id, player.clone());

        Ok(player)
    }

    #[tracing::instrument(name = "repo.inmemory.get_player", skip_all)]
    fn get_player(&self, player_id: &PlayerId) -> Result<Player, Error> {
        let store = self
            .players
            .read()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        store
            .get(player_id)
            .map_or_else(|| Err(Error::new(ErrorKind::NotFound)), |p| Ok(p.clone()))
    }

    #[tracing::instrument(name = "repo.inmemory.update_player", skip_all)]
    fn update_player(&self, player: Player) -> Result<(), Error> {
        {
            let mut store = self
                .players
                .write()
                .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

            if !store.contains_key(&player.id) {
                return Err(Error::new(ErrorKind::NotFound));
            }

            _ = store.insert(player.id, player);
        }

        Ok(())
    }

    #[tracing::instrument(name = "repo.inmemory.delete_player", skip_all)]
    fn delete_player(&self, player_id: &PlayerId) -> Result<(), Error> {
        let mut store = self
            .players
            .write()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        store
            .remove(player_id)
            .map_or_else(|| Err(Error::new(ErrorKind::NotFound)), |_p| Ok(()))
    }

    #[tracing::instrument(name = "repo.inmemory.count_games", skip_all)]
    fn count_games(&self) -> Result<usize, Error> {
        let store = self
            .games
            .read()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        Ok(store.len())
    }

    #[tracing::instrument(name = "repo.inmemory.list_games", skip_all)]
    fn list_games(&self) -> Result<Vec<Game>, Error> {
        let store = self
            .games
            .read()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        let mut games: Vec<Game> = store.values().cloned().collect();
        games.sort_by_key(|g| Reverse(g.id.0)); // Descending (newest).
        Ok(games)
    }

    #[tracing::instrument(name = "repo.inmemory.create_game", skip_all)]
    fn create_game(&self, game: Game) -> Result<(), Error> {
        {
            let mut store = self
                .games
                .write()
                .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

            if store.contains_key(&game.id) {
                return Err(Error::new(ErrorKind::Conflict));
            }

            _ = store.insert(game.id, game);
        }

        Ok(())
    }

    #[tracing::instrument(name = "repo.inmemory.get_game", skip_all)]
    fn get_game(&self, game_id: &GameId) -> Result<Game, Error> {
        let store = self
            .games
            .read()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        store
            .get(game_id)
            .map_or_else(|| Err(Error::new(ErrorKind::NotFound)), |g| Ok(g.clone()))
    }

    #[tracing::instrument(name = "repo.inmemory.update_game", skip_all)]
    fn update_game(&self, game: Game) -> Result<(), Error> {
        {
            let mut store = self
                .games
                .write()
                .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

            if !store.contains_key(&game.id) {
                return Err(Error::new(ErrorKind::NotFound));
            }

            _ = store.insert(game.id, game);
        }

        Ok(())
    }

    #[tracing::instrument(name = "repo.inmemory.delete_game", skip_all)]
    fn delete_game(&self, game_id: &GameId) -> Result<(), Error> {
        let mut store = self
            .games
            .write()
            .map_err(|e| Error::new(ErrorKind::Internal).with_source_str(e.to_string()))?;

        store
            .remove(game_id)
            .map_or_else(|| Err(Error::new(ErrorKind::NotFound)), |_g| Ok(()))
    }
}
