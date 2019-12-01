CREATE TABLE basho_result (
    basho_id        INTEGER NOT NULL REFERENCES basho(id) ON DELETE CASCADE,
    player_id       INTEGER NOT NULL REFERENCES player(id) ON DELETE CASCADE,
    wins            INTEGER NOT NULL,
    rank            INTEGER NOT NULL,

    PRIMARY KEY (basho_id, player_id)
);

CREATE INDEX basho_result__player_id ON basho_result (player_id);

CREATE TABLE external_basho_result (
    basho_id        INTEGER PRIMARY KEY REFERENCES basho(id) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    players         INTEGER NOT NULL,
    winning_score   INTEGER
);

#ALTER TABLE basho DROP COLUMN external_link;
UPDATE basho SET external_link = NULL;

INSERT INTO external_basho_result (basho_id, url, players, winning_score)
VALUES
    (201901, 'https://docs.google.com/spreadsheets/d/e/2PACX-1vQPpo_GmEGc9ExoV4ayt2esIdyLFY__gtlkAC22AbM7wjy0B0py9Fo3dhjCi67zzU_rFYcQT0f56tC3/pubhtml', 60, 49),
    (201903, 'https://docs.google.com/spreadsheets/d/e/2PACX-1vSDRWK98rhwv75SMLN2mGrFSHIMv2rVi2_-PpQz20C1uTqdoC2vZ_RSbWA7MqY8APKdV4gwsb7YZhDB/pubhtml', 100, 61),
    (201905, 'https://docs.google.com/spreadsheets/d/1XQcTNX3GWCxd6mueQUd8uGb7BoPV9Q9k5RLExr4uzlY/pubhtml', 122, 48),
    (201907, 'https://docs.google.com/spreadsheets/d/e/2PACX-1vR2w4tx6BYrwAGiBA9k45g9tfk_-IpwWNDI94luffWbi2dyGPs602TG80OGdkuKARJNsHTK-N7mu-hW/pubhtml', 108, 49),
    (201909, 'https://docs.google.com/spreadsheets/d/e/2PACX-1vThU4fGcK22acfjvMo5e2zq8Ycfm8GpaIN2y_DVuMvKD9KTT5TAJWloSVMVSV5CyFXNm-E16rqtxJnJ/pubhtml', 117, 48);
