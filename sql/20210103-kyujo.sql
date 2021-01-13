ALTER TABLE banzuke ADD COLUMN kyujyo INTEGER NOT NULL DEFAULT 0;

# update banzuke set kyujyo = 1 where basho_id = 202101 and family_name = 'Hakuho';
# update banzuke set kyujyo = 1 where basho_id = 202101 and family_name = 'Wakatakakage';
update banzuke set kyujyo = 1 where basho_id = 202101 and family_name = 'Chiyonokuni';
update banzuke set kyujyo = 1 where basho_id = 202101 and family_name = 'Chiyotairyu';
update banzuke set kyujyo = 1 where basho_id = 202101 and family_name = 'Chiyoshoma';
update banzuke set kyujyo = 1 where basho_id = 202101 and family_name = 'Kaisei';