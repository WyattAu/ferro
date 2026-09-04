# OCIS Space Recovery Tooling

Recovers files from an orphaned oCIS decomposedfs space (e.g. after a Keycloak
user re-provision changed the OCIS user UUID and re-minted an empty personal space).

## 1. Recover blobs + metadata -> staging tree

Run on the host with access to the OCIS data dir (TrueNAS: appdata/storage/ocis-data):

    sudo python3 ocis_decomposedfs_recover.py

Edit SPACE / STAGE / ROOT_ID constants at the top first. Layout notes:
- nodes/<4 hex buckets>/-<uuid-rest>[.mpk] = node metadata (msgpack, key prefix user.ocis.*)
- container nodes have NO id key; id = bucket chars + filename (sans .mpk)
- file nodes carry id/blobid in metadata; blob = blobs/<uuid[:8] split into
  2-char dirs>/-<uuid[9:]>
- type '1' = file, '2' = container

## 2. Upload staging tree into Ferro

    python3 upload_to_ferro.py

Uploads via WebDAV PUT with ROPC token auto-refresh every 240s (Keycloak
password grant). Files land under /users/<sub>/ automatically.

## Status of the 2026-09 recovery
- 12,935 files / 13.31 GB recovered of 13,219 nodes; 222 unrecoverable
  (Books/Calibre mount blobs 404-ing inside OCIS itself + old version blobs)
- Orphaned space backed up at sis/backups/ocis-orphan-space-222dad5a/
