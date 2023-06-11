use chrono::naive::NaiveDate;
use chrono::Datelike;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::From;
use std::fmt;
use std::result::Result as StdResult;
use std::str::FromStr;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Hash)]
pub struct BashoId {
    pub year: i32,
    pub month: u8,
}

impl BashoId {
    pub fn id(self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }

    pub fn url_path(self) -> String {
        format!("/basho/{}", self.id())
    }

    pub fn season(self) -> String {
        match self.month {
            1 => "Hatsu".to_string(),
            3 => "Haru".to_string(),
            5 => "Natsu".to_string(),
            7 => "Nagoya".to_string(),
            9 => "Aki".to_string(),
            11 => "Kyushu".to_string(),
            _ => self.month_name(),
        }
    }

    fn month_name(self) -> String {
        let date = NaiveDate::from_ymd_opt(self.year, self.month.into(), 1)
            .expect("invalid basho month date");
        format!("{}", date.format("%B"))
    }

    pub fn next(self) -> BashoId {
        self.incr(1)
    }

    pub fn incr(self, count: isize) -> BashoId {
        self.incr_month(count * 2)
    }

    fn incr_month(self, months: isize) -> BashoId {
        let mut year = self.year;
        let mut month = (self.month as isize) + months;
        while month > 12 {
            year += 1;
            month -= 12;
        }
        while month < 1 {
            year -= 1;
            month += 12;
        }
        BashoId {
            year,
            month: month as u8,
        }
    }
}

impl fmt::Display for BashoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        write!(
            f,
            "{} – {} {:04}",
            self.season(),
            self.month_name(),
            self.year
        )
    }
}

impl FromStr for BashoId {
    type Err = chrono::format::ParseError;
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        let with_day = format!("{}01", s);
        NaiveDate::parse_from_str(&with_day, "%Y%m%d").map(|date| date.into())
    }
}

impl From<NaiveDate> for BashoId {
    fn from(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month() as u8,
        }
    }
}

impl Serialize for BashoId {
    fn serialize<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.id())
    }
}

impl<'de> Deserialize<'de> for BashoId {
    fn deserialize<D>(deserializer: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromSql for BashoId {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value.as_i64().map(|num| Self {
            year: (num / 100) as i32,
            month: (num % 100) as u8,
        })
    }
}

impl ToSql for BashoId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let id: u32 = self
            .id()
            .parse()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(id))
    }
}
