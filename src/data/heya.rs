use std::ops::RangeInclusive;

use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use slug::slugify;

use super::{DataError, Player, PlayerId, Result};

pub const MEMBER_MAX: usize = 50;
pub const JOIN_MAX: usize = 5;
pub const HOST_MAX: usize = 3;
pub const NAME_LENGTH: RangeInclusive<usize> = 3..=30;

pub type HeyaId = i64;

#[derive(Debug)]
pub struct Heya {
    pub id: HeyaId,
    pub name: String,
    pub slug: String,
    pub oyakata: Player,
    pub member_count: usize,
    pub members: Option<Vec<Player>>, // might not be populated in all cases
}

impl Heya {
    pub fn list_all(db: &Connection) -> SqlResult<Vec<Heya>> {
        db.prepare(
            "
                SELECT
                    heya.id AS heya_id,
                    heya.name AS heya_name,
                    heya.slug AS heya_slug,
                    heya.oyakata_player_id,
                    (
                        SELECT COUNT(*) FROM heya_player AS hp2
                        WHERE hp2.heya_id = heya.id
                    ) AS member_count,
                    oyakata.*
                FROM heya
                JOIN player_info AS oyakata ON oyakata.id = heya.oyakata_player_id
                ORDER BY heya.name
            ",
        )?
        .query_and_then(params![], |row| {
            Ok(Self {
                id: row.get("heya_id")?,
                name: row.get("heya_name")?,
                slug: row.get("heya_slug")?,
                oyakata: Player::from_row(row)?,
                members: None,
                member_count: row.get("member_count")?,
            })
        })?
        .collect()
    }

    pub fn with_slug(db: &Connection, slug: &str) -> SqlResult<Option<Self>> {
        match db
            .query_row_and_then(
                "
                    SELECT
                        heya.id AS heya_id,
                        heya.name AS heya_name,
                        heya.slug AS heya_slug,
                        oyakata.*
                    FROM heya
                    JOIN player_info AS oyakata ON oyakata.id = heya.oyakata_player_id
                    WHERE slug = ?
                ",
                params![slug],
                |row| {
                    Ok(Self {
                        id: row.get("heya_id")?,
                        name: row.get("heya_name")?,
                        slug: row.get("heya_slug")?,
                        oyakata: Player::from_row(row)?,
                        members: None,
                        member_count: 0,
                    })
                },
            )
            .optional()?
        {
            Some(mut heya) => {
                let members = Self::fetch_members(&db, heya.id)?;
                heya.member_count = members.len();
                heya.members = Some(members);
                Ok(Some(heya))
            }
            None => Ok(None),
        }
    }

    pub fn for_player(db: &Connection, player_id: PlayerId) -> SqlResult<Vec<Self>> {
        db.prepare(
            "
                SELECT
                    heya.id AS heya_id,
                    heya.name AS heya_name,
                    heya.slug AS heya_slug,
                    (
                        SELECT COUNT(*) FROM heya_player AS hp2
                        WHERE hp2.heya_id = heya.id
                    ) AS member_count,
                    oyakata.*
                FROM heya_player AS hp
                JOIN heya ON heya.id = hp.heya_id
                JOIN player_info AS oyakata ON oyakata.id = heya.oyakata_player_id
                WHERE hp.player_id = ?
            ",
        )?
        .query_and_then(params![player_id], |row| {
            Ok(Self {
                id: row.get("heya_id")?,
                name: row.get("heya_name")?,
                slug: row.get("heya_slug")?,
                oyakata: Player::from_row(row)?,
                members: None,
                member_count: row.get("member_count")?,
            })
        })?
        .collect()
    }

    fn fetch_members(db: &Connection, heya_id: HeyaId) -> SqlResult<Vec<Player>> {
        Ok(db
            .prepare(
                "
                    SELECT p.*
                    FROM heya_player AS hp
                    JOIN player_info AS p ON p.id = hp.player_id
                    WHERE hp.heya_id = ?
                    ORDER BY p.name
                ",
            )
            .unwrap()
            .query_map(params![heya_id], Player::from_row)?
            .map(|r| r.unwrap())
            .collect())
    }

    pub fn url_path(&self) -> String {
        format!("/heya/{}", self.slug)
    }

    pub fn new(db: &mut Connection, name: &str, oyakata: PlayerId) -> Result<Self> {
        Self::validate_name(name)?;
        let slug = slugify(name);
        let txn = db.transaction()?;
        txn.prepare(
            "
                INSERT INTO heya (name, slug, oyakata_player_id)
                VALUES (?, ?, ?)
            ",
        )?
        .execute(params![name, slug, oyakata])?;
        let heya_id = txn.last_insert_rowid();
        txn.prepare(
            "
                INSERT INTO heya_player (heya_id, player_id)
                VALUES (?, ?)
            ",
        )?
        .execute(params![heya_id, oyakata])?;

        Self::validate_quota(&txn, oyakata)?;
        let heya = Self::with_slug(&txn, &slug)?.ok_or(DataError::HeyaIntegrity {
            what: "heya failed to insert".to_string(),
        })?;
        txn.commit()?;

        Ok(heya)
    }

    pub fn set_name(&mut self, db: &Connection, name: &str) -> Result<()> {
        Self::validate_name(name)?;
        db.prepare(
            "
                UPDATE heya SET name = ? WHERE id = ?
            ",
        )?
        .execute(params![name, self.id])?;
        Ok(())
    }

    pub fn add_member(&mut self, db: &mut Connection, player: PlayerId) -> Result<()> {
        let txn = db.transaction()?;
        txn.prepare(
            "
                INSERT INTO heya_player (heya_id, player_id)
                VALUES (?, ?)
            ",
        )?
        .execute(params![self.id, player])?;

        Self::validate_quota(&txn, player)?;

        self.member_count += 1;
        self.members = None;

        txn.commit()?;
        Ok(())
    }

    pub fn delete_member(&mut self, db: &mut Connection, player: PlayerId) -> Result<()> {
        if self.oyakata.id == player {
            return Err(DataError::HeyaIntegrity {
                what: "Oyakata can’t leave heya".to_string(),
            });
        }

        let txn = db.transaction()?;
        txn.prepare(
            "
                DELETE FROM heya_player
                WHERE player_id = ?
            ",
        )?
        .execute(params![player])?;
        self.member_count -= 1;
        self.members = None;
        txn.commit()?;
        Ok(())
    }

    fn validate_name(name: &str) -> Result<()> {
        if NAME_LENGTH.contains(&name.len()) {
            Ok(())
        } else {
            Err(DataError::HeyaIntegrity {
                what: format!(
                    "Name must be {} to {} characters",
                    NAME_LENGTH.start(),
                    NAME_LENGTH.end()
                ),
            })
        }
    }

    fn validate_quota(db: &Connection, player: PlayerId) -> Result<()> {
        let player_heyas = Self::for_player(&db, player)?;
        if player_heyas.len() > JOIN_MAX {
            return Err(DataError::HeyaIntegrity {
                what: format!("Player {} in too many heyas (max {})", player, JOIN_MAX),
            });
        }

        if player_heyas
            .iter()
            .filter(|h| h.oyakata.id == player)
            .count()
            > HOST_MAX
        {
            return Err(DataError::HeyaIntegrity {
                what: format!(
                    "Player {} hosting too many heyas (max {})",
                    player, HOST_MAX
                ),
            });
        }

        if let Some(heya) = player_heyas.iter().find(|h| h.member_count > MEMBER_MAX) {
            return Err(DataError::HeyaIntegrity {
                what: format!("Heya {} is full (max {} members)", heya.name, MEMBER_MAX),
            });
        }

        Ok(())
    }
}