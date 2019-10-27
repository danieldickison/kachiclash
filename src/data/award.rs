use rusqlite::types::{FromSql, ValueRef, FromSqlResult, FromSqlError, ToSqlOutput};
use rusqlite::ToSql;

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
