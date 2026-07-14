use std::collections::HashMap;

use rusqlite::{Connection, params};

use crate::error::Result as MigrateResult;

#[derive(Debug, Clone)]
pub struct NcUser {
    pub uid: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub quota: i64,
    pub last_login: i64,
    pub backend: Option<String>,
    pub state: i64,
    pub home: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NcFileCache {
    pub file_id: i64,
    pub storage: i64,
    pub path: String,
    pub path_hash: String,
    pub parent: i64,
    pub name: String,
    pub mimetype: i64,
    pub mimepart: i64,
    pub size: i64,
    pub mtime: i64,
    pub storage_mtime: i64,
    pub etag: String,
    pub encrypted: i64,
    pub unencrypted_size: i64,
    pub mount_point: Option<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone)]
pub struct NcShare {
    pub id: i64,
    pub share_type: i64,
    pub share_with: Option<String>,
    pub uid_owner: String,
    pub uid_initiator: String,
    pub parent: Option<i64>,
    pub item_type: String,
    pub item_source: i64,
    pub file_source: i64,
    pub file_target: String,
    pub permissions: i64,
}

#[derive(Debug, Clone)]
pub struct NcSystemTag {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct NcTagMapping {
    pub object_id: i64,
    pub object_type: String,
    pub systemtag_id: i64,
}

#[derive(Debug, Clone)]
pub struct NcFavorite {
    pub object_id: i64,
    pub object_type: String,
    pub category_id: i64,
}

pub struct NextcloudDb {
    conn: Connection,
}

impl NextcloudDb {
    pub fn open(path: &str) -> MigrateResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA readonly=1;")?;
        Ok(Self { conn })
    }

    pub fn read_users(&self) -> MigrateResult<Vec<NcUser>> {
        let mut stmt = self
            .conn
            .prepare("SELECT uid, displayname, email, quota, last_login, backend, state, home FROM oc_accounts")?;
        let rows = stmt.query_map(params![], |row| {
            Ok(NcUser {
                uid: row.get(0)?,
                display_name: row.get(1)?,
                email: row.get(2)?,
                quota: row.get(3)?,
                last_login: row.get(4)?,
                backend: row.get(5)?,
                state: row.get(6)?,
                home: row.get(7)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn read_filecache(&self) -> MigrateResult<Vec<NcFileCache>> {
        let mut stmt = self.conn.prepare(
            "SELECT fileid, storage, path, path_hash, parent, name, mimetype, mimepart, \
             size, mtime, storage_mtime, etag, encrypted, unencrypted_size, mount_point \
             FROM oc_filecache",
        )?;
        let rows = stmt.query_map(params![], |row| {
            Ok(NcFileCache {
                file_id: row.get(0)?,
                storage: row.get(1)?,
                path: row.get(2)?,
                path_hash: row.get(3)?,
                parent: row.get(4)?,
                name: row.get(5)?,
                mimetype: row.get(6)?,
                mimepart: row.get(7)?,
                size: row.get(8)?,
                mtime: row.get(9)?,
                storage_mtime: row.get(10)?,
                etag: row.get(11)?,
                encrypted: row.get(12)?,
                unencrypted_size: row.get(13)?,
                mount_point: row.get(14)?,
                favorite: false,
            })
        })?;
        let mut files: Vec<NcFileCache> = rows.collect::<std::result::Result<Vec<_>, _>>()?;

        let favorites = self.read_favorites_set()?;
        for f in &mut files {
            if favorites.contains_key(&f.file_id) {
                f.favorite = true;
            }
        }
        Ok(files)
    }

    pub fn read_shares(&self) -> MigrateResult<Vec<NcShare>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, share_type, share_with, uid_owner, uid_initiator, parent, \
             item_type, item_source, file_source, file_target, permissions \
             FROM oc_share",
        )?;
        let rows = stmt.query_map(params![], |row| {
            Ok(NcShare {
                id: row.get(0)?,
                share_type: row.get(1)?,
                share_with: row.get(2)?,
                uid_owner: row.get(3)?,
                uid_initiator: row.get(4)?,
                parent: row.get(5)?,
                item_type: row.get(6)?,
                item_source: row.get(7)?,
                file_source: row.get(8)?,
                file_target: row.get(9)?,
                permissions: row.get(10)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn read_system_tags(&self) -> MigrateResult<Vec<NcSystemTag>> {
        let mut stmt = self.conn.prepare("SELECT id, name FROM oc_systemtag")?;
        let rows = stmt.query_map(params![], |row| {
            Ok(NcSystemTag {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn read_tag_mappings(&self) -> MigrateResult<Vec<NcTagMapping>> {
        let mut stmt = self
            .conn
            .prepare("SELECT objectid, objecttype, systemtagid FROM oc_systemtag_object_mapping")?;
        let rows = stmt.query_map(params![], |row| {
            Ok(NcTagMapping {
                object_id: row.get(0)?,
                object_type: row.get(1)?,
                systemtag_id: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn read_favorites(&self) -> MigrateResult<Vec<NcFavorite>> {
        let mut stmt = self
            .conn
            .prepare("SELECT objid, type, categoryid FROM oc_vcategory_to_object")?;
        let rows = stmt.query_map(params![], |row| {
            Ok(NcFavorite {
                object_id: row.get(0)?,
                object_type: row.get(1)?,
                category_id: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn read_favorites_set(&self) -> MigrateResult<HashMap<i64, bool>> {
        let mut stmt = self.conn.prepare("SELECT objid FROM oc_vcategory_to_object")?;
        let rows = stmt.query_map(params![], |row| {
            let id: i64 = row.get(0)?;
            Ok(id)
        })?;
        let mut map = HashMap::new();
        for row in rows {
            let id = row?;
            map.insert(id, true);
        }
        Ok(map)
    }
}
