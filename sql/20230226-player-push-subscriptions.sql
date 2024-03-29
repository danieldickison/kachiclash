CREATE TABLE player_push_subscriptions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    info_json       TEXT NOT NULL UNIQUE,
    user_agent      TEXT NOT NULL,
    opt_in_json     TEXT NOT NULL,
    create_date     INTEGER NOT NULL DEFAULT (cast(strftime('%s', 'now') AS INTEGER))
);
