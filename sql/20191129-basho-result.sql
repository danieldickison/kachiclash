CREATE TABLE basho_result (
    basho_id        INTEGER NOT NULL REFERENCES basho(id) ON DELETE CASCADE,
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    wins            INTEGER NOT NULL,
    rank            INTEGER NOT NULL,

    PRIMARY KEY (basho_id, player_id)
);

CREATE INDEX basho_result__player_id ON basho_result (player_id);
