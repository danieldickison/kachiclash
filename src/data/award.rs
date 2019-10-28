use rusqlite::types::{FromSql, ValueRef, FromSqlResult, FromSqlError, ToSqlOutput};
use rusqlite::{ToSql, Connection};
use super::PlayerId;
use crate::data::BashoId;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum Award {
    EmperorsCup = 1
}

impl Award {
    pub fn emoji(self) -> &'static str {
        match self {
            Award::EmperorsCup => "🏆"
        }
    }

    pub fn bestow(self, db: &mut Connection, basho_id: BashoId, player_id: PlayerId) -> rusqlite::Result<()> {
        db.prepare("
                INSERT INTO award (basho_id, type, player_id)
                VALUES (?, ?, ?)
            ")?
            .execute(params![basho_id, self, player_id])
            .map(|_| ())
    }

    pub fn revoke(self, db: &mut Connection, basho_id: BashoId, player_id: PlayerId) -> rusqlite::Result<()> {
        db.prepare("
                DELETE FROM award
                WHERE basho_id = ? AND type = ? AND player_id = ?
            ")?
            .execute(params![basho_id, self, player_id])
            .and_then(|count| match count {
                1 => Ok(()),
                n => Err(rusqlite::Error::StatementChangedRows(n))
            })
    }
}

impl FromSql for Award {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value
            .as_i64()
            .and_then(|num| {
                match num {
                    1 => Ok(Award::EmperorsCup),
                    _ => Err(FromSqlError::OutOfRange(num)),
                }
            })
    }
}

impl ToSql for Award {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(*self as u8))
    }
}