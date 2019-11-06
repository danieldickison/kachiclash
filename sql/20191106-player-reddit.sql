CREATE TABLE player_reddit (
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    id              TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    icon_img        TEXT,
    mod_date        TEXT NOT NULL
);

CREATE INDEX player_reddit__player_id ON player_reddit (player_id);
