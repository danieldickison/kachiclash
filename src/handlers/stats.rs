use std::ops::Range;

use crate::AppState;
use crate::data::{BashoInfo, BashoId};
use crate::data::leaders::HistoricLeader;
use super::{BaseTemplate, IdentityExt, Result};
use actix_web::web;
use actix_identity::Identity;
use askama::Template;

#[derive(Template)]
#[template(path = "stats.html")]
pub struct StatsTemplate {
    base: BaseTemplate,
    basho_list: Vec<BashoInfo>,
    leader_basho_count: usize,
    leader_basho_count_options: Vec<usize>,
    leaders: Vec<HistoricLeader>,
    self_leader_index: Option<usize>,
}

impl StatsTemplate {
    fn self_leader(&self) -> Option<&HistoricLeader> {
        self.self_leader_index.and_then(|i| self.leaders.get(i))
    }

    fn is_self(&self, leader: &HistoricLeader) -> bool {
        if let Some(self_leader) = self.self_leader() {
            self_leader.player.id == leader.player.id
        } else {
            false
        }
    }
}

#[derive(Deserialize)]
pub struct QueryParams {
    b: Option<usize>
}

const LEADER_BASHO_COUNT_OPTIONS: [usize; 3] = [6, 3, 2];
const LEADERS_LIMIT: u32 = 500;

pub async fn stats_page(query: web::Query<QueryParams>, state: web::Data<AppState>, identity: Identity) -> Result<StatsTemplate> {
    let db = state.db.lock().unwrap();
    let basho_list = BashoInfo::list_all(&db)?;
    let leader_basho_count = query.b.unwrap_or(6);
    let basho_range = n_completed_basho(&basho_list, leader_basho_count);
    let leaders = HistoricLeader::with_basho_range(&db, basho_range, LEADERS_LIMIT)?;
    let self_leader_index = match identity.player_id() {
        Some(id) => leaders.iter().position(|l| l.player.id == id),
        None => None
    };
    Ok(StatsTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        basho_list,
        leader_basho_count,
        leader_basho_count_options: LEADER_BASHO_COUNT_OPTIONS.iter().copied()
            .filter(|c| *c != leader_basho_count)
            .collect(),
        leaders,
        self_leader_index,
    })
}

fn n_completed_basho(basho_list: &[BashoInfo], n: usize) -> Range<BashoId> {
    if basho_list.is_empty() {
        return Range {start: "201901".parse().unwrap(), end: "202001".parse().unwrap()};
    }

    let first = basho_list.first().unwrap();
    let end;
    if first.winners.is_empty() {
        end = first.id;
    } else {
        end = first.id.incr(1);
    }
    Range {end, start: end.incr(-(n as isize))}
}
