//! `Svc` contains the main `Service` struct, which can be interacted with to manage sports games.

use super::Repo;
use super::{Error, ErrorKind};
use super::{EventError, Game, GameId};
use super::{JerseyNumber, Player, PlayerId};
use std::sync::Arc;

/// `Service` provides `Game` and `Player` management services, storing data in its repository.
#[derive(Clone)]
pub struct Service {
    repo: Arc<dyn Repo>,
}

impl Service {
    #[tracing::instrument(name = "svc.new", skip_all)]
    pub fn new(repo: Arc<dyn Repo>) -> Self {
        Self { repo }
    }

    #[tracing::instrument(name = "svc.list_players", skip(self))]
    pub fn list_players(&self) -> Result<Vec<Player>, Error> {
        self.repo.list_players()
    }

    #[tracing::instrument(name = "svc.create_player", skip(self))]
    pub fn create_player(&self, number: JerseyNumber, name: String) -> Result<Player, Error> {
        let player = self.repo.create_player(number, name)?;
        Ok(player)
    }

    #[tracing::instrument(name = "svc.get_player", skip(self))]
    pub fn get_player(&self, player_id: &PlayerId) -> Result<Player, Error> {
        self.repo.get_player(player_id)
    }

    #[tracing::instrument(name = "svc.update_player", skip(self))]
    pub fn update_player(&self, player: Player) -> Result<Player, Error> {
        // TODO: add/validate player id, etc
        self.repo.update_player(player.clone()).map(|()| player)
    }

    #[tracing::instrument(name = "svc.delete_player", skip(self))]
    pub fn delete_player(&self, player_id: &PlayerId) -> Result<(), Error> {
        self.repo.delete_player(player_id)
    }

    #[tracing::instrument(name = "svc.list_games", skip(self))]
    pub fn list_games(&self) -> Result<Vec<Game>, Error> {
        self.repo.list_games()
    }

    #[tracing::instrument(name = "svc.create_game", skip(self))]
    pub fn create_game(&self) -> Result<Game, Error> {
        let next = self.repo.count_games()? + 1;
        let players = self
            .repo
            .list_players()?
            .into_iter()
            .map(|p| p.reset_stats()) // zero game stats for new game.
            .collect();

        let game = Game::new(GameId(next as u32), players);

        self.repo.create_game(game.clone())?;

        Ok(game)
    }

    #[tracing::instrument(name = "svc.get_game", skip(self))]
    pub fn get_game(&self, game_id: &GameId) -> Result<Game, Error> {
        self.repo.get_game(game_id)
    }

    #[tracing::instrument(name = "svc.start_game", skip(self))]
    pub fn start_game(&self, game_id: &GameId) -> Result<Game, Error> {
        let game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::StartGame)
            .map_err(|e| match e {
                EventError::NoOp => {
                    Error::new(ErrorKind::InvalidInput("game already started".to_string()))
                }
                EventError::Invalid => {
                    Error::new(ErrorKind::InvalidInput("no state change".to_string()))
                }
            })?;

        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    #[tracing::instrument(name = "svc.end_game", skip(self))]
    pub fn end_game(&self, game_id: &GameId) -> Result<Game, Error> {
        let mut game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::EndGame)
            .map_err(|e| match e {
                EventError::NoOp => {
                    Error::new(ErrorKind::InvalidInput("game already ended".to_string()))
                }
                EventError::Invalid => {
                    Error::new(ErrorKind::InvalidInput("no state change".to_string()))
                }
            })?;

        for p in game.shared.players.iter_mut() {
            p.sub_off(); // game finished, everyone should be subbed off.
        }

        self.repo.update_game(game.clone())?;

        for p in &game.shared.players {
            // N(game players) DB calls. `WHERE id IN (...)` optimization possible.
            if let Ok(mut ep) = self.repo.get_player(&p.id) {
                ep.add_stats(p.play_count, p.play_duration);
                self.repo.update_player(ep)?;
            }
        }

        Ok(game)
    }

    #[tracing::instrument(name = "svc.start_game_period", skip(self))]
    pub fn start_game_period(&self, game_id: &GameId) -> Result<Game, Error> {
        let game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::StartPeriod)
            .map_err(|e| match e {
                EventError::NoOp => Error::new(ErrorKind::InvalidInput(
                    "period already started".to_string(),
                )),
                EventError::Invalid => {
                    Error::new(ErrorKind::InvalidInput("no state change".to_string()))
                }
            })?;

        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    #[tracing::instrument(name = "svc.end_game_period", skip(self))]
    pub fn end_game_period(&self, game_id: &GameId) -> Result<Game, Error> {
        let mut game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::EndPeriod)
            .map_err(|e| match e {
                EventError::NoOp => {
                    Error::new(ErrorKind::InvalidInput("period already ended".to_string()))
                }
                EventError::Invalid => {
                    Error::new(ErrorKind::InvalidInput("no state change".to_string()))
                }
            })?;

        for p in game.shared.players.iter_mut() {
            p.sub_off(); // period finished, everyone should be subbed off.
        }

        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    #[tracing::instrument(name = "svc.sub_player_on", skip(self))]
    pub fn sub_player_on(&self, game_id: &GameId, player_id: &PlayerId) -> Result<Game, Error> {
        let mut game = self.repo.get_game(game_id)?;
        let player = game
            .shared
            .players
            .iter_mut()
            .find(|p| &p.id == player_id)
            .ok_or(Error::new(ErrorKind::NotFound))?;

        player.sub_on();
        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    #[tracing::instrument(name = "svc.sub_player_off", skip(self))]
    pub fn sub_player_off(&self, game_id: &GameId, player_id: &PlayerId) -> Result<Game, Error> {
        let mut game = self.repo.get_game(game_id)?;
        let player = game
            .shared
            .players
            .iter_mut()
            .find(|p| &p.id == player_id)
            .ok_or(Error::new(ErrorKind::NotFound))?;

        player.sub_off();
        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    #[tracing::instrument(name = "svc.upsert_mvp", skip(self))]
    pub fn upsert_mvp(&self, game_id: &GameId, player_id: &PlayerId) -> Result<Game, Error> {
        let mut game = self.repo.get_game(game_id)?;
        game.shared
            .players
            .iter()
            .find(|p| &p.id == player_id)
            .ok_or(Error::new(ErrorKind::NotFound))?;

        game.shared.mvp = Some(*player_id);
        self.repo.update_game(game.clone())?;

        Ok(game)
    }
}
