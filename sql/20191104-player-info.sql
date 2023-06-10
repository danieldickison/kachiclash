-- DEPRECATED; see new version in 20191106 migration

CREATE VIEW player_info (
    id, name, join_date, admin_level,
    discord_user_id, discord_avatar, discord_discriminator,
    google_picture,
    emperors_cups
)
AS SELECT
    p.id, p.name, p.join_date, p.admin_level,
    d.user_id, d.avatar, d.discriminator,
    g.picture,
    (
        SELECT COUNT(*)
        FROM award AS a
        WHERE a.player_id = p.id AND type = 1
    ) AS emperors_cups
FROM player AS p
LEFT JOIN player_discord AS d ON d.player_id = p.id
LEFT JOIN player_google AS g ON g.player_id = p.id;
