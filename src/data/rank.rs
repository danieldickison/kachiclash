use std::str::Chars;
use std::error::Error;
use std::convert::TryFrom;
use std::fmt;
use rusqlite::types::{FromSql, ToSql, ValueRef, FromSqlResult, FromSqlError};

#[derive(Debug)]
pub enum RankError {
    UnknownChar(char),
    MissingChar,
    ParseIntError(std::num::ParseIntError)
}

impl fmt::Display for RankError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:}", self)
    }
}

impl Error for RankError {}

impl Into<FromSqlError> for RankError {
    fn into(self) -> FromSqlError {
        FromSqlError::Other(Box::new(self))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
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

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum RankSide {
    East,
    West,
}

impl fmt::Display for RankSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            RankSide::East => 'E',
            RankSide::West => 'W',
        })
    }
}

impl TryFrom<char> for RankSide {
    type Error = RankError;
    fn try_from(c: char) -> Result<Self, RankError> {
        match c {
            'E' => Ok(RankSide::East),
            'W' => Ok(RankSide::West),
            _ => Err(RankError::UnknownChar(c))
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Rank {
    pub name: RankName,
    pub number: u8,
    pub side: RankSide,
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}{}", self.name, self.number, self.side)
    }
}

impl FromSql for Rank {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        let parse = |mut chars: Chars| -> Result<Rank, RankError> {
            let name_char = chars.next().ok_or_else(|| RankError::MissingChar.into())?;
            let side_char = chars.next().ok_or_else(|| RankError::MissingChar.into())?;
            Ok(Rank {
                name: RankName::try_from(name_char)?,
                side: RankSide::try_from(side_char)?,
                number: chars.as_str().parse().map_err(|err| RankError::ParseIntError(err))?
            })
        };
        let str = value.as_str()?;
        parse(str.chars()).map_err(|err| err.into())
    }
}
