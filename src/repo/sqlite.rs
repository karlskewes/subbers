use chrono::Duration;
use rusqlite::Connection;

use super::Repo;
use crate::game::{Data, Game, GameId, State};
use crate::{Error, ErrorKind};
use crate::{JerseyNumber, Player, PlayerId};

use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

impl From<rusqlite::Error> for Error {
    fn from(re: rusqlite::Error) -> Error {
        match re {
            rusqlite::Error::QueryReturnedNoRows => Self::new(ErrorKind::NotFound),
            _ => Self::new(ErrorKind::Internal).with_source(re),
            // Self::Conflict => write!(f, "resource already exists"),
            // Self::NotFound => write!(f, "resource not found"),
            // Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            // Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

pub struct SqliteRepo {
    conn: Arc<Mutex<Connection>>,
}

// GameSqlRow is a convenience transport struct for holding the Game data going in and out of
// Sqlite and does not enforce that the contained fields are valid Game fields.
#[derive(Debug)]
struct GameSqlRow {
    id: GameId,
    shared_json: String,
    state_json: String,
}

impl TryFrom<Game> for GameSqlRow {
    type Error = Error;

    fn try_from(game: Game) -> Result<Self, Self::Error> {
        let shared_json = serde_json::to_string(&game.shared).map_err(|e| {
            Error::new(ErrorKind::Internal)
                .with_context(format!("game id: {} json: shared", game.id))
                .with_source(e)
        })?;
        let state_json = serde_json::to_string(&game.state).map_err(|e| {
            Error::new(ErrorKind::Internal)
                .with_context(format!("game id: {} json: state", game.id))
                .with_source(e)
        })?;

        Ok(GameSqlRow {
            id: game.id,
            shared_json,
            state_json,
        })
    }
}

impl TryFrom<GameSqlRow> for Game {
    type Error = Error;

    fn try_from(row: GameSqlRow) -> Result<Self, Self::Error> {
        let shared: Data = serde_json::from_str(&row.shared_json).map_err(|e| {
            Error::new(ErrorKind::Internal)
                .with_context(format!("game id: {} json: shared", row.id))
                .with_source(e)
        })?;

        let state: State = serde_json::from_str(&row.state_json).map_err(|e| {
            Error::new(ErrorKind::Internal)
                .with_context(format!("game id: {} json: state", row.id))
                .with_source(e)
        })?;

        Ok(Game {
            id: row.id,
            shared,
            state,
        })
    }
}

#[derive(Debug)]
struct PlayerSqlRow {
    id: PlayerId,
    name: String,
    number: String,
    play_count: u32,
    play_start_time: Option<i64>,
    play_duration: Option<i64>,
}

impl TryFrom<PlayerSqlRow> for Player {
    type Error = Error;

    fn try_from(row: PlayerSqlRow) -> Result<Self, Self::Error> {
        let pd = row.play_duration.map_or(Duration::zero(), |d| {
            chrono::TimeDelta::try_milliseconds(d).unwrap_or(Duration::zero())
        });

        let number = JerseyNumber::from_str(&row.number).map_err(|e| {
            Error::new(ErrorKind::Internal)
                .with_context(format!("player id: {} number: {}", row.id, row.number))
                .with_source(e)
        })?;

        Ok(Player {
            id: row.id,
            name: row.name,
            number,
            play_count: row.play_count,
            play_start_time: row
                .play_start_time
                .map_or_else(|| None, chrono::DateTime::from_timestamp_millis),
            play_duration: pd,
        })
    }
}

impl From<Player> for PlayerSqlRow {
    fn from(player: Player) -> Self {
        let pst = player.play_start_time.map(|t| t.timestamp_millis());
        PlayerSqlRow {
            id: player.id,
            name: player.name,
            number: player.number.to_string(),
            play_count: player.play_count,
            play_start_time: pst,
            play_duration: Some(player.play_duration.num_milliseconds()),
        }
    }
}

/// `SqliteRepo` provides a sqlite `Repo` implementation.
impl SqliteRepo {
    fn get_conn(&self) -> Result<MutexGuard<'_, Connection>, Error> {
        self.conn.lock().map_err(|e| {
            Error::new(ErrorKind::Internal)
                .with_context("Failed to acquire lock on SQL connection".to_string())
                .with_source_str(e.to_string())
        })
    }

    /// `new` constructs a sqlite repo for persisting game and player data. If a file exists at the
    /// provided `path` then it is used, otherwise a new file is created.
    pub fn new(path: Option<std::path::PathBuf>) -> Result<Self, Error> {
        let conn = path.map_or_else(Connection::open_in_memory, Connection::open)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS game (
                id     INTEGER PRIMARY KEY,
                shared TEXT NOT NULL,
                state  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS player (
                id              INTEGER PRIMARY KEY,
                name            TEXT NOT NULL,
                number          TEXT NOT NULL,
                play_count      INTEGER NOT NULL DEFAULT 0,
                play_start_time INTEGER, -- unix timestamp milliseconds
                play_duration   INTEGER NOT NULL DEFAULT 0 -- milliseconds
            );
            ",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

impl Repo for SqliteRepo {
    fn count_players(&self) -> Result<usize, Error> {
        let conn = self.get_conn()?;

        let count = conn
            .query_row("SELECT count(id) from player", [], |row| row.get(0))
            .map_err(Error::from)?;

        Ok(count)
    }

    fn list_players(&self) -> Result<Vec<Player>, Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            SELECT
                id,
                name,
                number,
                play_count,
                play_start_time,
                play_duration
            FROM
                player
            ORDER BY
                lower(name) ASC
        ",
        )?;

        let players = stmt
            .query_map([], |row| {
                Ok(PlayerSqlRow {
                    id: PlayerId(row.get(0)?),
                    name: row.get(1)?,
                    number: row.get(2)?,
                    play_count: row.get(3)?,
                    play_start_time: row.get(4)?,
                    play_duration: row.get(5)?,
                })
            })
            .map_err(Error::from)?
            .map(|row| {
                let sql_row = row.map_err(Error::from)?;
                Player::try_from(sql_row)
            })
            .collect::<Result<Vec<Player>, _>>()?;

        Ok(players)
    }

    fn create_player(&self, number: JerseyNumber, name: String) -> Result<Player, Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            INSERT INTO
                player
                (name, number)
            VALUES
                (?1, ?2)
            RETURNING
                id
            ",
        )?;

        let row = stmt
            .query_one((name.clone(), number.to_string()), |row| {
                Ok(PlayerSqlRow {
                    id: PlayerId(row.get(0)?),
                    name,
                    number: number.to_string(),
                    play_count: 0,
                    play_start_time: None,
                    play_duration: None,
                })
            })
            .map_err(Error::from)?;

        Player::try_from(row)
    }

    fn get_player(&self, player_id: &PlayerId) -> Result<Player, Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            SELECT
                id,
                name,
                number,
                play_count,
                play_start_time,
                play_duration
            FROM
                player
            WHERE
                id = ?1
        ",
        )?;

        let row = stmt
            .query_one([player_id.0], |row| {
                Ok(PlayerSqlRow {
                    id: PlayerId(row.get(0)?),
                    name: row.get(1)?,
                    number: row.get(2)?,
                    play_count: row.get(3)?,
                    play_start_time: row.get(4)?,
                    play_duration: row.get(5)?,
                })
            })
            .map_err(Error::from)?;

        Player::try_from(row)
    }

    fn update_player(&self, player: Player) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            UPDATE
                player
            SET
                name = ?1,
                number = ?2,
                play_count = ?3,
		play_start_time = ?4,
		play_duration = ?5
            WHERE
                id = ?6
            ",
        )?;

        let row = PlayerSqlRow::from(player);

        let result = stmt
            .execute((
                row.name,
                row.number.to_string(),
                row.play_count,
                row.play_start_time,
                row.play_duration,
                row.id.0,
            ))
            .map_err(Error::from)?;

        match result {
            0 => Err(Error::new(ErrorKind::NotFound)),
            1 => Ok(()),
            count => Err(Error::new(ErrorKind::Internal)
                .with_context(format!("unexpected updated count: {count}"))),
        }
    }

    fn delete_player(&self, player_id: &PlayerId) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            DELETE FROM
                player
            WHERE
                id = ?1
            ",
        )?;

        let result = stmt.execute([player_id.0]).map_err(Error::from)?;

        match result {
            0 => Err(Error::new(ErrorKind::NotFound)),
            1 => Ok(()),
            count => Err(Error::new(ErrorKind::Internal)
                .with_context(format!("unexpected deleted count: {count}"))),
        }
    }

    fn count_games(&self) -> Result<usize, Error> {
        let conn = self.get_conn()?;

        let count = conn
            .query_row("SELECT count(id) from game", [], |row| row.get(0))
            .map_err(Error::from)?;

        Ok(count)
    }

    fn list_games(&self) -> Result<Vec<Game>, Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            SELECT
                id,
                shared,
                state
            FROM
                game
            ORDER BY
                id DESC
        ",
        )?;

        let games = stmt
            .query_map([], |row| {
                Ok(GameSqlRow {
                    id: GameId(row.get(0)?),
                    shared_json: row.get(1)?,
                    state_json: row.get(2)?,
                })
            })
            .map_err(Error::from)?
            .map(|row| {
                let sql_row = row.map_err(Error::from)?;
                Game::try_from(sql_row)
            })
            .collect::<Result<Vec<Game>, _>>()?;

        Ok(games)
    }

    fn create_game(&self, game: Game) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let row = GameSqlRow::try_from(game)?;

        conn.execute(
            "
            INSERT INTO
                game
                (id, shared, state)
            VALUES
                (?1, ?2, ?3)
            ",
            (&row.id.0, &row.shared_json, row.state_json),
        )
        .map_err(Error::from)?;

        Ok(())
    }

    fn get_game(&self, game_id: &GameId) -> Result<Game, Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            SELECT
                id,
                shared,
                state
            FROM
                game
            WHERE
                id = ?1
        ",
        )?;

        let row = stmt
            .query_one([game_id.0], |row| {
                Ok(GameSqlRow {
                    id: GameId(row.get(0)?),
                    shared_json: row.get(1)?,
                    state_json: row.get(2)?,
                })
            })
            .map_err(Error::from)?;

        Game::try_from(row)
    }

    fn update_game(&self, game: Game) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            UPDATE
                game
            SET
                shared = ?2,
                state = ?3
            WHERE
                id = ?1
            ",
        )?;

        let row = GameSqlRow::try_from(game)?;

        let result = stmt
            .execute((row.id.0, row.shared_json, row.state_json))
            .map_err(Error::from)?;

        match result {
            0 => Err(Error::new(ErrorKind::NotFound)),
            1 => Ok(()),
            count => Err(Error::new(ErrorKind::Internal)
                .with_context(format!("unexpected updated count: {count}"))),
        }
    }

    fn delete_game(&self, game_id: &GameId) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            DELETE FROM
                game
            WHERE
                id = ?1
            ",
        )?;

        let result = stmt.execute([game_id.0]).map_err(Error::from)?;

        match result {
            0 => Err(Error::new(ErrorKind::NotFound)),
            1 => Ok(()),
            count => Err(Error::new(ErrorKind::Internal)
                .with_context(format!("unexpected deleted count: {count}"))),
        }
    }
}
