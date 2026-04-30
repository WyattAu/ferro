use anyhow::Result;
use bytes::Bytes;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[cfg(target_os = "linux")]
use fuse3::raw::reply::*;
#[cfg(target_os = "linux")]
use fuse3::raw::{Filesystem, Request};
#[cfg(target_os = "linux")]
use fuse3::{FileType, SetAttr, Timestamp};

#[cfg(target_os = "linux")]
use fuse3::Result as FuseResult;

#[cfg(target_os = "linux")]
use futures::stream;

const TTL: std::time::Duration = std::time::Duration::from_secs(1);

pub struct FerroFs {
    client: Client,
    server_url: String,
    auth_header: Option<String>,
    uid: u32,
    gid: u32,
    cache: Arc<RwLock<HashMap<String, Bytes>>>,
}

#[allow(dead_code)]
impl FerroFs {
    pub fn new(server_url: &str, token: Option<&str>, uid: u32, gid: u32) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let auth_header = token.map(|t| {
            if t.contains(':') {
                format!("Basic {}", base64_encode(t))
            } else {
                format!("Bearer {}", t)
            }
        });

        Ok(Self {
            client,
            server_url: server_url.trim_end_matches('/').to_string(),
            auth_header,
            uid,
            gid,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn make_url(&self, path: &str) -> String {
        let encoded = urlencoding::encode(path.trim_start_matches('/'));
        format!("{}/{}", self.server_url, encoded)
    }

    async fn webdav_propfind(&self, path: &str) -> Result<Vec<FileEntry>> {
        let url = self.make_url(path);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url);
        req = req.header("Depth", "1");
        req = req.header("Content-Type", "application/xml");
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        req = req.body(r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:displayname/><d:getcontentlength/><d:getlastmodified/><d:getcontenttype/><d:resourcetype/></d:prop></d:propfind>"#);

        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("PROPFIND {} failed: {}", url, resp.status());
        }
        let body = resp.text().await?;
        parse_propfind_response(&body)
    }

    async fn webdav_get(&self, path: &str) -> Result<Bytes> {
        {
            let cache = self.cache.read().await;
            if let Some(data) = cache.get(path) {
                return Ok(data.clone());
            }
        }

        let url = self.make_url(path);
        let mut req = self.client.get(&url);
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("GET {} failed: {}", url, resp.status());
        }
        let data = resp.bytes().await?;

        if data.len() < 10 * 1024 * 1024 {
            let mut cache = self.cache.write().await;
            cache.insert(path.to_string(), data.clone());
            if cache.len() > 10_000 {
                cache.clear();
            }
        }

        Ok(data)
    }

    async fn webdav_put(&self, path: &str, data: &[u8], content_type: Option<&str>) -> Result<()> {
        let url = self.make_url(path);
        let mut req = self.client.put(&url).body(data.to_vec());
        if let Some(ct) = content_type {
            req = req.header("Content-Type", ct);
        }
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() && resp.status().as_u16() != 204 {
            anyhow::bail!("PUT {} failed: {}", url, resp.status());
        }

        if data.len() < 10 * 1024 * 1024 {
            let mut cache = self.cache.write().await;
            cache.insert(path.to_string(), Bytes::from(data.to_vec()));
        }

        Ok(())
    }

    async fn webdav_delete(&self, path: &str) -> Result<()> {
        let url = self.make_url(path);
        let mut req = self.client.delete(&url);
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() && resp.status().as_u16() != 204 {
            anyhow::bail!("DELETE {} failed: {}", url, resp.status());
        }

        let mut cache = self.cache.write().await;
        cache.remove(path);
        let prefix = format!("{}/", path);
        cache.retain(|k, _| !k.starts_with(&prefix));

        Ok(())
    }

    async fn webdav_mkcol(&self, path: &str) -> Result<()> {
        let url = self.make_url(path);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url);
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() && resp.status().as_u16() != 201 {
            anyhow::bail!("MKCOL {} failed: {}", url, resp.status());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: String,
    is_collection: bool,
    size: u64,
    #[allow(dead_code)]
    modified: String,
    #[allow(dead_code)]
    content_type: String,
}

fn parse_propfind_response(xml: &str) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();

    let response_re = lazy_regex::regex!(r#"<d:response>([\s\S]*?)</d:response>"#);
    let href_re = lazy_regex::regex!(r#"<d:href>([^<]*)</d:href>"#);
    let size_re = lazy_regex::regex!(r#"<d:getcontentlength>([^<]*)</d:getcontentlength>"#);
    let modified_re = lazy_regex::regex!(r#"<d:getlastmodified>([^<]*)</d:getlastmodified>"#);
    let content_type_re = lazy_regex::regex!(r#"<d:getcontenttype>([^<]*)</d:getcontenttype>"#);

    for response_cap in response_re.captures_iter(xml) {
        let response = &response_cap[1];

        let path = href_re
            .captures(response)
            .map(|c| percent_decode(&c[1]))
            .unwrap_or_default();
        let is_collection =
            response.contains("<d:collection/>") || response.contains("<d:collection ");
        let size: u64 = size_re
            .captures(response)
            .and_then(|c| c[1].parse().ok())
            .unwrap_or(0);
        let modified = modified_re
            .captures(response)
            .map(|c| c[1].to_string())
            .unwrap_or_default();
        let content_type = content_type_re
            .captures(response)
            .map(|c| c[1].to_string())
            .unwrap_or_default();

        entries.push(FileEntry {
            path,
            is_collection,
            size,
            modified,
            content_type,
        });
    }

    Ok(entries)
}

fn percent_decode(s: &str) -> String {
    urlencoding::decode(s)
        .map(|b| b.to_string())
        .unwrap_or_else(|_| s.to_string())
}

fn base64_encode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

fn path_to_ino(path: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

#[cfg(target_os = "linux")]
fn now_timestamp() -> Timestamp {
    std::time::SystemTime::now().into()
}

#[cfg(target_os = "linux")]
fn dir_attr(ino: u64, uid: u32, gid: u32) -> FileAttr {
    FileAttr {
        ino,
        size: 0,
        blocks: 0,
        atime: now_timestamp(),
        mtime: now_timestamp(),
        ctime: now_timestamp(),
        kind: FileType::Directory,
        perm: 0o755,
        nlink: 2,
        uid,
        gid,
        rdev: 0,
        blksize: 4096,
    }
}

#[cfg(target_os = "linux")]
fn file_attr(ino: u64, size: u64, uid: u32, gid: u32) -> FileAttr {
    FileAttr {
        ino,
        size,
        blocks: size.div_ceil(512),
        atime: now_timestamp(),
        mtime: now_timestamp(),
        ctime: now_timestamp(),
        kind: FileType::RegularFile,
        perm: 0o644,
        nlink: 1,
        uid,
        gid,
        rdev: 0,
        blksize: 4096,
    }
}

#[cfg(target_os = "linux")]
impl Filesystem for FerroFs {
    type DirEntryStream<'a> = stream::Iter<std::vec::IntoIter<FuseResult<DirectoryEntry>>>;
    type DirEntryPlusStream<'a> = stream::Iter<std::vec::IntoIter<FuseResult<DirectoryEntryPlus>>>;

    async fn init(&self, _req: Request) -> FuseResult<ReplyInit> {
        Ok(ReplyInit {
            max_write: std::num::NonZeroU32::new(4 * 1024 * 1024).unwrap(),
        })
    }

    async fn destroy(&self, _req: Request) {}

    async fn lookup(
        &self,
        _req: Request,
        parent: u64,
        name: &std::ffi::OsStr,
    ) -> FuseResult<ReplyEntry> {
        let child_path = format!("/{}/{}", parent, name.to_string_lossy());
        debug!(
            "lookup: parent={} name={} -> {}",
            parent,
            name.to_string_lossy(),
            child_path
        );

        match self.webdav_propfind(&child_path).await {
            Ok(entries) => {
                if let Some(entry) = entries.first() {
                    let ino = path_to_ino(&entry.path);
                    let attr = if entry.is_collection {
                        dir_attr(ino, self.uid, self.gid)
                    } else {
                        file_attr(ino, entry.size, self.uid, self.gid)
                    };
                    Ok(ReplyEntry {
                        ttl: TTL,
                        attr,
                        generation: 0,
                    })
                } else {
                    Err(libc::ENOENT.into())
                }
            }
            Err(_) => Err(libc::ENOENT.into()),
        }
    }

    async fn getattr(
        &self,
        _req: Request,
        inode: u64,
        _fh: Option<u64>,
        _flags: u32,
    ) -> FuseResult<ReplyAttr> {
        debug!("getattr: ino={}", inode);
        if inode == 1 {
            return Ok(ReplyAttr {
                ttl: TTL,
                attr: dir_attr(1, self.uid, self.gid),
            });
        }
        Err(libc::ENOENT.into())
    }

    async fn setattr(
        &self,
        _req: Request,
        _inode: u64,
        _fh: Option<u64>,
        _set_attr: SetAttr,
    ) -> FuseResult<ReplyAttr> {
        Err(libc::ENOSYS.into())
    }

    async fn readlink(&self, _req: Request, _inode: u64) -> FuseResult<ReplyData> {
        Err(libc::ENOSYS.into())
    }

    async fn mknod(
        &self,
        _req: Request,
        _parent: u64,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _rdev: u32,
    ) -> FuseResult<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }

    async fn mkdir(
        &self,
        _req: Request,
        _parent: u64,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _umask: u32,
    ) -> FuseResult<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }

    async fn unlink(&self, _req: Request, _parent: u64, _name: &std::ffi::OsStr) -> FuseResult<()> {
        Err(libc::ENOSYS.into())
    }

    async fn rmdir(&self, _req: Request, _parent: u64, _name: &std::ffi::OsStr) -> FuseResult<()> {
        Err(libc::ENOSYS.into())
    }

    async fn symlink(
        &self,
        _req: Request,
        _parent: u64,
        _name: &std::ffi::OsStr,
        _link: &std::ffi::OsStr,
    ) -> FuseResult<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }

    async fn rename(
        &self,
        _req: Request,
        _parent: u64,
        _name: &std::ffi::OsStr,
        _new_parent: u64,
        _new_name: &std::ffi::OsStr,
    ) -> FuseResult<()> {
        Err(libc::ENOSYS.into())
    }

    async fn link(
        &self,
        _req: Request,
        _inode: u64,
        _new_parent: u64,
        _new_name: &std::ffi::OsStr,
    ) -> FuseResult<ReplyEntry> {
        Err(libc::ENOSYS.into())
    }

    async fn open(&self, _req: Request, _inode: u64, _flags: u32) -> FuseResult<ReplyOpen> {
        debug!("open: ino={}", _inode);
        Ok(ReplyOpen { fh: 0, flags: 0 })
    }

    async fn read(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _offset: u64,
        size: u32,
    ) -> FuseResult<ReplyData> {
        debug!("read: ino={} size={}", _inode, size);
        Ok(ReplyData {
            data: Bytes::from(vec![0u8; size as usize]),
        })
    }

    async fn write(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _offset: u64,
        data: &[u8],
        _write_flags: u32,
        _flags: u32,
    ) -> FuseResult<ReplyWrite> {
        debug!("write: ino={} size={}", _inode, data.len());
        Ok(ReplyWrite {
            written: data.len() as u32,
        })
    }

    async fn flush(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _lock_owner: u64,
    ) -> FuseResult<()> {
        Ok(())
    }

    async fn release(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    async fn fsync(&self, _req: Request, _inode: u64, _fh: u64, _datasync: bool) -> FuseResult<()> {
        Ok(())
    }

    async fn opendir(&self, _req: Request, _inode: u64, _flags: u32) -> FuseResult<ReplyOpen> {
        debug!("opendir: ino={}", _inode);
        Ok(ReplyOpen { fh: 0, flags: 0 })
    }

    async fn readdir<'a>(
        &'a self,
        _req: Request,
        _parent: u64,
        _fh: u64,
        _offset: i64,
    ) -> FuseResult<ReplyDirectory<Self::DirEntryStream<'a>>> {
        let entries: Vec<FuseResult<DirectoryEntry>> = vec![];
        Ok(ReplyDirectory {
            entries: stream::iter(entries),
        })
    }

    async fn readdirplus<'a>(
        &'a self,
        _req: Request,
        _parent: u64,
        _fh: u64,
        _offset: u64,
        _lock_owner: u64,
    ) -> FuseResult<ReplyDirectoryPlus<Self::DirEntryPlusStream<'a>>> {
        debug!("readdirplus: parent={}", _parent);

        let mut entries = Vec::new();

        entries.push(Ok(DirectoryEntryPlus {
            inode: 1,
            generation: 0,
            kind: FileType::Directory,
            name: ".".into(),
            offset: 1,
            attr: dir_attr(1, self.uid, self.gid),
            entry_ttl: TTL,
            attr_ttl: TTL,
        }));
        entries.push(Ok(DirectoryEntryPlus {
            inode: 1,
            generation: 0,
            kind: FileType::Directory,
            name: "..".into(),
            offset: 2,
            attr: dir_attr(1, self.uid, self.gid),
            entry_ttl: TTL,
            attr_ttl: TTL,
        }));

        match self.webdav_propfind("/").await {
            Ok(items) => {
                for (i, item) in items.iter().enumerate() {
                    let name = item.path.rsplit('/').next().unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }
                    let ino = path_to_ino(&item.path);
                    let attr = if item.is_collection {
                        dir_attr(ino, self.uid, self.gid)
                    } else {
                        file_attr(ino, item.size, self.uid, self.gid)
                    };
                    entries.push(Ok(DirectoryEntryPlus {
                        inode: ino,
                        generation: 0,
                        kind: attr.kind,
                        name: name.into(),
                        offset: (i + 3) as i64,
                        attr,
                        entry_ttl: TTL,
                        attr_ttl: TTL,
                    }));
                }
            }
            Err(e) => {
                warn!("Failed to list directory: {}", e);
            }
        }

        Ok(ReplyDirectoryPlus {
            entries: stream::iter(entries),
        })
    }

    async fn releasedir(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _flags: u32,
    ) -> FuseResult<()> {
        Ok(())
    }

    async fn fsyncdir(
        &self,
        _req: Request,
        _inode: u64,
        _fh: u64,
        _datasync: bool,
    ) -> FuseResult<()> {
        Ok(())
    }

    async fn statfs(&self, _req: Request, _inode: u64) -> FuseResult<ReplyStatFs> {
        Ok(ReplyStatFs {
            blocks: 1_000_000,
            bfree: 500_000,
            bavail: 500_000,
            files: 100_000,
            ffree: 99_999,
            bsize: 4096,
            namelen: 255,
            frsize: 4096,
        })
    }

    async fn access(&self, _req: Request, _inode: u64, _mask: u32) -> FuseResult<()> {
        Ok(())
    }

    async fn create(
        &self,
        _req: Request,
        _parent: u64,
        _name: &std::ffi::OsStr,
        _mode: u32,
        _flags: u32,
    ) -> FuseResult<ReplyCreated> {
        Err(libc::ENOSYS.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_ino_deterministic() {
        assert_eq!(path_to_ino("/file.txt"), path_to_ino("/file.txt"));
        assert_ne!(path_to_ino("/file.txt"), path_to_ino("/other.txt"));
    }

    #[test]
    fn test_base64_encode() {
        use base64::Engine;
        assert_eq!(
            base64_encode("admin:secret"),
            base64::engine::general_purpose::STANDARD.encode("admin:secret".as_bytes())
        );
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("%2Fpath%2Ffile.txt"), "/path/file.txt");
        assert_eq!(percent_decode("hello"), "hello");
    }

    #[test]
    fn test_ferro_fs_new() {
        let fs = FerroFs::new("http://localhost:8080", None, 1000, 1000);
        assert!(fs.is_ok());
    }

    #[test]
    fn test_ferro_fs_new_with_token() {
        let fs = FerroFs::new("http://localhost:8080", Some("mytoken"), 1000, 1000);
        assert!(fs.is_ok());
        let fs = fs.unwrap();
        assert!(fs.auth_header.is_some());
        assert_eq!(fs.auth_header.unwrap(), "Bearer mytoken");
    }

    #[test]
    fn test_ferro_fs_new_with_basic_auth() {
        let fs = FerroFs::new("http://localhost:8080", Some("user:pass"), 1000, 1000);
        assert!(fs.is_ok());
        let fs = fs.unwrap();
        assert!(fs.auth_header.is_some());
        assert!(fs.auth_header.unwrap().starts_with("Basic "));
    }

    #[test]
    fn test_make_url() {
        let fs = FerroFs::new("http://localhost:8080", None, 1000, 1000).unwrap();
        assert_eq!(fs.make_url("/file.txt"), "http://localhost:8080/file.txt");
        assert_eq!(
            fs.make_url("/path with spaces/file.txt"),
            "http://localhost:8080/path%20with%20spaces%2Ffile.txt"
        );
    }

    #[test]
    fn test_make_url_trailing_slash() {
        let fs = FerroFs::new("http://localhost:8080/", None, 1000, 1000).unwrap();
        assert_eq!(fs.make_url("/file.txt"), "http://localhost:8080/file.txt");
    }
}
