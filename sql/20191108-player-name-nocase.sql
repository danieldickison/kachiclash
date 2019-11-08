CREATE UNIQUE INDEX player__name_nocase ON player (name COLLATE NOCASE);
DROP INDEX player__name;
