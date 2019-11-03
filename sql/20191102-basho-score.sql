CREATE VIEW basho_score (basho_id, player_id, wins)
AS SELECT
    pick.basho_id,
    pick.player_id,
    COALESCE(SUM(torikumi.win), 0) AS wins
FROM pick
LEFT JOIN torikumi
    ON torikumi.rikishi_id = pick.rikishi_id
    AND torikumi.basho_id = pick.basho_id
GROUP BY pick.basho_id, pick.player_id;
