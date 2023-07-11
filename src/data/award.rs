use super::{BashoId, DataError, PlayerId};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use rusqlite::{Connection, ToSql};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, serde::Serialize)]
pub enum Award {
    EmperorsCup = 1,
}

impl Award {
    pub fn emoji(self) -> &'static str {
        match self {
            Award::EmperorsCup => "🏆",
        }
    }

    pub fn _bestow(
        self,
        db: &mut Connection,
        basho_id: BashoId,
        player_id: PlayerId,
    ) -> Result<(), DataError> {
        db.prepare(
            "
                INSERT INTO award (basho_id, type, player_id)
                VALUES (?, ?, ?)
            ",
        )?
        .execute(params![basho_id, self, player_id])
        .map(|_| ())
        .map_err(|e| e.into())
    }

    pub fn _revoke(
        self,
        db: &mut Connection,
        basho_id: BashoId,
        player_id: PlayerId,
    ) -> Result<(), DataError> {
        db.prepare(
            "
                DELETE FROM award
                WHERE basho_id = ? AND type = ? AND player_id = ?
            ",
        )?
        .execute(params![basho_id, self, player_id])
        .and_then(|count| match count {
            1 => Ok(()),
            n => Err(rusqlite::Error::StatementChangedRows(n)),
        })
        .map_err(|e| e.into())
    }

    pub fn parse_list(opt_string: Option<String>) -> Vec<Self> {
        if let Some(string) = opt_string {
            if string.is_empty() {
                vec![]
            } else {
                string
                    .split(',')
                    .filter_map(|a| match a.parse() {
                        Err(e) => {
                            warn!("failed to parse award type {}: {}", a, e);
                            None
                        }
                        Ok(award) => Some(award),
                    })
                    .collect()
            }
        } else {
            vec![]
        }
    }
}

impl Display for Award {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.emoji())
    }
}

impl FromStr for Award {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "1" => Ok(Award::EmperorsCup),
            _ => Err(format!("unknown award type {}", s)),
        }
    }
}

impl FromSql for Award {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value.as_i64().and_then(|num| match num {
            1 => Ok(Award::EmperorsCup),
            _ => Err(FromSqlError::OutOfRange(num)),
        })
    }
}

impl ToSql for Award {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as u8))
    }
}
