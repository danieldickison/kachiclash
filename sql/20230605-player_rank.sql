-- Denormalized player rank based on past 6 bashos for efficient display on various pages.
CREATE TABLE player_rank (
    before_basho_id INTEGER NOT NULL REFERENCES basho(id),
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    rank            TEXT NOT NULL,
    past_year_wins  INTEGER NOT NULL,
    
    PRIMARY KEY (before_basho_id, player_id)
);
