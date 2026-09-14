//! Implementation of the `enter-picks` binary: manually enter a player's picks
//! for a basho, bypassing the pick deadline enforced by the web app.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context};
use chrono::Local;
use rusqlite::{Connection, Row};

use crate::data::{
    self, basho::force_save_player_picks, BashoId, BashoInfo, Player, PlayerId, Rank, RikishiId,
};
use crate::DEFAULT_DB_PATH;

const USAGE: &str = "\
Usage: enter-picks [options] <player> <basho> <rikishi1> .. <rikishi5>

Manually enter a player's picks for a basho, bypassing the pick deadline that
applies to the web app. Picks are validated the way the web app validates them,
and nothing is written until you confirm.

Arguments:
  <player>        player name (case-insensitive)
  <basho>         basho id in YYYYMM form, e.g. 202511
  <rikishi1..5>   the five picks, as shikona (case-insensitive), in any order

Options:
  --db <path>     sqlite db to update
                  [default: $KACHI_DB_PATH, or var/kachiclash.sqlite]
  -y, --yes       save without asking for confirmation
  -h, --help      print this message

Example:
  enter-picks CoolPlayer 202511 Onosato Wakatakakage Takanosho Atamifuji Asakoryu
";

pub fn run() -> anyhow::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "warn,kachiclash=info");
    }
    pretty_env_logger::init();

    let args = match Args::parse(std::env::args().skip(1))? {
        Some(args) => args,
        None => {
            print!("{USAGE}");
            return Ok(());
        }
    };

    if !args.db_path.is_file() {
        bail!(
            "no database at {}; pass --db PATH or set KACHI_DB_PATH",
            args.db_path.display()
        );
    }
    let db_conn = data::make_conn(&args.db_path);
    let mut db = db_conn.lock().unwrap();

    let basho = BashoInfo::with_id(&db, args.basho_id)?
        .ok_or_else(|| anyhow!("no basho {} in the database", args.basho_id.id()))?;
    if !basho.winners.is_empty() {
        bail!(
            "basho {} is already finalized; changing picks now would leave basho_result, awards, and player ranks stale",
            basho.id.id()
        );
    }
    let player = Player::with_name(&db, args.player_name.clone(), basho.id)?
        .ok_or_else(|| anyhow!("no player named {}", args.player_name))?;
    let picks = lookup_picks(&db, basho.id, &args.rikishi)?;
    let existing = existing_picks(&db, basho.id, player.id)?;

    print_summary(&basho, &player, &picks, &existing);

    if !args.assume_yes && !confirm()? {
        println!("Aborted; nothing was written.");
        return Ok(());
    }

    let pick_ids: [Option<RikishiId>; 5] = picks.map(|pick| Some(pick.id));
    force_save_player_picks(&mut db, player.id, basho.id, pick_ids)?;

    println!(
        "Saved picks for {} in basho {} and recomputed the basho results.",
        player.name,
        basho.id.id()
    );

    Ok(())
}

#[derive(Debug)]
struct Args {
    db_path: PathBuf,
    player_name: String,
    basho_id: BashoId,
    rikishi: [String; 5],
    assume_yes: bool,
}

impl Args {
    /// Returns `Ok(None)` if the caller asked for `--help`.
    fn parse(argv: impl Iterator<Item = String>) -> anyhow::Result<Option<Self>> {
        let mut argv = argv;
        let mut db_path: Option<PathBuf> = None;
        let mut assume_yes = false;
        let mut positional: Vec<String> = Vec::new();

        while let Some(arg) = argv.next() {
            match arg.as_str() {
                "-h" | "--help" => return Ok(None),
                "-y" | "--yes" => assume_yes = true,
                "--db" => {
                    db_path = Some(
                        argv.next()
                            .ok_or_else(|| anyhow!("--db needs a path"))?
                            .into(),
                    )
                }
                other if other.starts_with("--db=") => {
                    db_path = Some(other.trim_start_matches("--db=").into())
                }
                other if other.starts_with('-') => bail!("unknown option {other}\n\n{USAGE}"),
                _ => positional.push(arg),
            }
        }

        let mut positional = positional.into_iter();
        let player_name = positional
            .next()
            .ok_or_else(|| anyhow!("missing player name\n\n{USAGE}"))?;
        let basho_arg = positional
            .next()
            .ok_or_else(|| anyhow!("missing basho id\n\n{USAGE}"))?;
        let basho_id: BashoId = basho_arg
            .parse()
            .with_context(|| format!("invalid basho id {basho_arg}; expected YYYYMM"))?;
        let rikishi: [String; 5] =
            positional
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|picks: Vec<String>| {
                    anyhow!("expected 5 picks but got {}\n\n{USAGE}", picks.len())
                })?;

        Ok(Some(Self {
            db_path: db_path
                .or_else(|| std::env::var_os("KACHI_DB_PATH").map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from(DEFAULT_DB_PATH)),
            player_name,
            basho_id,
            rikishi,
            assume_yes,
        }))
    }
}

#[derive(Debug)]
struct Pick {
    id: RikishiId,
    name: String,
    rank: Rank,
    is_kyujyo: bool,
}

impl Pick {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("rikishi_id")?,
            name: row.get("family_name")?,
            rank: row.get("rank")?,
            is_kyujyo: row.get("kyujyo")?,
        })
    }
}

impl std::fmt::Display for Pick {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:16} {:6} (rikishi id {}){}",
            self.name,
            self.rank.to_string(),
            self.id,
            if self.is_kyujyo { "  KYUJYO!" } else { "" }
        )
    }
}

/// Resolves each shikona to a rikishi in this basho's banzuke, slotted by rank
/// group so the picks line up with what the web app would submit.
fn lookup_picks(
    db: &Connection,
    basho_id: BashoId,
    shikona: &[String; 5],
) -> anyhow::Result<[Pick; 5]> {
    let mut picks: [Option<Pick>; 5] = Default::default();
    for name in shikona {
        let pick = lookup_rikishi(db, basho_id, name)?;
        if !pick.rank.is_makuuchi() {
            bail!(
                "{} is ranked {} in basho {}; only makuuchi rikishi can be picked",
                pick.name,
                pick.rank,
                basho_id.id()
            );
        }
        let group = pick.rank.group();
        if let Some(other) = &picks[group.as_index()] {
            bail!(
                "{} ({}) and {} ({}) are both in rank group {}; each pick must come from a different group",
                other.name,
                other.rank,
                pick.name,
                pick.rank,
                group
            );
        }
        picks[group.as_index()] = Some(pick);
    }

    // Five makuuchi picks in five distinct rank groups fill every slot.
    Ok(picks.map(|pick| pick.expect("a pick in every rank group")))
}

fn lookup_rikishi(db: &Connection, basho_id: BashoId, name: &str) -> anyhow::Result<Pick> {
    let matches: Vec<Pick> = db
        .prepare(
            "
            SELECT rikishi_id, family_name, rank, kyujyo
            FROM banzuke
            WHERE basho_id = ? AND family_name = ? COLLATE NOCASE
            ",
        )?
        .query_map(params![basho_id, name], Pick::from_row)?
        .collect::<rusqlite::Result<_>>()?;

    match matches.len() {
        0 => Err(anyhow!(
            "no rikishi named {} in the banzuke for basho {}",
            name,
            basho_id.id()
        )),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => Err(anyhow!(
            "{} rikishi in basho {} are named {}",
            matches.len(),
            basho_id.id(),
            name
        )),
    }
}

fn existing_picks(
    db: &Connection,
    basho_id: BashoId,
    player_id: PlayerId,
) -> anyhow::Result<Vec<Pick>> {
    let mut picks: Vec<Pick> = db
        .prepare(
            "
            SELECT b.rikishi_id, b.family_name, b.rank, b.kyujyo
            FROM pick AS p
            JOIN banzuke AS b ON b.rikishi_id = p.rikishi_id AND b.basho_id = p.basho_id
            WHERE p.player_id = ? AND p.basho_id = ?
            ",
        )?
        .query_map(params![player_id, basho_id], Pick::from_row)?
        .collect::<rusqlite::Result<_>>()?;
    picks.sort_by_key(|pick| pick.rank);
    Ok(picks)
}

fn print_summary(basho: &BashoInfo, player: &Player, picks: &[Pick; 5], existing: &[Pick]) {
    let start_date = basho.start_date.with_timezone(&Local).format("%F %R %:z");
    println!();
    println!("  Player:  {} (player id {})", player.name, player.id);
    println!("  Basho:   {} ({})", basho.id, basho.id.id());
    if basho.has_started() {
        println!("  Started: {start_date} — past the pick deadline!");
    } else {
        println!("  Starts:  {start_date}");
    }

    if !existing.is_empty() {
        println!();
        println!(
            "  {} already has picks for this basho, which will be REPLACED:",
            player.name
        );
        for pick in existing {
            println!("    {pick}");
        }
    }

    println!();
    println!("  Picks to save:");
    for (index, pick) in picks.iter().enumerate() {
        println!("    group {}:  {}", index + 1, pick);
    }
    println!();
}

fn confirm() -> anyhow::Result<bool> {
    print!("Save these picks? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "Yes"))
}
