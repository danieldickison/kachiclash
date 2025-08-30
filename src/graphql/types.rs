use crate::data::{PlayerId, RikishiId};
use async_graphql::*;
use chrono::{DateTime, Utc};

/// A player in the game
#[derive(SimpleObject)]
pub struct Player {
    /// Unique player ID
    pub id: PlayerId,
    /// Player's display name
    pub name: String,
    /// When the player joined
    pub join_date: DateTime<Utc>,
    /// Number of Emperor's Cups won
    pub emperors_cups: u8,
    /// Current rank (if any)
    pub rank: Option<String>,
    /// Whether player has won an Emperor's Cup
    pub has_emperors_cup: bool,
    /// URL path for player profile
    pub url_path: String,
}

/// A basho (tournament)
#[derive(SimpleObject)]
pub struct Basho {
    /// Basho ID (e.g., "202401" for January 2024)
    pub id: String,
    /// When the basho starts
    pub start_date: DateTime<Utc>,
    /// Venue name
    pub venue: String,
    /// External link (if available)
    pub external_link: Option<String>,
    /// Number of participating players
    pub player_count: usize,
    /// Winning score
    pub winning_score: Option<u8>,
    /// Whether the basho has started
    pub has_started: bool,
}

/// A rikishi (sumo wrestler) with records for a particular basho
#[derive(SimpleObject)]
pub struct Rikishi {
    /// Unique rikishi ID
    pub id: RikishiId,
    /// Wrestling name
    pub name: String,
    /// Rank in the tournament
    pub rank: String,
    /// Daily results (true=win, false=loss, null=kyujyo or fusen loss)
    pub results: Vec<Option<bool>>,
    /// Total wins
    pub wins: u8,
    /// Total losses
    pub losses: u8,
    /// Number of picks by players
    pub picks: u16,
    /// Whether this rikishi is kyujo (absent) at the start of the basho
    pub is_kyujyo: bool,
}

/// A player's picks for a basho
#[derive(SimpleObject)]
pub struct PlayerPicks {
    /// The player
    pub player: Player,
    /// The basho
    pub basho: Basho,
    /// Picked rikishi (5 wrestlers, one from each rank group)
    pub rikishi: Vec<Option<Rikishi>>,
}

/// A player's score for a basho
#[derive(SimpleObject)]
pub struct PlayerScore {
    /// The player
    pub player: Player,
    /// The basho
    pub basho: Basho,
    /// Player's rank before this basho
    pub rank: Option<String>,
    /// Picked rikishi with their performance
    pub rikishi: Vec<Option<PlayerBashoRikishi>>,
    /// Total wins achieved
    pub wins: Option<u8>,
    /// Final ranking/place
    pub place: Option<u16>,
    /// Awards earned
    pub awards: Vec<Award>,
}

/// A rikishi picked by a player with their performance
#[derive(SimpleObject)]
pub struct PlayerBashoRikishi {
    /// Rikishi name
    pub name: String,
    /// Wins achieved
    pub wins: u8,
    /// Losses incurred
    pub losses: u8,
}

/// An award earned by a player
#[derive(SimpleObject)]
pub struct Award {
    /// Award type
    pub award_type: String,
    /// Human-readable name
    pub name: String,
}

/// Leaderboard entry
#[derive(SimpleObject)]
pub struct LeaderboardEntry {
    /// Player
    pub player: Player,
    /// Current score/wins
    pub score: u8,
    /// Current rank/position
    pub rank: u16,
}

/// Basho results summary
#[derive(SimpleObject)]
pub struct BashoResults {
    /// The basho
    pub basho: Basho,
    /// Winners of the basho
    pub winners: Vec<Player>,
    /// Winning score
    pub winning_score: Option<u8>,
    /// All player scores
    pub player_scores: Vec<PlayerScore>,
}

/// Input for filtering bashos
#[derive(InputObject)]
pub struct BashoFilter {
    /// Filter by specific basho ID
    pub id: Option<String>,
    /// Only include completed bashos
    pub completed_only: Option<bool>,
    /// Limit number of results
    pub limit: Option<i32>,
}

/// Input for filtering players
#[derive(InputObject)]
pub struct PlayerFilter {
    /// Filter by player name
    pub name: Option<String>,
    /// Filter by player ID
    pub id: Option<PlayerId>,
    /// Only include players with Emperor's Cups
    pub has_emperors_cup: Option<bool>,
    /// Limit number of results
    pub limit: Option<i32>,
}
