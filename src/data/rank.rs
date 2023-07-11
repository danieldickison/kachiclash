use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::TryFrom;
use std::error::Error;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

#[derive(Debug)]
pub enum RankError {
    UnknownChar(char),
    MissingChar,
    ParseIntError(std::num::ParseIntError),
}

impl fmt::Display for RankError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for RankError {}

impl From<RankError> for FromSqlError {
    fn from(e: RankError) -> Self {
        Self::Other(Box::new(e))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum RankName {
    Yokozuna,
    Ozeki,
    Sekiwake,
    Komusubi,
    Maegashira,
    Juryo,
}

impl RankName {
    fn next(self) -> Option<Self> {
        match self {
            RankName::Yokozuna => Some(Self::Ozeki),
            RankName::Ozeki => Some(Self::Sekiwake),
            RankName::Sekiwake => Some(Self::Komusubi),
            RankName::Komusubi => Some(Self::Maegashira),
            RankName::Maegashira => Some(Self::Juryo),
            RankName::Juryo => None,
        }
    }
}

impl fmt::Display for RankName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(if f.alternate() {
            match self {
                RankName::Yokozuna => "Yokozuna",
                RankName::Ozeki => "Ozeki",
                RankName::Sekiwake => "Sekiwake",
                RankName::Komusubi => "Komusubi",
                RankName::Maegashira => "Maegashira",
                RankName::Juryo => "Juryo",
            }
        } else {
            match self {
                RankName::Yokozuna => "Y",
                RankName::Ozeki => "O",
                RankName::Sekiwake => "S",
                RankName::Komusubi => "K",
                RankName::Maegashira => "M",
                RankName::Juryo => "J",
            }
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
            'J' => Ok(RankName::Juryo),
            _ => Err(RankError::UnknownChar(c)),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum RankSide {
    East,
    West,
}

impl RankSide {
    pub fn other(self) -> RankSide {
        match self {
            RankSide::East => RankSide::West,
            RankSide::West => RankSide::East,
        }
    }
}

impl fmt::Display for RankSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(if f.alternate() {
            match self {
                RankSide::East => "East",
                RankSide::West => "West",
            }
        } else {
            match self {
                RankSide::East => "e",
                RankSide::West => "w",
            }
        })
    }
}

impl TryFrom<char> for RankSide {
    type Error = RankError;
    fn try_from(c: char) -> Result<Self, RankError> {
        match c {
            'E' | 'e' => Ok(RankSide::East),
            'W' | 'w' => Ok(RankSide::West),
            _ => Err(RankError::UnknownChar(c)),
        }
    }
}

impl ToSql for RankSide {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Hash)]
pub struct RankGroup(pub(crate) u8);

impl RankGroup {
    pub fn for_rank(name: RankName, number: u16) -> Self {
        match name {
            RankName::Yokozuna | RankName::Ozeki => Self(1),
            RankName::Sekiwake | RankName::Komusubi => Self(2),
            RankName::Maegashira => match number {
                0..=5 => Self(3),
                6..=10 => Self(4),
                11..=std::u16::MAX => Self(5),
            },
            RankName::Juryo => Self(6),
        }
    }

    pub fn count() -> usize {
        5
    }

    pub fn as_index(self) -> usize {
        (self.0 - 1) as usize
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

/// A banzuke rank, composed of, in decreasing order of importance: name, number, and side.
///
/// **Note:** "less than" means a higher rank. While this might be unintuitive, we generally want to order things from high rank to low rank, so this is more convenient than the intuitive ordering.
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct Rank {
    pub name: RankName,
    pub number: u16,
    pub side: RankSide,
}

impl Rank {
    pub fn top() -> Self {
        Self {
            name: RankName::Yokozuna,
            number: 1,
            side: RankSide::East,
        }
    }

    pub fn group(self) -> RankGroup {
        RankGroup::for_rank(self.name, self.number)
    }

    pub fn is_makuuchi(self) -> bool {
        self.name <= RankName::Maegashira
    }

    pub fn next_lower(self) -> Self {
        match self {
            Self {
                name: RankName::Yokozuna | RankName::Ozeki | RankName::Sekiwake | RankName::Komusubi,
                side: RankSide::West,
                ..
            } => Self {
                name: self.name.next().unwrap(),
                side: RankSide::East,
                number: 1,
            },

            Self {
                name: RankName::Maegashira,
                side: RankSide::West,
                number: 17,
            } => Self {
                name: RankName::Juryo,
                side: RankSide::East,
                number: 1,
            },

            Self {
                side: RankSide::East,
                ..
            } => Self {
                name: self.name,
                side: RankSide::West,
                number: self.number,
            },

            _ => Self {
                name: self.name,
                side: RankSide::East,
                number: self.number + 1,
            },
        }
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{:#} {} {:#}", self.name, self.number, self.side)
        } else {
            write!(f, "{}{}{}", self.name, self.number, self.side)
        }
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
        let name_char = chars.next().ok_or(RankError::MissingChar)?;
        let side_char = chars.next_back().ok_or(RankError::MissingChar)?;
        let num_str = chars.as_str();
        //debug!("parsing rank got name char {} side char {} with remaining {}", name_char, side_char, num_str);
        Ok(Rank {
            name: RankName::try_from(name_char)?,
            side: RankSide::try_from(side_char)?,
            number: num_str.parse().map_err(RankError::ParseIntError)?,
        })
    }
}

impl<'de> Deserialize<'de> for Rank {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for Rank {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse() {
        assert_eq!(
            Rank {
                name: RankName::Yokozuna,
                number: 1,
                side: RankSide::East
            },
            Rank::from_str("Y1e").unwrap()
        );
        assert_eq!(
            Rank {
                name: RankName::Maegashira,
                number: 1,
                side: RankSide::East
            },
            Rank::from_str("M1e").unwrap()
        );
        assert_eq!(
            Rank {
                name: RankName::Maegashira,
                number: 15,
                side: RankSide::West
            },
            Rank::from_str("M15w").unwrap()
        );
    }

    #[test]
    fn serialize() {
        assert_eq!("Y1e", Rank::from_str("Y1e").unwrap().to_string());
        assert_eq!("Y1w", Rank::from_str("Y1w").unwrap().to_string());
        assert_eq!("O2e", Rank::from_str("O2e").unwrap().to_string());
        assert_eq!("M11w", Rank::from_str("M11w").unwrap().to_string());
    }

    #[test]
    fn display_alt() {
        assert_eq!(
            "Yokozuna 1 East",
            format!("{:#}", Rank::from_str("Y1e").unwrap())
        );
        assert_eq!(
            "Yokozuna 1 West",
            format!("{:#}", Rank::from_str("Y1w").unwrap())
        );
        assert_eq!(
            "Ozeki 2 East",
            format!("{:#}", Rank::from_str("O2e").unwrap())
        );
        assert_eq!(
            "Sekiwake 1 West",
            format!("{:#}", Rank::from_str("S1w").unwrap())
        );
        assert_eq!(
            "Komusubi 1 East",
            format!("{:#}", Rank::from_str("K1e").unwrap())
        );
        assert_eq!(
            "Maegashira 1 East",
            format!("{:#}", Rank::from_str("M1e").unwrap())
        );
        assert_eq!(
            "Maegashira 11 West",
            format!("{:#}", Rank::from_str("M11w").unwrap())
        );
        assert_eq!(
            "Juryo 11 West",
            format!("{:#}", Rank::from_str("J11w").unwrap())
        );
    }

    #[test]
    #[should_panic]
    fn reject_lowercase_name() {
        Rank::from_str("o1e").unwrap();
    }

    #[test]
    #[should_panic]
    fn reject_invalid_name() {
        Rank::from_str("X1e").unwrap();
    }

    #[test]
    #[should_panic]
    fn reject_invalid_side() {
        Rank::from_str("M1n").unwrap();
    }

    #[test]
    fn ordering() {
        // Note: "less than" means a higher rank. While this might be unintuitive, we generally want to order things from high rank to low rank, so this is more convenient than the intuitive ordering.
        assert!(Rank::top() < Rank::top().next_lower());
        assert!(Rank::from_str("Y1e").unwrap() < Rank::from_str("M1e").unwrap());
        assert!(Rank::from_str("S2w").unwrap() < Rank::from_str("K1e").unwrap());
        assert!(Rank::from_str("O1e").unwrap() < Rank::from_str("O1w").unwrap());
        assert!(Rank::from_str("M3e").unwrap() < Rank::from_str("M3w").unwrap());
        assert!(Rank::from_str("M1e").unwrap() < Rank::from_str("M15e").unwrap());
        assert!(Rank::from_str("O1e").unwrap() < Rank::from_str("M15e").unwrap());
        assert!(Rank::from_str("M15w").unwrap() < Rank::from_str("J1e").unwrap());
    }
}
