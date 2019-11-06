CREATE TABLE player_reddit (
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    id              TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    icon_img        TEXT,
    mod_date        TEXT NOT NULL
);

CREATE INDEX player_reddit__player_id ON player_reddit (player_id);


DROP VIEW player_info;

CREATE VIEW player_info (
    id, name, join_date, admin_level,
    discord_user_id, discord_avatar, discord_discriminator,
    google_picture,
    reddit_icon,
    emperors_cups
)
AS SELECT
    p.id, p.name, p.join_date, p.admin_level,
    d.user_id, d.avatar, d.discriminator,
    g.picture,
    r.icon_img,
    (
        SELECT COUNT(*)
        FROM award AS a
        WHERE a.player_id = p.id AND type = 1
    ) AS emperors_cups
FROM player AS p
LEFT JOIN player_discord AS d ON d.player_id = p.id
LEFT JOIN player_google AS g ON g.player_id = p.id
LEFT JOIN player_reddit AS r ON r.player_id = p.id;
