use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, Result, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub started_at: i64,
    pub duration_seconds: u64,
    pub characters: usize,
    pub wpm: u16,
    pub accuracy: u16,
}

pub struct SessionStore {
    connection: Connection,
}

impl SessionStore {
    pub fn open_default() -> Result<Self> {
        let directory = dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .ok_or_else(|| rusqlite::Error::InvalidPath(PathBuf::from("typesafe")))?
            .join("typesafe");
        fs::create_dir_all(&directory)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        Self::open(directory.join("sessions.sqlite3"))
    }

    pub fn open(path: PathBuf) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS typing_sessions (
                id INTEGER PRIMARY KEY,
                started_at INTEGER NOT NULL,
                duration_seconds INTEGER NOT NULL,
                characters INTEGER NOT NULL,
                wpm INTEGER NOT NULL,
                accuracy INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS typing_sessions_started_at ON typing_sessions(started_at);",
        )?;
        Ok(Self { connection })
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        connection.execute_batch(
            "CREATE TABLE typing_sessions (
                id INTEGER PRIMARY KEY,
                started_at INTEGER NOT NULL,
                duration_seconds INTEGER NOT NULL,
                characters INTEGER NOT NULL,
                wpm INTEGER NOT NULL,
                accuracy INTEGER NOT NULL
            );",
        )?;
        Ok(Self { connection })
    }

    pub fn save(&self, session: &Session) -> Result<()> {
        self.connection.execute(
            "INSERT INTO typing_sessions (started_at, duration_seconds, characters, wpm, accuracy)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session.started_at,
                session.duration_seconds,
                session.characters,
                session.wpm,
                session.accuracy
            ],
        )?;
        Ok(())
    }

    pub fn sessions(&self) -> Result<Vec<Session>> {
        let mut statement = self.connection.prepare(
            "SELECT started_at, duration_seconds, characters, wpm, accuracy
             FROM typing_sessions ORDER BY started_at ASC",
        )?;
        statement
            .query_map([], |row| {
                Ok(Session {
                    started_at: row.get(0)?,
                    duration_seconds: row.get(1)?,
                    characters: row.get(2)?,
                    wpm: row.get(3)?,
                    accuracy: row.get(4)?,
                })
            })?
            .collect()
    }
}

pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_and_loads_a_typing_session() {
        let store = SessionStore::in_memory().unwrap();
        let session = Session {
            started_at: 1_700_000_000,
            duration_seconds: 60,
            characters: 250,
            wpm: 50,
            accuracy: 96,
        };
        store.save(&session).unwrap();
        assert_eq!(store.sessions().unwrap(), vec![session]);
    }
}
