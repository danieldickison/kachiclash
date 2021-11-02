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
    let first_leaders_basho = nth_completed_basho_id(&basho_list, leader_basho_count);
    let leaders = HistoricLeader::with_first_basho(&db, first_leaders_basho, LEADERS_LIMIT)?;
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

fn nth_completed_basho_id(basho_list: &[BashoInfo], n: usize) -> Option<BashoId> {
    if basho_list.is_empty() { return None; }

    let mut n = n;
    if basho_list.first().unwrap().winners.is_empty() {
        n += 1;
    }
    if n >= basho_list.len() {
        n = basho_list.len() - 1;
    }
    basho_list.get(n).map(|b| b.id)
}
