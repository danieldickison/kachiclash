use crate::data::{
    player::BashoScore, BashoId, BashoInfo, BashoRikishi as DataBashoRikishi, DbConn,
    FetchBashoRikishi, Player as DataPlayer, PlayerId,
};
use crate::graphql::types::*;
use async_graphql::*;

pub type GraphQLSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get all bashos (tournaments)
    async fn bashos(&self, ctx: &Context<'_>, filter: Option<BashoFilter>) -> Result<Vec<Basho>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        let basho_infos = if let Some(ref filter) = filter {
            if let Some(basho_id_str) = &filter.id {
                // Parse basho ID string
                let basho_id: BashoId = basho_id_str
                    .parse()
                    .map_err(|_| Error::new("Invalid basho ID format"))?;
                // Get specific basho
                match BashoInfo::with_id(&conn, basho_id)? {
                    Some(info) => vec![info],
                    None => vec![],
                }
            } else {
                // Get all bashos
                BashoInfo::list_all(&conn)?
            }
        } else {
            BashoInfo::list_all(&conn)?
        };

        let mut bashos = Vec::new();
        for info in basho_infos {
            bashos.push(Basho {
                id: info.id.id(),
                start_date: info.start_date,
                venue: info.venue.clone(),
                external_link: info.external_link.clone(),
                player_count: info.player_count,
                winning_score: info.winning_score,
                has_started: info.has_started(),
            });
        }

        // Apply limit if specified
        if let Some(filter) = &filter {
            if let Some(limit) = filter.limit {
                bashos.truncate(limit as usize);
            }
        }

        Ok(bashos)
    }

    /// Get a specific basho by ID
    async fn basho(&self, ctx: &Context<'_>, id: String) -> Result<Option<Basho>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        let basho_id: BashoId = id
            .parse()
            .map_err(|_| Error::new("Invalid basho ID format"))?;

        match BashoInfo::with_id(&conn, basho_id)? {
            Some(info) => Ok(Some(Basho {
                id: info.id.id(),
                start_date: info.start_date,
                venue: info.venue.clone(),
                external_link: info.external_link.clone(),
                player_count: info.player_count,
                winning_score: info.winning_score,
                has_started: info.has_started(),
            })),
            None => Ok(None),
        }
    }

    /// Get all players
    async fn players(
        &self,
        ctx: &Context<'_>,
        filter: Option<PlayerFilter>,
    ) -> Result<Vec<Player>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        // Get current basho for rank information
        let current_basho = BashoInfo::current_or_next_basho_id(&conn)?;

        let mut data_players = if let Some(filter) = &filter {
            if let Some(player_id) = filter.id {
                // Get specific player
                match DataPlayer::with_id(&conn, player_id, current_basho)? {
                    Some(player) => vec![player],
                    None => vec![],
                }
            } else if let Some(name) = &filter.name {
                // Get player by name
                match DataPlayer::with_name(&conn, name.clone(), current_basho)? {
                    Some(player) => vec![player],
                    None => vec![],
                }
            } else {
                // Get all players
                DataPlayer::list_all(&conn, current_basho)?
            }
        } else {
            DataPlayer::list_all(&conn, current_basho)?
        };

        // Apply filters
        if let Some(filter) = &filter {
            if let Some(has_cup) = filter.has_emperors_cup {
                data_players.retain(|p| p.has_emperors_cup() == has_cup);
            }
        }

        let mut players = Vec::new();
        for data_player in data_players {
            players.push(Player {
                id: data_player.id,
                name: data_player.name.clone(),
                join_date: data_player.join_date,
                emperors_cups: data_player.emperors_cups,
                rank: data_player.rank.map(|r| r.to_string()),
                has_emperors_cup: data_player.has_emperors_cup(),
                url_path: data_player.url_path(),
            });
        }

        // Apply limit if specified
        if let Some(filter) = filter {
            if let Some(limit) = filter.limit {
                players.truncate(limit as usize);
            }
        }

        Ok(players)
    }

    /// Get a specific player by ID
    async fn player(&self, ctx: &Context<'_>, id: PlayerId) -> Result<Option<Player>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        // Get current basho for rank information
        let current_basho = BashoInfo::current_or_next_basho_id(&conn)?;

        match DataPlayer::with_id(&conn, id, current_basho)? {
            Some(data_player) => Ok(Some(Player {
                id: data_player.id,
                name: data_player.name.clone(),
                join_date: data_player.join_date,
                emperors_cups: data_player.emperors_cups,
                rank: data_player.rank.map(|r| r.to_string()),
                has_emperors_cup: data_player.has_emperors_cup(),
                url_path: data_player.url_path(),
            })),
            None => Ok(None),
        }
    }

    /// Get a player by name
    async fn player_by_name(&self, ctx: &Context<'_>, name: String) -> Result<Option<Player>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        // Get current basho for rank information
        let current_basho = BashoInfo::current_or_next_basho_id(&conn)?;

        match DataPlayer::with_name(&conn, name, current_basho)? {
            Some(data_player) => Ok(Some(Player {
                id: data_player.id,
                name: data_player.name.clone(),
                join_date: data_player.join_date,
                emperors_cups: data_player.emperors_cups,
                rank: data_player.rank.map(|r| r.to_string()),
                has_emperors_cup: data_player.has_emperors_cup(),
                url_path: data_player.url_path(),
            })),
            None => Ok(None),
        }
    }

    /// Get player scores for a specific basho
    async fn player_scores(
        &self,
        ctx: &Context<'_>,
        basho_id: String,
        player_id: Option<PlayerId>,
    ) -> Result<Vec<PlayerScore>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        let basho_id_parsed: BashoId = basho_id
            .parse()
            .map_err(|_| Error::new("Invalid basho ID format"))?;

        if let Some(pid) = player_id {
            // Get scores for specific player
            let player = DataPlayer::with_id(&conn, pid, basho_id_parsed)?
                .ok_or_else(|| Error::new("Player not found"))?;

            let basho_scores = BashoScore::with_player_id(&conn, pid, &player.name)?;
            let target_score = basho_scores
                .into_iter()
                .find(|bs| bs.basho_id == basho_id_parsed);

            if let Some(score) = target_score {
                let basho_info = BashoInfo::with_id(&conn, basho_id_parsed)?
                    .ok_or_else(|| Error::new("Basho not found"))?;

                let player_score = PlayerScore {
                    player: Player {
                        id: player.id,
                        name: player.name.clone(),
                        join_date: player.join_date,
                        emperors_cups: player.emperors_cups,
                        rank: player.rank.map(|r| r.to_string()),
                        has_emperors_cup: player.has_emperors_cup(),
                        url_path: player.url_path(),
                    },
                    basho: Basho {
                        id: basho_info.id.id(),
                        start_date: basho_info.start_date,
                        venue: basho_info.venue.clone(),
                        external_link: basho_info.external_link.clone(),
                        player_count: basho_info.player_count,
                        winning_score: basho_info.winning_score,
                        has_started: basho_info.has_started(),
                    },
                    rank: score.rank.map(|r| r.to_string()),
                    rikishi: score
                        .rikishi
                        .into_iter()
                        .map(|r| {
                            r.map(|rikishi| PlayerBashoRikishi {
                                name: rikishi.name,
                                wins: rikishi.wins,
                                losses: rikishi.losses,
                            })
                        })
                        .collect(),
                    wins: score.wins,
                    place: score.place,
                    awards: score
                        .awards
                        .into_iter()
                        .map(|award| Award {
                            award_type: format!("{:?}", award),
                            name: match award {
                                crate::data::Award::EmperorsCup => "Emperor's Cup".to_string(),
                            },
                        })
                        .collect(),
                };

                Ok(vec![player_score])
            } else {
                Ok(vec![])
            }
        } else {
            // Get scores for all players in the basho
            let basho_info = BashoInfo::with_id(&conn, basho_id_parsed)?
                .ok_or_else(|| Error::new("Basho not found"))?;

            let players = DataPlayer::list_all(&conn, basho_id_parsed)?;
            let mut player_scores = Vec::new();

            for player in players {
                let basho_scores = BashoScore::with_player_id(&conn, player.id, &player.name)?;
                if let Some(score) = basho_scores
                    .into_iter()
                    .find(|bs| bs.basho_id == basho_id_parsed)
                {
                    player_scores.push(PlayerScore {
                        player: Player {
                            id: player.id,
                            name: player.name.clone(),
                            join_date: player.join_date,
                            emperors_cups: player.emperors_cups,
                            rank: player.rank.map(|r| r.to_string()),
                            has_emperors_cup: player.has_emperors_cup(),
                            url_path: player.url_path(),
                        },
                        basho: Basho {
                            id: basho_info.id.id(),
                            start_date: basho_info.start_date,
                            venue: basho_info.venue.clone(),
                            external_link: basho_info.external_link.clone(),
                            player_count: basho_info.player_count,
                            winning_score: basho_info.winning_score,
                            has_started: basho_info.has_started(),
                        },
                        rank: score.rank.map(|r| r.to_string()),
                        rikishi: score
                            .rikishi
                            .into_iter()
                            .map(|r| {
                                r.map(|rikishi| PlayerBashoRikishi {
                                    name: rikishi.name,
                                    wins: rikishi.wins,
                                    losses: rikishi.losses,
                                })
                            })
                            .collect(),
                        wins: score.wins,
                        place: score.place,
                        awards: score
                            .awards
                            .into_iter()
                            .map(|award| Award {
                                award_type: format!("{:?}", award),
                                name: match award {
                                    crate::data::Award::EmperorsCup => "Emperor's Cup".to_string(),
                                },
                            })
                            .collect(),
                    });
                }
            }

            Ok(player_scores)
        }
    }

    /// Get rikishi (wrestlers) for a specific basho
    async fn basho_rikishi(&self, ctx: &Context<'_>, basho_id: String) -> Result<Vec<Rikishi>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        let basho_id_parsed: BashoId = basho_id
            .parse()
            .map_err(|_| Error::new("Invalid basho ID format"))?;

        let empty_picks = std::collections::HashSet::new();
        let fetch_result = FetchBashoRikishi::with_db(&conn, basho_id_parsed, &empty_picks)?;
        let mut rikishi_list = Vec::new();

        for by_rank in fetch_result.by_rank {
            if let Some(east) = by_rank.east {
                rikishi_list.push(convert_basho_rikishi(east));
            }
            if let Some(west) = by_rank.west {
                rikishi_list.push(convert_basho_rikishi(west));
            }
        }

        Ok(rikishi_list)
    }

    /// Get current leaderboard
    async fn leaderboard(
        &self,
        ctx: &Context<'_>,
        basho_id: Option<String>,
    ) -> Result<Vec<LeaderboardEntry>> {
        let db = ctx.data::<DbConn>()?;
        let conn = db.lock().unwrap();

        let target_basho = if let Some(bid_str) = basho_id {
            bid_str
                .parse()
                .map_err(|_| Error::new("Invalid basho ID format"))?
        } else {
            BashoInfo::current_or_next_basho_id(&conn)?
        };

        // For now, return basic leaderboard - could be enhanced with actual Leaders implementation
        let players = DataPlayer::list_all(&conn, target_basho)?;
        let mut entries = Vec::new();

        for (rank, player) in players.into_iter().enumerate() {
            entries.push(LeaderboardEntry {
                player: Player {
                    id: player.id,
                    name: player.name.clone(),
                    join_date: player.join_date,
                    emperors_cups: player.emperors_cups,
                    rank: player.rank.map(|r| r.to_string()),
                    has_emperors_cup: player.has_emperors_cup(),
                    url_path: player.url_path(),
                },
                score: 0, // This would need actual score calculation
                rank: (rank + 1) as u16,
            });
        }

        Ok(entries)
    }
}

fn convert_basho_rikishi(rikishi: DataBashoRikishi) -> Rikishi {
    Rikishi {
        id: rikishi.id,
        name: rikishi.name,
        rank: rikishi.rank.to_string(),
        results: rikishi.results.to_vec(),
        wins: rikishi.wins,
        losses: rikishi.losses,
        picks: rikishi.picks,
        is_kyujyo: rikishi.is_kyujyo,
    }
}

pub fn create_schema(db: DbConn) -> GraphQLSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(db)
        .finish()
}
