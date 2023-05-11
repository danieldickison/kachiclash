BEGIN;
  -- delete old basho data that conflict
  DELETE FROM basho_result WHERE player_id = 901 AND basho_id = 202109;
  DELETE FROM pick WHERE player_id = 901 AND basho_id = 202109;
  
  -- move the "from" player data to the "to" player data.
  UPDATE OR ROLLBACK pick SET player_id = 901 WHERE player_id = 950;
  UPDATE OR ROLLBACK award SET player_id = 901 WHERE player_id = 950;
  UPDATE OR ROLLBACK basho_result SET player_id = 901 WHERE player_id = 950;
COMMIT;
