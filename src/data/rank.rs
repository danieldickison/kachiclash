use std::ops::Deref;
use std::str::FromStr;
use std::error::Error;
use std::convert::TryFrom;
use std::fmt;
use serde::{Deserialize, Deserializer};
use rusqlite::types::{FromSql, ToSql, ValueRef, FromSqlResult, FromSqlError, ToSqlOutput};

#[derive(Debug)]
pub enum RankError {
    UnknownChar(char),
    MissingChar,
    ParseIntError(std::num::ParseIntError)
}

impl fmt::Display for RankError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for RankError {}

impl Into<FromSqlError> for RankError {
    fn into(self) -> FromSqlError {
        FromSqlError::Other(Box::new(self))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum RankName {
    Yokozuna,
    Ozeki,
    Sekiwake,
    Komusubi,
    Maegashira,
}

impl fmt::Display for RankName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            RankName::Yokozuna => 'Y',
            RankName::Ozeki => 'O',
            RankName::Sekiwake => 'S',
            RankName::Komusubi => 'K',
            RankName::Maegashira => 'M',
        })
    }
}

impl TryFrom<char> for RankName {
    type Error = RankError;
    fn try_from(c: char) -> Result<Self, RankError> {
        match c {
            'Y' => Ok(RankName::Yokozuna),
            'O' => Ok(RankName::Ozeki),
            'S' => Ok(RankName::Sekiwake),
            'K' => Ok(RankName::Komusubi),
            'M' => Ok(RankName::Maegashira),
            _ => Err(RankError::UnknownChar(c))
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum RankSide {
    East,
    West,
}

impl fmt::Display for RankSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            RankSide::East => 'e',
            RankSide::West => 'w',
        })
    }
}

impl TryFrom<char> for RankSide {
    type Error = RankError;
    fn try_from(c: char) -> Result<Self, RankError> {
        match c {
            'E' | 'e' => Ok(RankSide::East),
            'W' | 'w' => Ok(RankSide::West),
            _ => Err(RankError::UnknownChar(c))
        }
    }
}

impl ToSql for RankSide {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Hash)]
pub struct RankGroup(u8);

impl RankGroup {
    pub fn for_rank(name: RankName, number: u16) -> Self {
        match name {
            RankName::Yokozuna | RankName::Ozeki => Self(1),
            RankName::Sekiwake | RankName::Komusubi => Self(2),
            RankName::Maegashira => match number {
                0..=5 => Self(3),
                6..=10 => Self(4),
                11..=std::u16::MAX => Self(5),
            }
        }
    }
}

impl Deref for RankGroup {
    type Target = u8;

    fn deref(&self) -> &u8 {
        &self.0
    }
}

impl fmt::Display for RankGroup {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct Rank {
    pub name: RankName,
    pub number: u16,
    pub side: RankSide,
}

impl Rank {
    pub fn group(self) -> RankGroup {
        RankGroup::for_rank(self.name, self.number)
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.name, self.number, self.side)
    }
}

impl FromSql for Rank {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let str = value.as_str()?;
        str.parse().map_err(|err: RankError| err.into())
    }
}

impl ToSql for Rank {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromStr for Rank {
    type Err = RankError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        let name_char = chars.next().ok_or_else(|| RankError::MissingChar)?;
        let side_char = chars.next_back().ok_or_else(|| RankError::MissingChar)?;
        let num_str = chars.as_str();
        //debug!("parsing rank got name char {} side char {} with remaining {}", name_char, side_char, num_str);
        Ok(Rank {
            name: RankName::try_from(name_char)?,
            side: RankSide::try_from(side_char)?,
            number: num_str.parse().map_err(RankError::ParseIntError)?
        })
    }
}

impl<'de> Deserialize<'de> for Rank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
