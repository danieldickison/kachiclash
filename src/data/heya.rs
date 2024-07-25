use rusqlite::{Connection, OptionalExtension};

use super::{Player, PlayerId, Result};

pub type HeyaId = i64;

pub struct Heya {
    pub id: HeyaId,
    pub name: String,
    pub slug: String,
    pub owner_player_id: PlayerId,
    pub members: Option<Vec<Player>>,
}

impl Heya {
    pub fn with_slug(db: &Connection, slug: &str) -> Result<Option<Self>> {
        match db
            .query_row_and_then(
                "
                SELECT *
                FROM heya
                WHERE slug = ?
            ",
                params![slug],
                |row| {
                    Ok(Self {
                        id: row.get("id")?,
                        name: row.get("name")?,
                        slug: row.get("slug")?,
                        owner_player_id: row.get("owner_player_id")?,
                        members: None,
                    })
                },
            )
            .optional()?
        {
            Some(mut heya) => {
                heya.members = Some(Self::fetch_members(&db, heya.id)?);
                Ok(Some(heya))
            }
            None => Ok(None),
        }
    }

    pub fn owner(&self) -> Option<&Player> {
        self.members
            .as_ref()
            .and_then(|m| m.iter().find(|p| p.id == self.owner_player_id))
    }

    pub fn for_player(db: &Connection, player_id: PlayerId) -> Result<Vec<Self>> {
        db.prepare(
            "
                    SELECT heya.*
                    FROM heya_player AS hp
                    JOIN heya ON heya.id = hp.heya_id
                    WHERE hp.player_id = ?
                ",
        )?
        .query_and_then(params![player_id], |row| {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                slug: row.get("slug")?,
                owner_player_id: row.get("owner_player_id")?,
                members: None,
            })
        })?
        .collect()
    }

    fn fetch_members(db: &Connection, heya_id: HeyaId) -> Result<Vec<Player>> {
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
}
