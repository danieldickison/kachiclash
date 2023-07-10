BEGIN;
  -- delete old basho data that conflict
  DELETE FROM basho_result WHERE player_id = 121 AND basho_id = 202305;
  DELETE FROM pick WHERE player_id = 121 AND basho_id = 202305;
  
  -- move the "from" player data to the "to" player data.
  UPDATE OR ROLLBACK pick SET player_id = 121 WHERE player_id = 2243 AND basho_id = 202305;
  UPDATE OR ROLLBACK award SET player_id = 121 WHERE player_id = 2243 AND basho_id = 202305;
  UPDATE OR ROLLBACK basho_result SET player_id = 121 WHERE player_id = 2243 AND basho_id = 202305;
COMMIT;
