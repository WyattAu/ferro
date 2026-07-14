use anyhow::Result;
use bytes::Bytes;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[cfg(feature = "offline-cache")]
use crate::cache::OfflineCache;
#[cfg(feature = "offline-cache")]
use std::path::PathBuf;

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

#[derive(Debug, Clone)]
struct InodeEntry {
    path: String,
    ino: u64,
    is_dir: bool,
    size: u64,
    modified: SystemTime,
}

#[cfg(target_os = "linux")]
impl InodeEntry {
    fn to_file_attr(&self, uid: u32, gid: u32) -> FileAttr {
        FileAttr {
            ino: self.ino,
            size: self.size,
            blocks: self.size.div_ceil(512),
            atime: self.modified.into(),
            mtime: self.modified.into(),
            ctime: self.modified.into(),
            kind: if self.is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            },
            perm: if self.is_dir { 0o755 } else { 0o644 },
            nlink: if self.is_dir { 2 } else { 1 },
            uid,
            gid,
            rdev: 0,
            blksize: 4096,
        }
    }
}

#[derive(Debug)]
struct FileHandleEntry {
    #[allow(dead_code)]
    path: String,
    flags: u32,
    #[allow(dead_code)]
    ino: u64,
}

impl FileHandleEntry {
    fn is_readable(&self) -> bool {
        (self.flags & libc::O_ACCMODE as u32) != libc::O_WRONLY as u32
    }

    fn is_writable(&self) -> bool {
        (self.flags & libc::O_ACCMODE as u32) != libc::O_RDONLY as u32
    }
}

struct HeadResult {
    size: u64,
    #[allow(dead_code)]
    modified: String,
    #[allow(dead_code)]
    content_type: String,
    is_collection: bool,
}

pub struct FerroFs {
    client: Client,
    server_url: String,
    auth_header: Option<String>,
    uid: u32,
    gid: u32,
    cache: Arc<RwLock<HashMap<String, Bytes>>>,
    inodes: Arc<RwLock<HashMap<u64, InodeEntry>>>,
    file_handles: Arc<RwLock<HashMap<u64, FileHandleEntry>>>,
    fh_counter: Arc<AtomicU64>,
    #[cfg(feature = "offline-cache")]
    offline_cache: Option<OfflineCache>,
}

#[allow(dead_code)]
impl FerroFs {
    pub fn new(server_url: &str, token: Option<&str>, uid: u32, gid: u32, cache_dir: Option<&str>) -> Result<Self> {
        let client = Client::builder().timeout(std::time::Duration::from_secs(30)).build()?;

        let auth_header = token.map(|t| {
            if t.contains(':') {
                format!("Basic {}", base64_encode(t))
            } else {
                format!("Bearer {}", t)
            }
        });

        #[cfg(feature = "offline-cache")]
        let offline_cache = match cache_dir {
            Some(dir) => Some(OfflineCache::new(PathBuf::from(dir)).map_err(|e| anyhow::anyhow!(e))?),
            None => None,
        };
        #[cfg(not(feature = "offline-cache"))]
        let _ = cache_dir;

        Ok(Self {
            client,
            server_url: server_url.trim_end_matches('/').to_string(),
            auth_header,
            uid,
            gid,
            cache: Arc::new(RwLock::new(HashMap::new())),
            inodes: Arc::new(RwLock::new(HashMap::new())),
            file_handles: Arc::new(RwLock::new(HashMap::new())),
            fh_counter: Arc::new(AtomicU64::new(0)),
            #[cfg(feature = "offline-cache")]
            offline_cache,
        })
    }

    fn make_url(&self, path: &str) -> String {
        let encoded = urlencoding::encode(path.trim_start_matches('/'));
        format!("{}/{}", self.server_url, encoded)
    }

    fn next_fh(&self) -> u64 {
        self.fh_counter.fetch_add(1, Ordering::Relaxed) + 1
    }

    async fn ino_to_path(&self, ino: u64) -> String {
        if ino == 1 {
            return "/".to_string();
        }
        let inodes = self.inodes.read().await;
        inodes
            .get(&ino)
            .map(|e| e.path.clone())
            .unwrap_or_else(|| format!("/unknown-{}", ino))
    }

    async fn webdav_head(&self, path: &str) -> Result<HeadResult> {
        let url = self.make_url(path);
        let mut req = self.client.head(&url);
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await?;

        let size: u64 = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let modified = resp
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let is_collection = resp
            .headers()
            .get("dav:resourcetype")
            .map(|v| v.to_str().unwrap_or("").contains("collection"))
            .unwrap_or(false);

        Ok(HeadResult {
            size,
            modified,
            content_type,
            is_collection,
        })
    }

    async fn webdav_propfind(&self, path: &str) -> Result<Vec<FileEntry>> {
        let url = self.make_url(path);
        let mut req = self.client.request(
            reqwest::Method::from_bytes(b"PROPFIND").expect("valid HTTP method"),
            &url,
        );
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

        #[cfg(feature = "offline-cache")]
        if let Some(ref oc) = self.offline_cache
            && let Ok(Some(data)) = oc.get(path).await
        {
            let bytes = Bytes::from(data);
            if bytes.len() < 10 * 1024 * 1024 {
                let mut mem_cache = self.cache.write().await;
                mem_cache.insert(path.to_string(), bytes.clone());
                if mem_cache.len() > 10_000 {
                    mem_cache.clear();
                }
            }
            return Ok(bytes);
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

        #[cfg(feature = "offline-cache")]
        if let Some(ref oc) = self.offline_cache
            && let Err(e) = oc.put(path, &data, None).await
        {
            warn!(error = %e, path = %path, "offline cache write failed during GET");
        }

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
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                #[cfg(feature = "offline-cache")]
                if let Some(ref oc) = self.offline_cache {
                    if let Err(qe) = oc.queue_write(path, data).await {
                        warn!(error = %qe, path = %path, "offline queue write failed during PUT send error");
                    }
                    return Ok(());
                }
                return Err(e.into());
            }
        };
        if !resp.status().is_success() && resp.status().as_u16() != 204 {
            #[cfg(feature = "offline-cache")]
            if let Some(ref oc) = self.offline_cache {
                if let Err(qe) = oc.queue_write(path, data).await {
                    warn!(error = %qe, path = %path, "offline queue write failed during PUT error response");
                }
                return Ok(());
            }
            anyhow::bail!("PUT {} failed: {}", url, resp.status());
        }

        if data.len() < 10 * 1024 * 1024 {
            let mut cache = self.cache.write().await;
            cache.insert(path.to_string(), Bytes::from(data.to_vec()));
        }

        #[cfg(feature = "offline-cache")]
        if let Some(ref oc) = self.offline_cache
            && let Err(e) = oc.put(path, data, None).await
        {
            warn!(error = %e, path = %path, "offline cache write failed during PUT");
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

        {
            let mut cache = self.cache.write().await;
            cache.remove(path);
            let prefix = format!("{}/", path);
            cache.retain(|k, _| !k.starts_with(&prefix));
        }

        #[cfg(feature = "offline-cache")]
        if let Some(ref oc) = self.offline_cache {
            let _ = oc.invalidate(path);
        }

        Ok(())
    }

    async fn webdav_mkcol(&self, path: &str) -> Result<()> {
        let url = self.make_url(path);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").expect("valid HTTP method"), &url);
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
        let is_collection = response.contains("<d:collection/>") || response.contains("<d:collection ");
        let size: u64 = size_re.captures(response).and_then(|c| c[1].parse().ok()).unwrap_or(0);
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
impl Filesystem for FerroFs {
    type DirEntryStream<'a> = stream::Iter<std::vec::IntoIter<FuseResult<DirectoryEntry>>>;
    type DirEntryPlusStream<'a> = stream::Iter<std::vec::IntoIter<FuseResult<DirectoryEntryPlus>>>;

    async fn init(&self, _req: Request) -> FuseResult<ReplyInit> {
        Ok(ReplyInit {
            max_write: std::num::NonZeroU32::new(4 * 1024 * 1024).expect("non-zero constant"),
        })
    }

    async fn destroy(&self, _req: Request) {}

    async fn lookup(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> FuseResult<ReplyEntry> {
        let parent_path = self.ino_to_path(parent).await;
        let child_path = format!("{}/{}", parent_path.trim_end_matches('/'), name.to_string_lossy());
        debug!(
            "lookup: parent={} name={} -> {}",
            parent,
            name.to_string_lossy(),
            child_path
        );

        match self.webdav_head(&child_path).await {
            Ok(meta) => {
                let ino = path_to_ino(&child_path);
                let entry = InodeEntry {
                    path: child_path.clone(),
                    ino,
                    is_dir: meta.is_collection,
                    size: meta.size,
                    modified: SystemTime::now(),
                };
                self.inodes.write().await.insert(ino, entry.clone());
                Ok(ReplyEntry {
                    ttl: TTL,
                    attr: entry.to_file_attr(self.uid, self.gid),
                    generation: 0,
                })
            }
            Err(_) => Err(libc::ENOENT.into()),
        }
    }

    async fn getattr(&self, _req: Request, inode: u64, _fh: Option<u64>, _flags: u32) -> FuseResult<ReplyAttr> {
        debug!("getattr: ino={}", inode);
        if inode == 1 {
            return Ok(ReplyAttr {
                ttl: TTL,
                attr: dir_attr(1, self.uid, self.gid),
            });
        }
        let inodes = self.inodes.read().await;
        match inodes.get(&inode) {
            Some(entry) => Ok(ReplyAttr {
                ttl: TTL,
                attr: entry.to_file_attr(self.uid, self.gid),
            }),
            None => Err(libc::ENOENT.into()),
        }
    }

    async fn setattr(&self, _req: Request, inode: u64, _fh: Option<u64>, _set_attr: SetAttr) -> FuseResult<ReplyAttr> {
        if inode == 1 {
            return Ok(ReplyAttr {
                ttl: TTL,
                attr: dir_attr(1, self.uid, self.gid),
            });
        }
        let inodes = self.inodes.read().await;
        match inodes.get(&inode) {
            Some(entry) => Ok(ReplyAttr {
                ttl: TTL,
                attr: entry.to_file_attr(self.uid, self.gid),
            }),
            None => Err(libc::ENOENT.into()),
        }
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
        parent: u64,
        name: &std::ffi::OsStr,
        _mode: u32,
        _umask: u32,
    ) -> FuseResult<ReplyEntry> {
        let parent_path = self.ino_to_path(parent).await;
        let child_path = format!("{}/{}", parent_path.trim_end_matches('/'), name.to_string_lossy());

        self.webdav_mkcol(&child_path).await.map_err(|_| libc::EIO)?;

        let ino = path_to_ino(&child_path);
        let entry = InodeEntry {
            path: child_path.clone(),
            ino,
            is_dir: true,
            size: 0,
            modified: SystemTime::now(),
        };
        let attr = entry.to_file_attr(self.uid, self.gid);
        self.inodes.write().await.insert(ino, entry);

        Ok(ReplyEntry {
            ttl: TTL,
            attr,
            generation: 0,
        })
    }

    async fn unlink(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> FuseResult<()> {
        let parent_path = self.ino_to_path(parent).await;
        let child_path = format!("{}/{}", parent_path.trim_end_matches('/'), name.to_string_lossy());

        self.webdav_delete(&child_path).await.map_err(|_| libc::ENOENT)?;

        let ino = path_to_ino(&child_path);
        self.inodes.write().await.remove(&ino);

        Ok(())
    }

    async fn rmdir(&self, _req: Request, parent: u64, name: &std::ffi::OsStr) -> FuseResult<()> {
        let parent_path = self.ino_to_path(parent).await;
        let child_path = format!("{}/{}", parent_path.trim_end_matches('/'), name.to_string_lossy());

        self.webdav_delete(&child_path).await.map_err(|_| libc::ENOENT)?;

        let ino = path_to_ino(&child_path);
        self.inodes.write().await.remove(&ino);

        Ok(())
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
        parent: u64,
        name: &std::ffi::OsStr,
        newparent: u64,
        newname: &std::ffi::OsStr,
    ) -> FuseResult<()> {
        let old_path = format!(
            "{}/{}",
            self.ino_to_path(parent).await.trim_end_matches('/'),
            name.to_string_lossy()
        );
        let new_path = format!(
            "{}/{}",
            self.ino_to_path(newparent).await.trim_end_matches('/'),
            newname.to_string_lossy()
        );

        let url = self.make_url(&old_path);
        let destination = self.make_url(&new_path);

        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"MOVE").expect("valid HTTP method"), &url);
        req = req.header("Destination", &destination);
        if let Some(ref auth) = self.auth_header {
            req = req.header("Authorization", auth);
        }
        let resp = req.send().await.map_err(|_| libc::EIO)?;

        if !resp.status().is_success() && resp.status().as_u16() != 204 {
            return Err(libc::EIO.into());
        }

        let old_ino = path_to_ino(&old_path);
        let new_ino = path_to_ino(&new_path);
        let mut inodes = self.inodes.write().await;
        if let Some(mut entry) = inodes.remove(&old_ino) {
            entry.path = new_path;
            entry.ino = new_ino;
            inodes.insert(new_ino, entry);
        }

        Ok(())
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

    async fn open(&self, _req: Request, inode: u64, flags: u32) -> FuseResult<ReplyOpen> {
        debug!("open: ino={}", inode);
        let path = self.ino_to_path(inode).await;
        let fh = self.next_fh();
        let handle = FileHandleEntry {
            path,
            flags,
            ino: inode,
        };
        self.file_handles.write().await.insert(fh, handle);
        Ok(ReplyOpen { fh, flags: 0 })
    }

    async fn read(&self, _req: Request, _inode: u64, fh: u64, offset: u64, size: u32) -> FuseResult<ReplyData> {
        debug!("read: fh={} offset={} size={}", fh, offset, size);
        let handles = self.file_handles.read().await;
        let handle = handles.get(&fh).ok_or(libc::EBADF)?;
        if !handle.is_readable() {
            return Err(libc::EBADF.into());
        }
        let path = handle.path.clone();
        drop(handles);

        let data = self.webdav_get(&path).await.map_err(|_| libc::EIO)?;

        let start = offset as usize;
        let end = std::cmp::min(start + size as usize, data.len());
        if start >= data.len() {
            return Ok(ReplyData { data: Bytes::new() });
        }
        Ok(ReplyData {
            data: data.slice(start..end),
        })
    }

    async fn write(
        &self,
        _req: Request,
        ino: u64,
        fh: u64,
        _offset: u64,
        data: &[u8],
        _write_flags: u32,
        _flags: u32,
    ) -> FuseResult<ReplyWrite> {
        debug!("write: ino={} fh={} size={}", ino, fh, data.len());
        let handles = self.file_handles.read().await;
        let handle = handles.get(&fh).ok_or(libc::EBADF)?;
        if !handle.is_writable() {
            return Err(libc::EBADF.into());
        }
        let path = handle.path.clone();
        drop(handles);

        self.webdav_put(&path, data, None).await.map_err(|_| libc::EIO)?;

        if let Some(entry) = self.inodes.write().await.get_mut(&ino) {
            entry.size = data.len() as u64;
            entry.modified = SystemTime::now();
        }

        Ok(ReplyWrite {
            written: data.len() as u32,
        })
    }

    async fn flush(&self, _req: Request, _inode: u64, _fh: u64, _lock_owner: u64) -> FuseResult<()> {
        Ok(())
    }

    async fn release(
        &self,
        _req: Request,
        _inode: u64,
        fh: u64,
        _flags: u32,
        _lock_owner: u64,
        _flush: bool,
    ) -> FuseResult<()> {
        self.file_handles.write().await.remove(&fh);
        Ok(())
    }

    async fn fsync(&self, _req: Request, _inode: u64, _fh: u64, _datasync: bool) -> FuseResult<()> {
        Ok(())
    }

    async fn opendir(&self, _req: Request, inode: u64, _flags: u32) -> FuseResult<ReplyOpen> {
        debug!("opendir: ino={}", inode);
        let path = self.ino_to_path(inode).await;
        let fh = self.next_fh();
        let handle = FileHandleEntry {
            path,
            flags: libc::O_RDONLY as u32,
            ino: inode,
        };
        self.file_handles.write().await.insert(fh, handle);
        Ok(ReplyOpen { fh, flags: 0 })
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
        parent: u64,
        _fh: u64,
        _offset: u64,
        _lock_owner: u64,
    ) -> FuseResult<ReplyDirectoryPlus<Self::DirEntryPlusStream<'a>>> {
        debug!("readdirplus: parent={}", parent);

        let dir_path = self.ino_to_path(parent).await;
        let mut entries = Vec::new();

        entries.push(Ok(DirectoryEntryPlus {
            inode: parent,
            generation: 0,
            kind: FileType::Directory,
            name: ".".into(),
            offset: 1,
            attr: dir_attr(parent, self.uid, self.gid),
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

        match self.webdav_propfind(&dir_path).await {
            Ok(items) => {
                for (i, item) in items.iter().enumerate() {
                    let item_path_trimmed = item.path.trim_end_matches('/');
                    let dir_path_trimmed = dir_path.trim_end_matches('/');
                    if item_path_trimmed == dir_path_trimmed {
                        continue;
                    }

                    let name = item.path.rsplit('/').next().unwrap_or("");
                    if name.is_empty() {
                        continue;
                    }

                    let ino = path_to_ino(&item.path);
                    let entry = InodeEntry {
                        path: item.path.clone(),
                        ino,
                        is_dir: item.is_collection,
                        size: item.size,
                        modified: SystemTime::now(),
                    };
                    let attr = entry.to_file_attr(self.uid, self.gid);

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

                    self.inodes.write().await.insert(ino, entry);
                }
            }
            Err(e) => {
                warn!("Failed to list directory {}: {}", dir_path, e);
            }
        }

        Ok(ReplyDirectoryPlus {
            entries: stream::iter(entries),
        })
    }

    async fn releasedir(&self, _req: Request, _inode: u64, fh: u64, _flags: u32) -> FuseResult<()> {
        self.file_handles.write().await.remove(&fh);
        Ok(())
    }

    async fn fsyncdir(&self, _req: Request, _inode: u64, _fh: u64, _datasync: bool) -> FuseResult<()> {
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
        parent: u64,
        name: &std::ffi::OsStr,
        _mode: u32,
        flags: u32,
    ) -> FuseResult<ReplyCreated> {
        let parent_path = self.ino_to_path(parent).await;
        let child_path = format!("{}/{}", parent_path.trim_end_matches('/'), name.to_string_lossy());

        self.webdav_put(&child_path, &[], None).await.map_err(|_| libc::EIO)?;

        let ino = path_to_ino(&child_path);
        let entry = InodeEntry {
            path: child_path.clone(),
            ino,
            is_dir: false,
            size: 0,
            modified: SystemTime::now(),
        };
        let attr = entry.to_file_attr(self.uid, self.gid);
        self.inodes.write().await.insert(ino, entry);

        let fh = self.next_fh();
        let file_handle = FileHandleEntry {
            path: child_path,
            flags,
            ino: attr.ino,
        };
        self.file_handles.write().await.insert(fh, file_handle);

        Ok(ReplyCreated {
            ttl: TTL,
            attr,
            generation: 0,
            fh,
            flags: 0,
        })
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
        let fs = FerroFs::new("http://localhost:8080", None, 1000, 1000, None);
        assert!(fs.is_ok());
    }

    #[test]
    fn test_ferro_fs_new_with_token() {
        let fs = FerroFs::new("http://localhost:8080", Some("mytoken"), 1000, 1000, None);
        assert!(fs.is_ok());
        let fs = fs.unwrap();
        assert!(fs.auth_header.is_some());
        assert_eq!(fs.auth_header.unwrap(), "Bearer mytoken");
    }

    #[test]
    fn test_ferro_fs_new_with_basic_auth() {
        let fs = FerroFs::new("http://localhost:8080", Some("user:pass"), 1000, 1000, None);
        assert!(fs.is_ok());
        let fs = fs.unwrap();
        assert!(fs.auth_header.is_some());
        assert!(fs.auth_header.unwrap().starts_with("Basic "));
    }

    #[test]
    fn test_make_url() {
        let fs = FerroFs::new("http://localhost:8080", None, 1000, 1000, None).unwrap();
        assert_eq!(fs.make_url("/file.txt"), "http://localhost:8080/file.txt");
        assert_eq!(
            fs.make_url("/path with spaces/file.txt"),
            "http://localhost:8080/path%20with%20spaces%2Ffile.txt"
        );
    }

    #[test]
    fn test_make_url_trailing_slash() {
        let fs = FerroFs::new("http://localhost:8080/", None, 1000, 1000, None).unwrap();
        assert_eq!(fs.make_url("/file.txt"), "http://localhost:8080/file.txt");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_inode_entry_file_attr() {
        let entry = InodeEntry {
            path: "/file.txt".to_string(),
            ino: 42,
            is_dir: false,
            size: 1024,
            modified: SystemTime::UNIX_EPOCH,
        };
        let attr = entry.to_file_attr(1000, 1000);
        assert_eq!(attr.ino, 42);
        assert_eq!(attr.size, 1024);
        assert_eq!(attr.perm, 0o644);
        assert_eq!(attr.nlink, 1);
        assert_eq!(attr.kind, FileType::RegularFile);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_inode_entry_dir_attr() {
        let entry = InodeEntry {
            path: "/docs".to_string(),
            ino: 43,
            is_dir: true,
            size: 0,
            modified: SystemTime::UNIX_EPOCH,
        };
        let attr = entry.to_file_attr(1000, 1000);
        assert_eq!(attr.perm, 0o755);
        assert_eq!(attr.nlink, 2);
        assert_eq!(attr.kind, FileType::Directory);
    }

    #[test]
    fn test_file_handle_readable() {
        let h = FileHandleEntry {
            path: "/f".into(),
            flags: libc::O_RDONLY as u32,
            ino: 1,
        };
        assert!(h.is_readable());
        assert!(!h.is_writable());
    }

    #[test]
    fn test_file_handle_writable() {
        let h = FileHandleEntry {
            path: "/f".into(),
            flags: libc::O_WRONLY as u32,
            ino: 1,
        };
        assert!(!h.is_readable());
        assert!(h.is_writable());
    }

    #[test]
    fn test_file_handle_rdwr() {
        let h = FileHandleEntry {
            path: "/f".into(),
            flags: libc::O_RDWR as u32,
            ino: 1,
        };
        assert!(h.is_readable());
        assert!(h.is_writable());
    }

    #[test]
    fn test_next_fh_increments() {
        let fs = FerroFs::new("http://localhost:8080", None, 1000, 1000, None).unwrap();
        let fh1 = fs.next_fh();
        let fh2 = fs.next_fh();
        let fh3 = fs.next_fh();
        assert_eq!(fh1, 1);
        assert_eq!(fh2, 2);
        assert_eq!(fh3, 3);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_inode_entry_blocks_calculation() {
        let entry = InodeEntry {
            path: "/big".to_string(),
            ino: 99,
            is_dir: false,
            size: 10000,
            modified: SystemTime::UNIX_EPOCH,
        };
        let attr = entry.to_file_attr(1000, 1000);
        assert_eq!(attr.blocks, 10000_u64.div_ceil(512));
    }
}
