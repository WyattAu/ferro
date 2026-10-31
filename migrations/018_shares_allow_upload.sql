-- Shares: allow_upload was written by ShareStore::persist_create's extended
-- schema probe but never added by a migration, so load_all_from_db's extended
-- SELECT failed with "no such column: allow_upload" and every stored share
-- was silently dropped on restart.
ALTER TABLE shares ADD COLUMN allow_upload INTEGER;
