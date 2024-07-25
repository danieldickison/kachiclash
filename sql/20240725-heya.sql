CREATE TABLE heya (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    slug            TEXT NOT NULL UNIQUE,
    owner_player_id INTEGER NOT NULL REFERENCES player(id) ON DELETE RESTRICT
);

CREATE TABLE heya_player (
    player_id   INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    heya_id     INTEGER NOT NULL REFERENCES heya(id) ON DELETE CASCADE,
    PRIMARY KEY (player_id, heya_id) ON CONFLICT IGNORE
);
CREATE INDEX heya_player__heya_id on heya_player (heya_id);
