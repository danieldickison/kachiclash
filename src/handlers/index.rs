use crate::AppState;
use crate::data::{BashoId, BashoInfo, Rank};
use crate::data::leaders::HistoricLeader;
use crate::util::GroupRuns;
use super::{BaseTemplate, IdentityExt, Result};
use actix_web::web;
use actix_identity::Identity;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplate,
    leaders: Vec<HistoricLeader>,
    self_leader_index: Option<usize>,
    current_basho: Option<BashoInfo>,
    prev_basho: Option<BashoInfo>,
    next_basho_id: BashoId,
}

impl IndexTemplate {
    fn leaders_by_rank(&self)
    -> Vec<(Rank, &[HistoricLeader])> {
        self.leaders
        .group_runs(|a, b| a.rank == b.rank)
        .map(|group| (group.first().unwrap().rank, group))
        .collect()
    }

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

const LEADER_BASHO_COUNT_OPTIONS: [usize; 3] = [6, 3, 2];
const LEADERS_LIMIT: u32 = 270;

pub async fn index(state: web::Data<AppState>, identity: Identity) -> Result<IndexTemplate> {
    let db = state.db.lock().unwrap();
    let (current_basho, prev_basho) = BashoInfo::current_and_previous(&db)?;
    let next_basho_id =
        current_basho.as_ref().or_else(|| prev_basho.as_ref())
        .map(|basho| basho.id.next())
        .unwrap_or_else(|| "201911".parse().unwrap());
    let first_leaders_basho = prev_basho.as_ref().map(|basho| basho.id.incr(-5));
    let leaders = HistoricLeader::with_first_basho(&db, first_leaders_basho, LEADERS_LIMIT)?;
    let self_leader_index = match identity.player_id() {
        Some(id) => leaders.iter().position(|l| l.player.id == id),
        None => None
    };
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders,
        self_leader_index,
        current_basho,
        prev_basho,
        next_basho_id
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
