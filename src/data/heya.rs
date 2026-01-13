use std::ops::RangeInclusive;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult, Row};
use slug_intl::slugify;

use super::{BashoId, BashoInfo, DataError, Player, PlayerId, Result};

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
    pub create_date: DateTime<Utc>,
    pub member_count: usize,
    pub members: Option<Vec<Member>>, // might not be populated in all cases
    pub recent_scores_bashos: Option<Vec<BashoId>>,
    pub recruit_date: Option<DateTime<Utc>>, // of the player for `for_player`
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
                    heya.create_date,
                    (
                        SELECT COUNT(*) FROM heya_player AS hp2
                        WHERE hp2.heya_id = heya.id
                    ) AS member_count,
                    oyakata.*
                FROM heya
                JOIN player_info AS oyakata ON oyakata.id = heya.oyakata_player_id
                ORDER BY heya.name COLLATE NOCASE
            ",
        )?
        .query_and_then(params![], |row| {
            Ok(Self {
                id: row.get("heya_id")?,
                name: row.get("heya_name")?,
                slug: row.get("heya_slug")?,
                create_date: row.get("create_date")?,
                oyakata: Player::from_row(row)?,
                members: None,
                member_count: row.get("member_count")?,
                recent_scores_bashos: None,
                recruit_date: None,
            })
        })?
        .collect()
    }

    pub fn with_id(db: &Connection, id: HeyaId, include_members: bool) -> Result<Self> {
        Self::query_one(db, Some(id), None, include_members)
    }

    pub fn with_slug(db: &Connection, slug: &str, include_members: bool) -> Result<Self> {
        Self::query_one(db, None, Some(slug), include_members)
    }

    pub fn query_one(
        db: &Connection,
        id: Option<HeyaId>,
        slug: Option<&str>,
        include_members: bool,
    ) -> Result<Self> {
        let rank_for_basho = BashoInfo::current_or_next_basho_id(db)?;
        let (where_clause, params) = if id.is_some() {
            ("heya.id = ?", params![id])
        } else if slug.is_some() {
            ("heya.slug = ?", params![slug])
        } else {
            panic!("must specify id or slug for Heya::query_one");
        };
        match db
            .query_row_and_then(
                &format!(
                    "
                    SELECT
                        heya.id AS heya_id,
                        heya.name AS heya_name,
                        heya.slug AS heya_slug,
                        heya.create_date,
                        (
                            SELECT COUNT(*) FROM heya_player AS hp2
                            WHERE hp2.heya_id = heya.id
                        ) AS member_count,
                        oyakata.*
                    FROM heya
                    JOIN player_info AS oyakata ON oyakata.id = heya.oyakata_player_id
                    WHERE {where_clause}
                "
                ),
                params,
                |row| -> SqlResult<Heya> {
                    Ok(Self {
                        id: row.get("heya_id")?,
                        name: row.get("heya_name")?,
                        slug: row.get("heya_slug")?,
                        create_date: row.get("create_date")?,
                        oyakata: Player::from_row(row)?,
                        members: None,
                        member_count: row.get("member_count")?,
                        recent_scores_bashos: None,
                        recruit_date: None,
                    })
                },
            )
            .optional()?
        {
            Some(mut heya) => {
                if include_members {
                    let current_basho = BashoInfo::with_id(db, rank_for_basho)?;
                    let include_current_basho = current_basho.is_some();

                    let members =
                        Member::in_heya(db, heya.id, rank_for_basho, include_current_basho)?;
                    heya.members = Some(members);

                    let mut bashos = rank_for_basho.range_for_banzuke().to_vec();
                    if include_current_basho {
                        bashos.push(rank_for_basho);
                    }
                    bashos.reverse();
                    heya.recent_scores_bashos = Some(bashos);
                }
                Ok(heya)
            }
            None => Err(DataError::HeyaNotFound {
                slug: slug.map(|s| s.to_string()),
                id,
            }),
        }
    }

    pub fn for_player(db: &Connection, player_id: PlayerId) -> SqlResult<Vec<Self>> {
        db.prepare(
            "
                SELECT
                    heya.id AS heya_id,
                    heya.name AS heya_name,
                    heya.slug AS heya_slug,
                    heya.create_date,
                    (
                        SELECT COUNT(*) FROM heya_player AS hp2
                        WHERE hp2.heya_id = heya.id
                    ) AS member_count,
                    oyakata.*,
                    hp.recruit_date
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
                create_date: row.get("create_date")?,
                oyakata: Player::from_row(row)?,
                members: None,
                member_count: row.get("member_count")?,
                recent_scores_bashos: None,
                recruit_date: Some(row.get("recruit_date")?),
            })
        })?
        .collect()
    }

    pub fn url_path(&self) -> String {
        format!("/heya/{}", self.slug)
    }

    pub fn new(db: &mut Connection, name: &str, oyakata: PlayerId) -> Result<Self> {
        Self::validate_name(name)?;
        let slug = slugify(name);
        let now = Utc::now();
        let txn = db.transaction()?;
        txn.prepare(
            "
                INSERT INTO heya (name, slug, oyakata_player_id, create_date)
                VALUES (?, ?, ?, ?)
            ",
        )?
        .execute(params![name, slug, oyakata, now])?;
        let heya_id = txn.last_insert_rowid();
        txn.prepare(
            "
                INSERT INTO heya_player (heya_id, player_id, recruit_date)
                VALUES (?, ?, ?)
            ",
        )?
        .execute(params![heya_id, oyakata, now])?;

        Self::validate_quota(&txn, oyakata)?;
        let heya = Self::with_slug(&txn, &slug, false)?;
        txn.commit()?;

        Ok(heya)
    }

    pub fn set_name(&mut self, db: &Connection, name: &str) -> Result<()> {
        Self::validate_name(name)?;
        let slug = slugify(name);
        db.prepare(
            "
                UPDATE heya SET name = ?, slug = ? WHERE id = ?
            ",
        )?
        .execute(params![name, slug, self.id])?;
        Ok(())
    }

    pub fn add_member(&mut self, db: &mut Connection, player: PlayerId) -> Result<()> {
        let txn = db.transaction()?;
        txn.prepare(
            "
                INSERT INTO heya_player (heya_id, player_id, recruit_date)
                VALUES (?, ?, ?)
            ",
        )?
        .execute(params![self.id, player, Utc::now()])?;

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
                WHERE heya_id = ? AND player_id = ?
            ",
        )?
        .execute(params![self.id, player])?;
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
        let player_heyas = Self::for_player(db, player)?;
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

#[derive(Debug)]
pub struct Member {
    pub player: Player,
    pub is_oyakata: bool,
    pub is_self: bool,
    pub recruit_date: DateTime<Utc>,
    pub recent_scores: Vec<Option<u8>>,
}

const SCORE_SENTINAL_NO_ENTRY: u8 = u8::MAX;

impl Member {
    fn from_row(row: &Row) -> SqlResult<Self> {
        let player = Player::from_row(row)?;
        Ok(Self {
            player,
            recruit_date: row.get("recruit_date")?,
            is_oyakata: row.get("is_oyakata")?,
            is_self: false,
            recent_scores: row
                .get::<_, String>("recent_scores")?
                .split(",")
                .map(|s| {
                    let score = s.parse::<u8>().unwrap();
                    if score == SCORE_SENTINAL_NO_ENTRY {
                        None
                    } else {
                        Some(score)
                    }
                })
                .collect(),
        })
    }

    fn in_heya(
        db: &Connection,
        heya_id: HeyaId,
        rank_for_basho: BashoId,
        include_current_basho: bool,
    ) -> SqlResult<Vec<Self>> {
        let basho_range = rank_for_basho.range_for_banzuke();
        let before_basho_operator = if include_current_basho { "<=" } else { "<" };
        Ok(db
            .prepare(
                &format!("
                    SELECT
                        p.*,
                        pr.rank,
                        hp.recruit_date,
                        p.id = h.oyakata_player_id AS is_oyakata,
                        (
                            SELECT GROUP_CONCAT(
                                COALESCE(
                                    br.wins,
                                    IIF(EXISTS(
                                        SELECT 1 FROM pick WHERE player_id = p.id AND basho_id = b.id
                                    ), 0, {SCORE_SENTINAL_NO_ENTRY})
                                )
                                ORDER BY b.id DESC
                            )
                            FROM basho AS b
                            LEFT JOIN basho_result AS br ON br.basho_id = b.id AND br.player_id = p.id
                            WHERE b.id >= :first_basho AND b.id {before_basho_operator} :before_basho
                        ) AS recent_scores
                    FROM heya AS h
                    JOIN heya_player AS hp ON hp.heya_id = h.id
                    JOIN player_info AS p ON p.id = hp.player_id
                    LEFT JOIN player_rank AS pr ON pr.player_id = p.id AND pr.before_basho_id = :before_basho
                    WHERE h.id = :heya
                "),
            )
            .unwrap()
            .query_map(named_params! {
                ":heya": heya_id,
                ":before_basho": basho_range.end,
                ":first_basho": basho_range.start
            }, Self::from_row)?
            .map(|r| r.unwrap())
            .sorted_by_key(|m| u16::MAX - m.recent_scores_total())
            .collect())
    }

    pub fn recent_scores_total(&self) -> u16 {
        self.recent_scores
            .iter()
            .map(|s| s.unwrap_or(0) as u16)
            .sum()
    }
}
