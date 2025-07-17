use chrono::Duration;
use rusqlite::Connection;

use super::Repo;
use crate::Error;
use crate::Player;
use crate::game::{Data, Game, State};

use std::sync::{Arc, Mutex, MutexGuard};

// Convert from an owned `Game`, avoiding clone in caller.
impl From<rusqlite::Error> for Error {
    fn from(re: rusqlite::Error) -> Error {
        match re {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
            _ => Self::Internal(re.to_string()),
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
struct GameSqlRow {
    id: u32,
    shared_json: String,
    state_json: String,
}

impl TryFrom<Game> for GameSqlRow {
    type Error = Error;

    fn try_from(game: Game) -> Result<Self, Self::Error> {
        let shared_json =
            serde_json::to_string(&game.shared).map_err(|e| Error::Internal(e.to_string()))?;
        let state_json =
            serde_json::to_string(&game.state).map_err(|e| Error::Internal(e.to_string()))?;

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
        let shared: Data =
            serde_json::from_str(&row.shared_json).map_err(|e| Error::Internal(e.to_string()))?;

        let state: State =
            serde_json::from_str(&row.state_json).map_err(|e| Error::Internal(e.to_string()))?;

        Ok(Game {
            id: row.id,
            shared,
            state,
        })
    }
}

struct PlayerSqlRow {
    id: u32,
    name: String,
    number: u32,
    play_count: u32,
    play_start_time: Option<i64>,
    play_duration: Option<i64>,
}

impl From<PlayerSqlRow> for Player {
    fn from(row: PlayerSqlRow) -> Self {
        let pd = row.play_duration.map_or(Duration::zero(), |d| {
            chrono::TimeDelta::try_milliseconds(d).map_or(Duration::zero(), |d| d)
        });

        Player {
            id: row.id,
            name: row.name,
            number: row.number,
            play_count: row.play_count,
            play_start_time: row
                .play_start_time
                .map_or_else(|| None, |ts| chrono::DateTime::from_timestamp_millis(ts)),
            play_duration: pd,
        }
    }
}

impl From<Player> for PlayerSqlRow {
    fn from(player: Player) -> Self {
        let pst = player.play_start_time.map(|t| t.timestamp_millis());
        PlayerSqlRow {
            id: player.id,
            name: player.name,
            number: player.number,
            play_count: player.play_count,
            play_start_time: pst,
            play_duration: Some(player.play_duration.num_milliseconds()),
        }
    }
}

/// `SqliteRepo` provides a sqlite `Repo` implementation.
impl SqliteRepo {
    fn get_conn(&self) -> Result<MutexGuard<Connection>, Error> {
        self.conn
            .lock()
            .map_err(|_| Error::Internal("Failed to acquire lock on SQL connection".to_string()))
    }

    #[must_use]
    /// `new` constructs a sqlite repo for persisting game and player data. If a file exists at the
    /// provided `path` then it is used, otherwise a new file is created.
    pub fn new(path: Option<String>) -> Result<Self, Error> {
        let conn = path.map_or_else(|| Connection::open_in_memory(), |p| Connection::open(p))?;

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
                number          INTEGER NOT NULL,
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
                let sql_row = PlayerSqlRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    number: row.get(2)?,
                    play_count: row.get(3)?,
                    play_start_time: row.get(4)?,
                    play_duration: row.get(5)?,
                };

                Ok(Player::from(sql_row))
            })
            .map_err(Error::from)?
            .collect::<Result<Vec<Player>, _>>()?;

        Ok(players)
    }

    fn create_player(&self, number: u32, name: String) -> Result<Player, Error> {
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

        let player = stmt
            .query_one((name.clone(), &number), |row| {
                let sql_row = PlayerSqlRow {
                    id: row.get(0)?,
                    name,
                    number,
                    play_count: 0,
                    play_start_time: None,
                    play_duration: None,
                };

                Ok(Player::from(sql_row))
            })
            .map_err(Error::from)?;

        Ok(player)
    }

    fn get_player(&self, player_id: &u32) -> Result<Player, Error> {
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

        let player = stmt
            .query_one([player_id], |row| {
                let sql_row = PlayerSqlRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    number: row.get(2)?,
                    play_count: row.get(3)?,
                    play_start_time: row.get(4)?,
                    play_duration: row.get(5)?,
                };

                Ok(Player::from(sql_row))
            })
            .map_err(Error::from)?;

        Ok(player)
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
                row.number,
                row.play_count,
                row.play_start_time,
                row.play_duration,
                row.id,
            ))
            .map_err(Error::from)?;

        match result {
            0 => Err(Error::NotFound),
            1 => Ok(()),
            count => Err(Error::Internal(format!(
                "unexpected updated count: {count}"
            ))),
        }
    }

    fn delete_player(&self, player_id: &u32) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            DELETE FROM
                player
            WHERE
                id = ?1
            ",
        )?;

        let result = stmt.execute([player_id]).map_err(Error::from)?;

        match result {
            0 => Err(Error::NotFound),
            1 => Ok(()),
            count => Err(Error::Internal(format!(
                "unexpected deleted count: {count}"
            ))),
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
                    id: row.get(0)?,
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
            (&row.id, &row.shared_json, row.state_json),
        )
        .map_err(Error::from)?;

        Ok(())
    }

    fn get_game(&self, game_id: &u32) -> Result<Game, Error> {
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
            .query_one([game_id], |row| {
                Ok(GameSqlRow {
                    id: row.get(0)?,
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
            .execute((row.id, row.shared_json, row.state_json))
            .map_err(Error::from)?;

        match result {
            0 => Err(Error::NotFound),
            1 => Ok(()),
            count => Err(Error::Internal(format!(
                "unexpected updated count: {count}"
            ))),
        }
    }

    fn delete_game(&self, game_id: &u32) -> Result<(), Error> {
        let conn = self.get_conn()?;

        let mut stmt = conn.prepare(
            "
            DELETE FROM
                game
            WHERE
                id = ?1
            ",
        )?;

        let result = stmt.execute([game_id]).map_err(Error::from)?;

        match result {
            0 => Err(Error::NotFound),
            1 => Ok(()),
            count => Err(Error::Internal(format!(
                "unexpected deleted count: {count}"
            ))),
        }
    }
}
