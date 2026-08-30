use anyhow::Result;
use bytes::Bytes;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory,
    ReplyEmpty, ReplyEntry, ReplyOpen, ReplyWrite, Request, Errno,
};
use reqwest::Client;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

const TTL: Duration = Duration::from_secs(1);
const BLOCK_SIZE: u32 = 4096;

#[derive(Debug, Clone)]
struct InodeEntry {
    path: String,
    ino: u64,
    is_dir: bool,
    size: u64,
    modified: SystemTime,
}

impl InodeEntry {
    fn to_file_attr(&self, uid: u32, gid: u32) -> FileAttr {
        FileAttr {
            ino: self.ino,
            size: self.size,
            blocks: self.size.div_ceil(512),
            atime: self.modified,
            mtime: self.modified,
            ctime: self.modified,
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
            blksize: BLOCK_SIZE,
            #[cfg(target_os = "macos")]
            crtime: self.modified,
            #[cfg(target_os = "macos")]
            flags: 0,
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

struct HeadResult {
    size: u64,
    modified: String,
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
    fh_counter: std::sync::atomic::AtomicU64,
    inode_counter: std::sync::atomic::AtomicU64,
}

impl FerroFs {
    pub fn new(
        server_url: String,
        auth_header: Option<String>,
        uid: u32,
        gid: u32,
    ) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            server_url,
            auth_header,
            uid,
            gid,
            cache: Arc::new(RwLock::new(HashMap::new())),
            inodes: Arc::new(RwLock::new(HashMap::new())),
            file_handles: Arc::new(RwLock::new(HashMap::new())),
            fh_counter: std::sync::atomic::AtomicU64::new(1),
            inode_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn dav_url(&self, path: &str) -> String {
        let clean = path.trim_start_matches('/');
        if clean.is_empty() {
            format!("{}/", self.server_url)
        } else {
            format!("{}/{}", self.server_url, clean)
        }
    }

    fn api_url(&self, path: &str) -> String {
        let clean = path.trim_start_matches('/');
        format!("{}/api/v1/files/{}", self.server_url, clean)
    }

    async fn webdav_head(&self, path: &str) -> Option<HeadResult> {
        let url = self.dav_url(path);
        let mut req = self.client.head(&url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                if status.is_success() || status == 404 {
                    let content_length = headers
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(0);
                    let modified = headers
                        .get("last-modified")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("")
                        .to_string();
                    let is_collection = headers
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .map(|ct| ct.contains("httpd/unix-directory"))
                        .unwrap_or(false);
                    Some(HeadResult {
                        size: content_length,
                        modified,
                        is_collection,
                    })
                } else {
                    None
                }
            }
            Err(e) => {
                warn!("HEAD {} failed: {}", path, e);
                None
            }
        }
    }

    async fn webdav_propfind(&self, path: &str) -> Option<Vec<InodeEntry>> {
        let url = self.dav_url(path);
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:allprop/>
</D:propfind>"#;
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        match req.send().await {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    warn!("PROPFIND {} returned {}", path, status);
                    return None;
                }
                match resp.text().await {
                    Ok(xml) => parse_propfind_response(&xml),
                    Err(e) => {
                        warn!("PROPFIND {} body read failed: {}", path, e);
                        None
                    }
                }
            }
            Err(e) => {
                warn!("PROPFIND {} failed: {}", path, e);
                None
            }
        }
    }

    async fn webdav_get(&self, path: &str, offset: u64, size: u32) -> Option<Bytes> {
        let url = self.dav_url(path);
        let range = format!("bytes={}-{}", offset, offset + size as u64 - 1);
        let mut req = self
            .client
            .get(&url)
            .header("Range", &range);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        match req.send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    resp.bytes().await.ok()
                } else {
                    warn!("GET {} returned {}", path, resp.status());
                    None
                }
            }
            Err(e) => {
                warn!("GET {} failed: {}", path, e);
                None
            }
        }
    }

    async fn webdav_put(&self, path: &str, data: &[u8]) -> bool {
        let url = self.dav_url(path);
        let mut req = self.client.put(&url).body(data.to_vec());
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        match req.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                warn!("PUT {} failed: {}", path, e);
                false
            }
        }
    }

    async fn webdav_delete(&self, path: &str) -> bool {
        let url = self.dav_url(path);
        let mut req = self.client.delete(&url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        match req.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                warn!("DELETE {} failed: {}", path, e);
                false
            }
        }
    }

    async fn webdav_mkcol(&self, path: &str) -> bool {
        let url = self.dav_url(path);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        match req.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(e) => {
                warn!("MKCOL {} failed: {}", path, e);
                false
            }
        }
    }

    async fn resolve_path(&self, path: &str) -> Option<InodeEntry> {
        let inodes = self.inodes.read().await;
        for entry in inodes.values() {
            if entry.path == path {
                return Some(entry.clone());
            }
        }
        drop(inodes);

        let head = self.webdav_head(path).await?;
        let ino = self
            .inode_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let entry = InodeEntry {
            path: path.to_string(),
            ino,
            is_dir: head.is_collection,
            size: head.size,
            modified: SystemTime::now(),
        };
        let mut inodes = self.inodes.write().await;
        inodes.insert(ino, entry.clone());
        Some(entry)
    }
}

fn parse_propfind_response(xml: &str) -> Option<Vec<InodeEntry>> {
    let mut entries = Vec::new();
    let mut current_href: Option<String> = None;
    let mut current_is_collection = false;
    let mut current_size: u64 = 0;

    for line in xml.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<D:href>") || trimmed.contains("<d:href>") {
            let start = trimmed.find('>').map(|i| i + 1)?;
            let end = trimmed.rfind('<')?;
            current_href = Some(trimmed[start..end].to_string());
        }
        if trimmed.contains("<D:collection") || trimmed.contains("<d:collection") {
            current_is_collection = true;
        }
        if trimmed.contains("<D:getcontentlength>")
            || trimmed.contains("<d:getcontentlength>")
        {
            let start = trimmed.find('>').map(|i| i + 1)?;
            let end = trimmed.rfind('<')?;
            current_size = trimmed[start..end].parse().unwrap_or(0);
        }
        if trimmed.contains("</D:response>") || trimmed.contains("</d:response>") {
            if let Some(href) = current_href.take() {
                let ino = fastrand::u64(..);
                entries.push(InodeEntry {
                    path: href,
                    ino,
                    is_dir: current_is_collection,
                    size: current_size,
                    modified: SystemTime::now(),
                });
            }
            current_is_collection = false;
            current_size = 0;
        }
    }
    Some(entries)
}

impl Filesystem for FerroFs {
    fn init(&mut self, _req: &Request<'_>, _config: &mut fuser::KernelConfig) -> Result<(), i32> {
        info!("FerroFS mounted");
        Ok(())
    }

    fn lookup(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEntry,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();
        let inodes = self.inodes.clone();
        let uid = self.uid;
        let gid = self.gid;
        let inode_counter = self.inode_counter.clone();

        tokio::runtime::Handle::current().spawn(async move {
            let parent_path = {
                let inodes = inodes.read().await;
                inodes.get(&parent).map(|e| e.path.clone())
            };
            let parent_path = match parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
            let child_path = if parent_path == "/" {
                format!("/{}", name_str)
            } else {
                format!("{}/{}", parent_path, name_str)
            };

            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            let dav_url = format!(
                "{}/{}",
                server_url,
                child_path.trim_start_matches('/')
            );
            let mut req = client.head(&dav_url);
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let headers = resp.headers().clone();
                    if status.is_success() || status == 404 {
                        if status == 404 {
                            reply.error(libc::ENOENT);
                            return;
                        }
                        let content_length = headers
                            .get("content-length")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(0);
                        let is_collection = headers
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .map(|ct| ct.contains("httpd/unix-directory"))
                            .unwrap_or(false);
                        let ino = inode_counter.fetch_add(1, Ordering::SeqCst);
                        let entry = InodeEntry {
                            path: child_path,
                            ino,
                            is_dir: is_collection,
                            size: content_length,
                            modified: SystemTime::now(),
                        };
                        let attr = entry.to_file_attr(uid, gid);
                        inodes.write().await.insert(ino, entry);
                        reply.entry(&TTL, &attr, 0);
                    } else {
                        reply.error(libc::ENOENT);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        let inodes = self.inodes.clone();
        let uid = self.uid;
        let gid = self.gid;

        tokio::runtime::Handle::current().spawn(async move {
            let inodes = inodes.read().await;
            match inodes.get(&ino) {
                Some(entry) => {
                    let attr = entry.to_file_attr(uid, gid);
                    reply.attr(&TTL, &attr);
                }
                None => {
                    reply.error(libc::ENOENT);
                }
            }
        });
    }

    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let inodes_ref = self.inodes.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();
        let inode_counter = self.inode_counter.clone();
        let uid = self.uid;
        let gid = self.gid;

        tokio::runtime::Handle::current().spawn(async move {
            let parent_path = {
                let inodes = inodes_ref.read().await;
                inodes.get(&ino).map(|e| e.path.clone())
            };
            let parent_path = match parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };

            // PROPFIND the parent directory
            let client = Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap();
            let dav_url = format!(
                "{}/{}",
                server_url,
                parent_path.trim_start_matches('/')
            );
            let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:allprop/>
</D:propfind>"#;
            let mut req = client
                .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &dav_url)
                .header("Depth", "1")
                .header("Content-Type", "application/xml")
                .body(body);
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            let entries = match req.send().await {
                Ok(resp) => {
                    if !resp.status().is_success() {
                        reply.error(libc::EIO);
                        return;
                    }
                    match resp.text().await {
                        Ok(xml) => {
                            let mut parsed = Vec::new();
                            let mut current_href: Option<String> = None;
                            let mut current_is_collection = false;
                            let mut current_size: u64 = 0;

                            for line in xml.lines() {
                                let t = line.trim();
                                if t.contains("<D:href>") || t.contains("<d:href>") {
                                    if let (Some(s), Some(e)) = (t.find('>').map(|i| i + 1), t.rfind('<'))
                                    {
                                        current_href = Some(t[s..e].to_string());
                                    }
                                }
                                if t.contains("<D:collection") || t.contains("<d:collection") {
                                    current_is_collection = true;
                                }
                                if t.contains("<D:getcontentlength>") || t.contains("<d:getcontentlength>") {
                                    if let (Some(s), Some(e)) = (t.find('>').map(|i| i + 1), t.rfind('<'))
                                    {
                                        current_size = t[s..e].parse().unwrap_or(0);
                                    }
                                }
                                if t.contains("</D:response>") || t.contains("</d:response>") {
                                    if let Some(href) = current_href.take() {
                                        let ino = inode_counter.fetch_add(1, Ordering::SeqCst);
                                        parsed.push(InodeEntry {
                                            path: href,
                                            ino,
                                            is_dir: current_is_collection,
                                            size: current_size,
                                            modified: SystemTime::now(),
                                        });
                                    }
                                    current_is_collection = false;
                                    current_size = 0;
                                }
                            }
                            parsed
                        }
                        Err(_) => {
                            reply.error(libc::EIO);
                            return;
                        }
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                    return;
                }
            };

            // Filter out the parent directory itself
            let filtered: Vec<_> = entries
                .into_iter()
                .filter(|e| e.path != parent_path && e.path != format!("{}/", parent_path))
                .collect();

            // Store inodes
            {
                let mut inodes = inodes_ref.write().await;
                for entry in &filtered {
                    inodes.insert(entry.ino, entry.clone());
                }
            }

            // Add . and .. entries
            let entries_with_dots = std::iter::once((ino, FileType::Directory, "."))
                .chain(std::iter::once((1, FileType::Directory, "..")))
                .chain(
                    filtered
                        .iter()
                        .map(|e| (e.ino, if e.is_dir { FileType::Directory } else { FileType::RegularFile }, "")),
                )
                .enumerate();

            for (i, (entry_ino, kind, name)) in entries_with_dots {
                if (i as i64) < offset {
                    continue;
                }
                let name_str = if name.is_empty() {
                    filtered[i - 2]
                        .path
                        .rsplit('/')
                        .next()
                        .unwrap_or("")
                        .to_string()
                } else {
                    name.to_string()
                };
                if reply.add(entry_ino, (i as i64) + 1, kind, &name_str) {
                    break;
                }
            }
            reply.ok();
        });
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: u32, reply: ReplyOpen) {
        let inodes = self.inodes.clone();
        let file_handles = self.file_handles.clone();
        let fh_counter = self.fh_counter.clone();

        tokio::runtime::Handle::current().spawn(async move {
            let path = {
                let inodes = inodes.read().await;
                inodes.get(&ino).map(|e| e.path.clone())
            };
            match path {
                Some(path) => {
                    let fh = fh_counter.fetch_add(1, Ordering::SeqCst);
                    file_handles.write().await.insert(
                        fh,
                        FileHandleEntry {
                            path,
                            flags,
                            ino,
                        },
                    );
                    reply.opened(fh, flags);
                }
                None => {
                    reply.error(libc::ENOENT);
                }
            }
        });
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let file_handles = self.file_handles.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();

        tokio::runtime::Handle::current().spawn(async move {
            let path = {
                let fh = file_handles.read().await;
                fh.get(&fh).map(|e| e.path.clone())
            };
            let path = match path {
                Some(p) => p,
                None => {
                    reply.error(libc::EBADF);
                    return;
                }
            };

            let client = Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap();
            let url = format!(
                "{}/{}",
                server_url,
                path.trim_start_matches('/')
            );
            let range = format!("bytes={}-{}", offset, offset + size as i64 - 1);
            let mut req = client.get(&url).header("Range", &range);
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.bytes().await {
                            Ok(data) => {
                                reply.data(&data);
                            }
                            Err(_) => {
                                reply.error(libc::EIO);
                            }
                        }
                    } else {
                        reply.error(libc::EIO);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        let file_handles = self.file_handles.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();

        tokio::runtime::Handle::current().spawn(async move {
            let path = {
                let fh = file_handles.read().await;
                fh.get(&fh).map(|e| e.path.clone())
            };
            let path = match path {
                Some(p) => p,
                None => {
                    reply.error(libc::EBADF);
                    return;
                }
            };

            let client = Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap();
            let url = format!(
                "{}/{}",
                server_url,
                path.trim_start_matches('/')
            );
            let mut req = client.put(&url).body(data.to_vec());
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        reply.written(data.len() as u32);
                    } else {
                        reply.error(libc::EIO);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let inodes_ref = self.inodes.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();
        let inode_counter = self.inode_counter.clone();
        let uid = self.uid;
        let gid = self.gid;

        tokio::runtime::Handle::current().spawn(async move {
            let parent_path = {
                let inodes = inodes_ref.read().await;
                inodes.get(&parent).map(|e| e.path.clone())
            };
            let parent_path = match parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
            let child_path = if parent_path == "/" {
                format!("/{}", name_str)
            } else {
                format!("{}/{}", parent_path, name_str)
            };

            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            let url = format!(
                "{}/{}",
                server_url,
                child_path.trim_start_matches('/')
            );
            let mut req = client
                .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url);
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let ino = inode_counter.fetch_add(1, Ordering::SeqCst);
                        let entry = InodeEntry {
                            path: child_path,
                            ino,
                            is_dir: true,
                            size: 0,
                            modified: SystemTime::now(),
                        };
                        let attr = entry.to_file_attr(uid, gid);
                        inodes_ref.write().await.insert(ino, entry);
                        reply.entry(&TTL, &attr, 0);
                    } else {
                        reply.error(libc::EIO);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn unlink(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let inodes_ref = self.inodes.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();

        tokio::runtime::Handle::current().spawn(async move {
            let parent_path = {
                let inodes = inodes_ref.read().await;
                inodes.get(&parent).map(|e| e.path.clone())
            };
            let parent_path = match parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
            let child_path = if parent_path == "/" {
                format!("/{}", name_str)
            } else {
                format!("{}/{}", parent_path, name_str)
            };

            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            let url = format!(
                "{}/{}",
                server_url,
                child_path.trim_start_matches('/')
            );
            let mut req = client.delete(&url);
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        reply.ok();
                    } else {
                        reply.error(libc::EIO);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn rmdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty,
    ) {
        self.unlink(_req, parent, name, reply);
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        new_parent: u64,
        new_name: &OsStr,
        _flags: u32,
        reply: ReplyEmpty,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let new_name_str = new_name.to_string_lossy().to_string();
        let inodes_ref = self.inodes.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();

        tokio::runtime::Handle::current().spawn(async move {
            let old_parent_path = {
                let inodes = inodes_ref.read().await;
                inodes.get(&parent).map(|e| e.path.clone())
            };
            let new_parent_path = {
                let inodes = inodes_ref.read().await;
                inodes.get(&new_parent).map(|e| e.path.clone())
            };
            let old_parent = match old_parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
            let new_parent = match new_parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };

            let old_path = if old_parent == "/" {
                format!("/{}", name_str)
            } else {
                format!("{}/{}", old_parent, name_str)
            };
            let new_path = if new_parent == "/" {
                format!("/{}", new_name_str)
            } else {
                format!("{}/{}", new_parent, new_name_str)
            };

            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            let dest_url = format!(
                "{}/{}",
                server_url,
                new_path.trim_start_matches('/')
            );
            let mut req = client
                .request(reqwest::Method::from_bytes(b"MOVE").unwrap(), &dest_url)
                .header(
                    "Destination",
                    format!(
                        "{}/{}",
                        server_url,
                        new_path.trim_start_matches('/')
                    ),
                )
                .header("Overwrite", "T");
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        reply.ok();
                    } else {
                        reply.error(libc::EIO);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        mode: u32,
        flags: u32,
        _umask: u32,
        reply: ReplyOpen,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let inodes_ref = self.inodes.clone();
        let server_url = self.server_url.clone();
        let auth = self.auth_header.clone();
        let inode_counter = self.inode_counter.clone();
        let file_handles = self.file_handles.clone();
        let fh_counter = self.fh_counter.clone();
        let uid = self.uid;
        let gid = self.gid;

        tokio::runtime::Handle::current().spawn(async move {
            let parent_path = {
                let inodes = inodes_ref.read().await;
                inodes.get(&parent).map(|e| e.path.clone())
            };
            let parent_path = match parent_path {
                Some(p) => p,
                None => {
                    reply.error(libc::ENOENT);
                    return;
                }
            };
            let child_path = if parent_path == "/" {
                format!("/{}", name_str)
            } else {
                format!("{}/{}", parent_path, name_str)
            };

            // PUT an empty file to create it
            let client = Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap();
            let url = format!(
                "{}/{}",
                server_url,
                child_path.trim_start_matches('/')
            );
            let mut req = client.put(&url).body(Vec::new());
            if let Some(ref token) = auth {
                req = req.header("Authorization", token.as_str());
            }

            match req.send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let ino = inode_counter.fetch_add(1, Ordering::SeqCst);
                        let entry = InodeEntry {
                            path: child_path,
                            ino,
                            is_dir: false,
                            size: 0,
                            modified: SystemTime::now(),
                        };
                        let attr = entry.to_file_attr(uid, gid);
                        inodes_ref.write().await.insert(ino, entry);
                        let fh = fh_counter.fetch_add(1, Ordering::SeqCst);
                        file_handles.write().await.insert(
                            fh,
                            FileHandleEntry {
                                path: entry.path,
                                flags,
                                ino,
                            },
                        );
                        reply.opened(fh, flags);
                    } else {
                        reply.error(libc::EIO);
                    }
                }
                Err(_) => {
                    reply.error(libc::EIO);
                }
            }
        });
    }

    fn release(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        let file_handles = self.file_handles.clone();
        tokio::runtime::Handle::current().spawn(async move {
            file_handles.write().await.remove(&fh);
            reply.ok();
        });
    }

    fn forget(&mut self, _req: &Request<'_>, _ino: u64, _nlookup: u64) {
        // No-op: inodes are kept in memory for simplicity
    }

    fn statfs(
        &mut self,
        _req: &Request<'_>,
        _ino: u64,
        reply: fuser::ReplyStatfs,
    ) {
        reply.statfs(0, 0, 0, 0, 0, BLOCK_SIZE, 255, 4096);
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let matches = clap::Command::new("ferro-fuse-cross")
        .about("Cross-platform FUSE mount for Ferro")
        .arg(
            clap::Arg::new("server-url")
                .long("server-url")
                .env("FERRO_URL")
                .default_value("http://localhost:8080")
                .help("Ferro server URL"),
        )
        .arg(
            clap::Arg::new("mount")
                .long("mount")
                .env("FERRO_MOUNT")
                .required(true)
                .help("Mount point path"),
        )
        .arg(
            clap::Arg::new("token")
                .long("token")
                .env("FERRO_TOKEN")
                .help("Bearer token for authentication"),
        )
        .arg(
            clap::Arg::new("allow-root")
                .long("allow-root")
                .action(clap::ArgAction::SetTrue)
                .help("Allow root access"),
        )
        .get_matches();

    let server_url = matches.get_one::<String>("server-url").unwrap().clone();
    let mount_point = matches.get_one::<String>("mount").unwrap().clone();
    let token = matches.get_one::<String>("token").map(|s| s.clone());
    let allow_root = matches.get_flag("allow-root");

    info!("Mounting Ferro at {} from {}", mount_point, server_url);

    // Create mount directory if needed
    std::fs::create_dir_all(&mount_point)?;

    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    let auth_header = token.map(|t| format!("Bearer {}", t));

    let mut fs = FerroFs::new(server_url, auth_header, uid, gid);

    let mut options = vec![
        MountOption::AutoUnmount,
        MountOption::FSName("ferro".to_string()),
        MountOption::Subtype("ferro".to_string()),
    ];
    if allow_root {
        options.push(MountOption::AllowOther);
    }

    // Run the FUSE event loop in a tokio runtime
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Spawn the FUSE mount in a blocking thread
        let mount_point_clone = mount_point.clone();
        let mount_handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mount_point = mount_point_clone;
                fuser::mount2(fs, &mount_point, &options).unwrap();
            });
        });

        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await?;
        info!("Unmounting...");

        // Try to unmount
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let _ = Command::new("fusermount")
                .args(["-u", &mount])
                .status();
        }
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let _ = Command::new("umount")
                .arg(&mount)
                .status();
        }

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
