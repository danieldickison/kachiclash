use std::str::FromStr;
use std::convert::From;
use std::fmt;
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef, FromSql, FromSqlResult, FromSqlError};
use chrono::naive::NaiveDate;
use chrono::Datelike;
use serde::Deserialize;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Deserialize)]
#[serde(from = "String")]
pub struct BashoId {
    pub year: i32,
    pub month: u8,
}

impl fmt::Display for BashoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let date = NaiveDate::from_ymd(self.year, self.month.into(), 1);
        write!(f, "{}", date.format("%B %Y"))
    }
}

impl FromStr for BashoId {
    type Err = chrono::format::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NaiveDate::parse_from_str(s, "%Y%m").map(|date|
            Self {
                year: date.year(),
                month: date.month() as u8,
            }
        )
    }
}

impl From<String> for BashoId {
    fn from(s: String) -> Self {
        s.as_str().parse().unwrap_or_else(|_| Self {
            year: 2019,
            month: 7,
        })
    }
}

impl FromSql for BashoId {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value
            .as_str()
            .and_then(|str|
                str.parse().map_err(|err| FromSqlError::Other(Box::new(err)))
            )
    }
}

impl ToSql for BashoId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let id: u32 = format!("{:04}{:02}", self.year, self.month)
            .parse()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(id))
    }
}
