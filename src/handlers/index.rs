use super::{BaseTemplate, IdentityExt, Result};
use crate::data::leaders::PlayerRanking;
use crate::data::{BashoId, BashoInfo, Rank};
use crate::util::GroupRuns;
use crate::AppState;
use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{get, route, web, HttpResponse};
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplate,
    leaders: Vec<PlayerRanking>,
    self_leader_index: Option<usize>,
    current_basho: Option<BashoInfo>,
    prev_basho: Option<BashoInfo>,
    next_basho_id: BashoId,
    hero_img_src: String,
}

impl IndexTemplate {
    fn leaders_by_rank(&self) -> Vec<(Rank, usize, u32, &[PlayerRanking])> {
        self.leaders
            .group_runs(|a, b| a.rank == b.rank)
            .map(|group| {
                let first = group.first().unwrap();
                (first.rank, first.ord, first.wins, group)
            })
            .collect()
    }

    fn self_leader(&self) -> Option<&PlayerRanking> {
        self.self_leader_index.and_then(|i| self.leaders.get(i))
    }

    fn is_self(&self, leader: &PlayerRanking) -> bool {
        if let Some(self_leader) = self.self_leader() {
            self_leader.player.id == leader.player.id
        } else {
            false
        }
    }
}

#[route("/", method = "GET", method = "HEAD")]
pub async fn index(
    state: web::Data<AppState>,
    identity: Option<Identity>,
) -> Result<IndexTemplate> {
    let db = state.db.lock().unwrap();
    let (current_basho, prev_basho) = BashoInfo::current_and_previous(&db)?;
    let next_basho_id = prev_basho
        .as_ref()
        .map(|basho| basho.id.next())
        .unwrap_or_else(|| "201911".parse().unwrap());
    let leaders = PlayerRanking::for_home_page(&db, next_basho_id)?;
    let self_leader_index = match identity.as_ref() {
        Some(id) => {
            let player_id = id.player_id()?;
            leaders.iter().position(|l| l.player.id == player_id)
        }
        None => None,
    };
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, identity.as_ref(), &state)?,
        leaders,
        self_leader_index,
        current_basho,
        prev_basho,
        next_basho_id,
        hero_img_src: state.config.hero_img_src.to_owned(),
    })
}

#[get("/pwa")]
pub async fn pwa(
    state: web::Data<AppState>,
    query: web::Query<Vec<(String, String)>>,
) -> Result<HttpResponse> {
    let db = state.db.lock().unwrap();
    let (current_basho, _) = BashoInfo::current_and_previous(&db)?;

    let mut page = state.config.url();
    page.query_pairs_mut().extend_pairs(query.iter());
    if let Some(basho) = current_basho {
        page.set_path(&basho.link_url());
    }

    Ok(HttpResponse::TemporaryRedirect()
        .insert_header((LOCATION, page.to_string()))
        .finish())
}
