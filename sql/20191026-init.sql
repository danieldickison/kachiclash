CREATE TABLE basho (
    id              INTEGER PRIMARY KEY,
    start_date      TEXT NOT NULL,
    venue           TEXT NOT NULL
);

CREATE TABLE rikishi (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    family_name     TEXT NOT NULL,
    given_name      TEXT NOT NULL
);

CREATE INDEX rikishi__family_name ON rikishi (family_name);

CREATE TABLE banzuke (
    rikishi_id      INTEGER NOT NULL REFERENCES rikishi(id) ON DELETE CASCADE,
    basho_id        INTEGER NOT NULL REFERENCES basho(id) ON DELETE CASCADE,
    family_name     TEXT NOT NULL,
    given_name      TEXT NOT NULL,
    rank            TEXT NOT NULL,

    PRIMARY KEY (rikishi_id, basho_id)
);

CREATE INDEX banzuke__basho_id ON banzuke (basho_id);

CREATE TABLE torikumi (
    basho_id        INTEGER NOT NULL REFERENCES basho(id) ON DELETE CASCADE,
    day             INTEGER NOT NULL,
    seq             INTEGER NOT NULL,
    side            TEXT NOT NULL,
    rikishi_id      INTEGER NOT NULL,
    win             INTEGER,

    PRIMARY KEY (basho_id, day, seq, side),
    FOREIGN KEY (rikishi_id, basho_id) REFERENCES banzuke(rikishi_id, basho_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX torikumi__rikishi_day ON torikumi (rikishi_id, basho_id, day);

CREATE TABLE player (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    join_date       TEXT NOT NULL,
    name            TEXT NOT NULL,
    admin_level     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE player_discord (
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL UNIQUE,
    username        TEXT NOT NULL,
    avatar          TEXT,
    discriminator   TEXT NOT NULL,
    mod_date        TEXT NOT NULL
);

CREATE INDEX player_discord__player_id ON player_discord (player_id);

CREATE TABLE pick (
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    basho_id        INTEGER NOT NULL REFERENCES basho(id) ON DELETE CASCADE,
    rikishi_id      INTEGER NOT NULL REFERENCES rikishi(id) ON DELETE CASCADE,

    PRIMARY KEY (player_id, basho_id, rikishi_id)
);

CREATE INDEX pick__basho_id ON pick (basho_id);

CREATE TABLE award (
    basho_id        INTEGER NOT NULL REFERENCES basho(id) ON DELETE CASCADE,
    type            INTEGER NOT NULL,
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,

    PRIMARY KEY (basho_id, type, player_id)
);

CREATE INDEX award__player_id ON award (player_id);
