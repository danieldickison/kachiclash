-- Denormalized player rank based on past 6 bashos for efficient display on various pages.
CREATE TABLE player_rank (
    player_id       INTEGER PRIMARY KEY REFERENCES player(id) ON DELETE CASCADE,
    rank            TEXT NOT NULL,
    past_year_wins  INTEGER NOT NULL
);

DROP VIEW player_info;
CREATE VIEW player_info (
    id, name, join_date, admin_level,
    rank, past_year_wins,
    discord_user_id, discord_avatar, discord_discriminator,
    google_picture,
    reddit_icon,
    emperors_cups
)
AS SELECT
    p.id, p.name, p.join_date, p.admin_level,
    rank.rank, COALESCE(rank.past_year_wins, 0),
    d.user_id, d.avatar, d.discriminator,
    g.picture,
    r.icon_img,
    (
        SELECT COUNT(*)
        FROM award AS a
        WHERE a.player_id = p.id AND type = 1
    ) AS emperors_cups
FROM player AS p
LEFT JOIN player_rank AS rank ON rank.player_id = p.id
LEFT JOIN player_discord AS d ON d.player_id = p.id
LEFT JOIN player_google AS g ON g.player_id = p.id
LEFT JOIN player_reddit AS r ON r.player_id = p.id;
