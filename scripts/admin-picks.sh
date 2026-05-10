#!/usr/bin/env bash
# admin-picks.sh — Set picks for a player, bypassing the basho-started guard.
# Looks up all IDs by name so you never have to touch the db directly.
#
# Usage: admin-picks.sh <db_path> <basho_id> <player_name> <rikishi1> [rikishi2..5]
set -euo pipefail

SCRIPT=$(basename "$0")

die()  { echo "Error: $*" >&2; exit 1; }
info() { printf "   %s\n" "$*"; }

usage() {
    cat >&2 <<EOF
Usage: $SCRIPT <db_path> <basho_id> <player_name> <rikishi1> <rikishi2> <rikishi3> <rikishi4> <rikishi5>

  db_path      Path to the SQLite database file
  basho_id     Basho ID in YYYYMM format (e.g. 202505)
  player_name  Player's display name (case-insensitive)
  rikishi1..5  Shikona of each pick in any order (exactly 5 required)

Example:
  $SCRIPT var/kachiclash.db 202505 CoolPlayer Kotonowaka Hoshoryu Daieisho Kirishima Onosato
EOF
    exit 1
}

# ── dependencies ──────────────────────────────────────────────────────────────
command -v sqlite3 &>/dev/null || die "sqlite3 not found in PATH"

# ── arguments ─────────────────────────────────────────────────────────────────
[ $# -ne 8 ] && usage

DB="$1"; BASHO_ID="$2"; PLAYER_NAME="$3"
shift 3

RIKISHI=("$@")
PICK_COUNT=${#RIKISHI[@]}

[ -f "$DB" ] || die "database file not found: $DB"

# Run a SQL statement against the db with tab-separated output
q() { sqlite3 -separator $'\t' "$DB" "$@"; }

# Escape single quotes for SQL string literals
sq() { local v="$1"; echo "${v//\'/\'\'}"; }

# ── validate basho ────────────────────────────────────────────────────────────
BASHO_DATE=$(q "SELECT start_date FROM basho WHERE id = $BASHO_ID;")
[ -z "$BASHO_DATE" ] && die "basho not found: $BASHO_ID"

# ── look up player ────────────────────────────────────────────────────────────
PLAYER_ROW=$(q "SELECT id, name FROM player WHERE name = '$(sq "$PLAYER_NAME")' COLLATE NOCASE;")
[ -z "$PLAYER_ROW" ] && die "player not found: $PLAYER_NAME"
PLAYER_COUNT=$(echo "$PLAYER_ROW" | wc -l | tr -d ' ')
[ "$PLAYER_COUNT" -gt 1 ] && die "multiple players matched '$PLAYER_NAME'; be more specific"
PLAYER_ID=$(echo "$PLAYER_ROW" | cut -f1)
PLAYER_DISPLAY=$(echo "$PLAYER_ROW" | cut -f2)

# ── rank-group SQL expression ─────────────────────────────────────────────────
# Mirrors RankGroup::for_rank() in src/data/rank.rs.
# Ranks are stored in short form: Y1E, O2W, S1E, K1W, M5E, M14W, J3E, Ms3E, …
RANK_GROUP_SQL="CASE
    WHEN SUBSTR(rank,1,1) IN ('Y','O')                                                             THEN 1
    WHEN rank GLOB 'S[0-9]*' OR rank GLOB 'K[0-9]*'                                               THEN 2
    WHEN rank GLOB 'M[0-9]*' AND CAST(SUBSTR(rank,2,LENGTH(rank)-2) AS INTEGER) BETWEEN 1 AND 5   THEN 3
    WHEN rank GLOB 'M[0-9]*' AND CAST(SUBSTR(rank,2,LENGTH(rank)-2) AS INTEGER) BETWEEN 6 AND 10  THEN 4
    WHEN rank GLOB 'M[0-9]*' AND CAST(SUBSTR(rank,2,LENGTH(rank)-2) AS INTEGER) >= 11             THEN 5
    WHEN rank GLOB 'J[0-9]*'                                                                       THEN 6
    ELSE 7
END"

# ── look up each rikishi in the banzuke ───────────────────────────────────────
PICK_ROWS=()
PICK_GROUPS=()

for name in "${RIKISHI[@]}"; do
    ROW=$(q "
        SELECT rikishi_id, family_name, rank, ($RANK_GROUP_SQL) AS rank_group
        FROM banzuke
        WHERE basho_id = $BASHO_ID
          AND family_name = '$(sq "$name")' COLLATE NOCASE;")
    [ -z "$ROW" ] && die "rikishi '$name' not found in banzuke for basho $BASHO_ID"
    GROUP=$(echo "$ROW" | cut -f4)
    [ "$GROUP" -ge 6 ] && {
        RANK=$(echo "$ROW" | cut -f3)
        die "$name ($RANK) is Juryo or below — only Makuuchi picks are valid"
    }
    PICK_ROWS+=("$ROW")
    PICK_GROUPS+=("$GROUP")
done

# ── validate that rank groups are unique ──────────────────────────────────────
UNIQUE_GROUPS=$(printf '%s\n' "${PICK_GROUPS[@]}" | sort -u | wc -l | tr -d ' ')
if [ "$UNIQUE_GROUPS" -lt "$PICK_COUNT" ]; then
    die "picks contain duplicate rank groups — each pick must come from a different tier:
  Group 1 — Yokozuna / Ozeki
  Group 2 — Sekiwake / Komusubi
  Group 3 — Maegashira 1–5
  Group 4 — Maegashira 6–10
  Group 5 — Maegashira 11+"
fi

# ── preview ───────────────────────────────────────────────────────────────────
echo
info "Player : $PLAYER_DISPLAY  (id $PLAYER_ID)"
info "Basho  : $BASHO_ID  (starts $BASHO_DATE)"
info "Picks  :"
for row in "${PICK_ROWS[@]}"; do
    IFS=$'\t' read -r rid name rank group <<< "$row"
    printf "     %-20s  %-6s  (group %s)\n" "$name" "$rank" "$group"
done

# Abort if the player already has any picks for this basho
EXISTING=$(q "
    SELECT b.family_name, b.rank
    FROM pick AS p
    JOIN banzuke AS b ON b.rikishi_id = p.rikishi_id AND b.basho_id = p.basho_id
    WHERE p.player_id = $PLAYER_ID AND p.basho_id = $BASHO_ID
    ORDER BY ($RANK_GROUP_SQL);")
if [ -n "$EXISTING" ]; then
    echo
    info "$PLAYER_DISPLAY already has picks for basho $BASHO_ID:"
    while IFS=$'\t' read -r name rank; do
        printf "     %-20s  %s\n" "$name" "$rank"
    done <<< "$EXISTING"
    echo
    die "aborting — delete their existing picks first if you really want to replace them"
fi

echo

# ── confirmation ──────────────────────────────────────────────────────────────
read -r -p "Proceed? [y/N] " CONFIRM < /dev/tty
[[ "$CONFIRM" =~ ^[Yy]$ ]] || { echo "Aborted."; exit 0; }

# ── execute ───────────────────────────────────────────────────────────────────
{
    echo "BEGIN;"
    for row in "${PICK_ROWS[@]}"; do
        rid=$(echo "$row" | cut -f1)
        echo "INSERT INTO pick (player_id, basho_id, rikishi_id) VALUES ($PLAYER_ID, $BASHO_ID, $rid);"
    done
    echo "COMMIT;"
} | sqlite3 "$DB"

# ── verify ────────────────────────────────────────────────────────────────────
SAVED=$(q "SELECT COUNT(*) FROM pick WHERE player_id = $PLAYER_ID AND basho_id = $BASHO_ID;")
echo
echo "Done — $SAVED pick(s) saved for $PLAYER_DISPLAY in basho $BASHO_ID."
