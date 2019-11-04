CREATE UNIQUE INDEX player__name ON player (name);

CREATE TABLE player_google (
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    id              TEXT NOT NULL UNIQUE,
    name            TEXT,
    picture         TEXT,
    mod_date        TEXT NOT NULL
);

CREATE INDEX player_google__player_id ON player_google (player_id);
