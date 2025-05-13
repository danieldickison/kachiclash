use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Utc, Weekday};
use juniper::GraphQLScalar;
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::From;
use std::fmt;
use std::ops::{Deref, Range};
use std::result::Result as StdResult;
use std::str::FromStr;
use std::sync::LazyLock;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Hash, GraphQLScalar)]
#[graphql(with = impl_graphql_scalar)]
pub struct BashoId {
    pub year: u16,
    pub month: u8,
}

static JST: LazyLock<FixedOffset> = LazyLock::new(|| FixedOffset::east_opt(9 * 60 * 60).unwrap());

pub const N_BASHO_FOR_BANZUKE: usize = 6;

impl BashoId {
    pub fn id(self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }

    pub fn url_path(self) -> String {
        format!("/basho/{}", self.id())
    }

    pub fn expected_start_date(self) -> DateTime<Utc> {
        NaiveDate::from_weekday_of_month_opt(self.year as i32, self.month as u32, Weekday::Sun, 2)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap()
            .and_local_timezone(*JST)
            .unwrap()
            .with_timezone(&Utc)
    }

    pub fn expected_venue(self) -> String {
        match self.month {
            1 | 5 | 9 => "Tokyo",
            3 => "Osaka",
            7 => "Nagoya",
            11 => "Fukuoka",
            _ => "?",
        }
        .to_string()
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
        let date = NaiveDate::from_ymd_opt(self.year.into(), self.month.into(), 1)
            .expect("invalid basho month date");
        format!("{}", date.format("%B"))
    }

    pub fn next(self) -> BashoId {
        self.incr(1)
    }

    /// Returns the range of basho that contribute to a player's ranking for this basho. This is currently defined as the six basho preceding this one.
    pub fn range_for_banzuke(self) -> BashoRange {
        BashoRange(self.incr(-(N_BASHO_FOR_BANZUKE as isize))..self)
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

mod impl_graphql_scalar {
    use super::BashoId;
    use juniper::{
        InputValue, ParseScalarResult, ParseScalarValue, ScalarToken, ScalarValue, Value,
    };

    pub(super) fn from_input<S: ScalarValue>(v: &InputValue<S>) -> Result<BashoId, String> {
        match v {
            InputValue::Scalar(s) => match s.as_int() {
                Some(i) => Ok((i as i64).into()),
                None => Err(format!("Expected Int value for BashoId but got {v}")),
            },
            _ => Err(format!("Expected Int value for BashoId but got {v}")),
        }
    }

    pub(super) fn to_output<S: ScalarValue>(v: &BashoId) -> Value<S> {
        Value::scalar(v.id().parse::<i32>().unwrap())
    }

    pub(super) fn parse_token<S: ScalarValue>(t: ScalarToken<'_>) -> ParseScalarResult<S> {
        <i32 as ParseScalarValue<S>>::from_str(t)
    }
}

impl fmt::Display for BashoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        if f.alternate() {
            write!(f, "{} {:04}", self.month_name(), self.year)
        } else {
            write!(
                f,
                "{} Basho {} {:04}",
                self.season(),
                self.month_name(),
                self.year
            )
        }
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
            year: date.year() as u16,
            month: date.month() as u8,
        }
    }
}

impl From<i64> for BashoId {
    fn from(value: i64) -> Self {
        Self {
            year: (value / 100) as u16,
            month: (value % 100) as u8,
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
        value.as_i64().map(|num| num.into())
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

#[derive(Debug, Clone)]
pub struct BashoRange(Range<BashoId>);

impl Deref for BashoRange {
    type Target = Range<BashoId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BashoRange {
    pub fn iter(&self) -> BashoIterator {
        BashoIterator {
            range: self.clone(),
        }
    }

    pub fn to_vec(&self) -> Vec<BashoId> {
        self.iter().collect()
    }
}

pub struct BashoIterator {
    range: BashoRange,
}

impl Iterator for BashoIterator {
    type Item = BashoId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() {
            None
        } else {
            let val = self.range.0.start;
            self.range.0.start = val.next();
            Some(val)
        }
    }
}
