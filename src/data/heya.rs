use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use slug::slugify;

use super::{Player, PlayerId, Result};

pub type HeyaId = i64;

#[derive(Debug)]
pub struct Heya {
    pub id: HeyaId,
    pub name: String,
    pub slug: String,
    pub oyakata_player_id: PlayerId,
    pub members: Option<Vec<Player>>,
    pub member_count: usize,
}

impl Heya {
    pub fn list_all(db: &Connection) -> SqlResult<Vec<Heya>> {
        db.prepare(
            "
                SELECT
                    heya.*,
                    (
                        SELECT COUNT(*) FROM heya_player AS hp2
                        WHERE hp2.heya_id = heya.id
                    ) AS member_count
                FROM heya
                ORDER BY heya.name
            ",
        )?
        .query_and_then(params![], |row| {
            Ok(Self {
                id: row.get("id")?,
                name: row.get("name")?,
                slug: row.get("slug")?,
                oyakata_player_id: row.get("oyakata_player_id")?,
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
                        oyakata_player_id: row.get("oyakata_player_id")?,
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
                    heya.*,
                    (
                        SELECT COUNT(*) FROM heya_player AS hp2
                        WHERE hp2.heya_id = heya.id
                    ) AS member_count
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
                oyakata_player_id: row.get("oyakata_player_id")?,
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

    pub fn oyakata(&self) -> Option<&Player> {
        self.members
            .as_ref()
            .and_then(|m| m.iter().find(|p| p.id == self.oyakata_player_id))
    }

    pub fn url_path(&self) -> String {
        format!("/heya/{}", self.slug)
    }

    pub fn new(db: &mut Connection, name: &str, oyakata: PlayerId) -> Result<Self> {
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
        txn.commit()?;
        Ok(Self {
            id: heya_id,
            name: name.to_string(),
            slug,
            oyakata_player_id: oyakata,
            members: None,
            member_count: 1,
        })
    }

    pub fn add_member(&self, db: &Connection, player: PlayerId) -> Result<()> {
        db.prepare(
            "
                INSERT INTO heya_player (heya_id, player_id)
                VALUES (?, ?)
            ",
        )?
        .execute(params![self.id, player])?;
        Ok(())
    }
}
