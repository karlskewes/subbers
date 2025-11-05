//! `Svc` contains the main `Service` struct, which can be interacted with to manage sports games.

use std::sync::Arc;

// TODO: may move/change re-export.
use super::Error;
use super::Player;
use super::Repo;
use super::{EventError, Game};

/// `Service` provides `Game` and `Player` management services, storing data in its repository.
#[derive(Clone)]
pub struct Service {
    repo: Arc<dyn Repo>,
}

pub fn new(repo: Arc<dyn Repo>) -> Service {
    Service { repo }
}

impl Service {
    pub fn new(repo: Arc<dyn Repo>) -> Self {
        Self { repo }
    }

    pub fn list_players(&self) -> Result<Vec<Player>, Error> {
        self.repo.list_players()
    }

    pub fn create_player(&self, number: u32, name: String) -> Result<Player, Error> {
        let player = self.repo.create_player(number, name)?;
        Ok(player)
    }

    pub fn get_player(&self, player_id: &u32) -> Result<Player, Error> {
        self.repo.get_player(player_id)
    }

    pub fn update_player(&self, player: Player) -> Result<Player, Error> {
        // TODO: add/validate player id, etc
        self.repo.update_player(player.clone()).map(|()| player)
    }

    pub fn delete_player(&self, player_id: &u32) -> Result<(), Error> {
        self.repo.delete_player(player_id)
    }

    pub fn list_games(&self) -> Result<Vec<Game>, Error> {
        self.repo.list_games()
    }

    pub fn create_game(&self) -> Result<Game, Error> {
        let next = self.repo.count_games()? + 1;
        let players = self
            .repo
            .list_players()?
            .into_iter()
            .map(|p| p.reset_stats()) // zero game stats for new game.
            .collect();

        let game = Game::new(next as u32, players);

        self.repo.create_game(game.clone())?;

        Ok(game)
    }

    pub fn get_game(&self, game_id: &u32) -> Result<Game, Error> {
        self.repo.get_game(game_id)
    }

    pub fn start_game(&self, game_id: &u32) -> Result<Game, Error> {
        let game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::StartGame)
            .map_err(|e| match e {
                EventError::NoOp => Error::InvalidInput("game already started".to_string()),
                EventError::Invalid => Error::InvalidInput("no state change".to_string()),
            })?;

        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    pub fn end_game(&self, game_id: &u32) -> Result<Game, Error> {
        let mut game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::EndGame)
            .map_err(|e| match e {
                EventError::NoOp => Error::InvalidInput("game already ended".to_string()),
                EventError::Invalid => Error::InvalidInput("no state change".to_string()),
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

    pub fn start_game_period(&self, game_id: &u32) -> Result<Game, Error> {
        let game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::StartPeriod)
            .map_err(|e| match e {
                EventError::NoOp => Error::InvalidInput("period already started".to_string()),
                EventError::Invalid => Error::InvalidInput("no state change".to_string()),
            })?;

        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    pub fn end_game_period(&self, game_id: &u32) -> Result<Game, Error> {
        let mut game = self
            .repo
            .get_game(game_id)?
            .on_event(crate::Event::EndPeriod)
            .map_err(|e| match e {
                EventError::NoOp => Error::InvalidInput("period already ended".to_string()),
                EventError::Invalid => Error::InvalidInput("no state change".to_string()),
            })?;

        for p in game.shared.players.iter_mut() {
            p.sub_off(); // period finished, everyone should be subbed off.
        }

        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    pub fn sub_player_on(&self, game_id: &u32, player_id: &u32) -> Result<Game, Error> {
        let mut game = self.repo.get_game(game_id)?;
        let player = game
            .shared
            .players
            .iter_mut()
            .find(|p| &p.id == player_id)
            .ok_or(Error::NotFound)?;

        player.sub_on();
        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    pub fn sub_player_off(&self, game_id: &u32, player_id: &u32) -> Result<Game, Error> {
        let mut game = self.repo.get_game(game_id)?;
        let player = game
            .shared
            .players
            .iter_mut()
            .find(|p| &p.id == player_id)
            .ok_or(Error::NotFound)?;

        player.sub_off();
        self.repo.update_game(game.clone())?;

        Ok(game)
    }

    pub fn upsert_mvp(&self, game_id: &u32, player_id: &u32) -> Result<Game, Error> {
        let mut game = self.repo.get_game(game_id)?;
        game.shared
            .players
            .iter()
            .find(|p| &p.id == player_id)
            .ok_or(Error::NotFound)?;

        game.shared.mvp = Some(*player_id);
        self.repo.update_game(game.clone())?;

        Ok(game)
    }
}
