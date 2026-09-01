use anyhow::Result;
use bytes::Bytes;
use fuser::*;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

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
            ino: INodeNo(self.ino),
            size: self.size,
            blocks: self.size.div_ceil(512),
            atime: self.modified,
            mtime: self.modified,
            ctime: self.modified,
            crtime: UNIX_EPOCH,
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
            flags: 0,
            blksize: BLOCK_SIZE,
        }
    }
}

#[allow(dead_code)]
struct HeadResult {
    size: u64,
    is_collection: bool,
}

pub struct FerroFs {
    client: Client,
    server_url: String,
    auth_header: Option<String>,
    uid: u32,
    gid: u32,
    inodes: RwLock<HashMap<u64, InodeEntry>>,
    path_index: RwLock<HashMap<String, u64>>,
    file_handles: RwLock<HashMap<u64, String>>,
    fh_counter: AtomicU64,
    inode_counter: AtomicU64,
}

impl FerroFs {
    pub fn new(server_url: String, auth_header: Option<String>, uid: u32, gid: u32) -> Self {
        let fs = Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            server_url,
            auth_header,
            uid,
            gid,
            inodes: RwLock::new(HashMap::new()),
            path_index: RwLock::new(HashMap::new()),
            file_handles: RwLock::new(HashMap::new()),
            fh_counter: AtomicU64::new(1),
            inode_counter: AtomicU64::new(2),
        };
        fs.inodes.write().unwrap().insert(
            1,
            InodeEntry {
                path: "/".to_string(),
                ino: 1,
                is_dir: true,
                size: 0,
                modified: SystemTime::now(),
            },
        );
        fs.path_index.write().unwrap().insert("/".to_string(), 1);
        fs
    }

    fn dav_url(&self, path: &str) -> String {
        // The server automatically prepends /users/{sub}/ based on the auth token.
        // Strip /users/{sub}/ prefix so the server receives the correct relative path.
        let clean = path
            .strip_prefix("/users/")
            .and_then(|p| p.find('/').map(|i| &p[i + 1..]))
            .unwrap_or(path)
            .trim_start_matches('/');
        if clean.is_empty() {
            format!("{}/", self.server_url)
        } else {
            format!("{}/{}/", self.server_url, clean)
        }
    }

    fn get_or_create_inode(&self, path: &str, is_dir: bool, size: u64) -> u64 {
        {
            let index = self.path_index.read().unwrap();
            if let Some(&ino) = index.get(path) {
                return ino;
            }
        }
        let ino = self.inode_counter.fetch_add(1, Ordering::SeqCst);
        let entry = InodeEntry {
            path: path.to_string(),
            ino,
            is_dir,
            size,
            modified: SystemTime::now(),
        };
        self.inodes.write().unwrap().insert(ino, entry);
        self.path_index.write().unwrap().insert(path.to_string(), ino);
        ino
    }

    #[allow(dead_code)]
    fn webdav_head(&self, path: &str) -> Option<HeadResult> {
        let url = self.dav_url(path);
        let mut req = self.client.head(&url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        let resp = req.send().ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let headers = resp.headers().clone();
        let content_length = headers
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let is_collection = headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("httpd/unix-directory") || ct.contains("text/xml"))
            .unwrap_or(false);
        Some(HeadResult {
            size: content_length,
            is_collection,
        })
    }

    fn webdav_propfind_children(&self, path: &str) -> Result<Vec<(String, bool, u64)>, Errno> {
        let url = self.dav_url(path);
        tracing::debug!("PROPFIND url={} auth={}", url, self.auth_header.is_some());
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
        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("PROPFIND failed: {}", e);
                return Err(Errno::EIO);
            }
        };
        tracing::debug!("PROPFIND status={}", resp.status());
        if !resp.status().is_success() {
            return Err(Errno::EIO);
        }
        let xml = match resp.text() {
            Ok(t) => t,
            Err(_) => return Err(Errno::EIO),
        };
        Ok(parse_propfind_children(&xml, path))
    }

    fn webdav_get(&self, path: &str, offset: u64, size: u32) -> Option<Bytes> {
        let url = self.dav_url(path);
        let range = format!("bytes={}-{}", offset, offset + size as u64 - 1);
        let mut req = self.client.get(&url).header("Range", &range);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        let resp = req.send().ok()?;
        if resp.status().is_success() {
            resp.bytes().ok()
        } else {
            None
        }
    }

    fn webdav_put(&self, path: &str, data: &[u8]) -> bool {
        let url = self.dav_url(path);
        let mut req = self.client.put(&url).body(data.to_vec());
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        req.send().map(|r| r.status().is_success()).unwrap_or(false)
    }

    /// Read the entire file content.
    fn read_full(&self, path: &str) -> Option<Vec<u8>> {
        let url = self.dav_url(path);
        let mut req = self.client.get(&url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        let resp = req.send().ok()?;
        if resp.status().is_success() {
            resp.bytes().ok().map(|b| b.to_vec())
        } else {
            None
        }
    }

    /// Get the file size via HEAD request.
    fn get_file_size(&self, path: &str) -> Option<u64> {
        let url = self.dav_url(path);
        let mut req = self.client.head(&url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        let resp = req.send().ok()?;
        if resp.status().is_success() {
            resp.headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
        } else {
            None
        }
    }

    fn webdav_delete(&self, path: &str) -> bool {
        let url = self.dav_url(path);
        let mut req = self.client.delete(&url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        req.send().map(|r| r.status().is_success()).unwrap_or(false)
    }

    fn webdav_mkcol(&self, path: &str) -> bool {
        let url = self.dav_url(path);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url);
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        req.send().map(|r| r.status().is_success()).unwrap_or(false)
    }

    fn webdav_move(&self, src: &str, dst: &str) -> bool {
        let src_url = self.dav_url(src);
        let dst_url = self.dav_url(dst);
        let mut req = self
            .client
            .request(reqwest::Method::from_bytes(b"MOVE").unwrap(), &src_url)
            .header("Destination", &dst_url)
            .header("Overwrite", "T");
        if let Some(ref token) = self.auth_header {
            req = req.header("Authorization", token);
        }
        req.send().map(|r| r.status().is_success()).unwrap_or(false)
    }
}

#[allow(dead_code, unused_assignments)]
fn parse_propfind_children(xml: &str, parent_path: &str) -> Vec<(String, bool, u64)> {
    let mut children = Vec::new();
    let mut current_href: Option<String> = None;
    let mut current_is_collection = false;
    let mut current_size: u64 = 0;

    // XML may be all on one line — split by response tags instead of newlines
    let chunks: Vec<&str> = if xml.contains("</D:response>") {
        xml.split("</D:response>").collect()
    } else {
        xml.split("</d:response>").collect()
    };
    for response_chunk in chunks {
        current_href = None;
        current_is_collection = false;
        current_size = 0;

        // Extract href
        if let Some(start) = response_chunk.find("<D:href>").or_else(|| response_chunk.find("<d:href>")) {
            let s = start + response_chunk[start..].find('>').unwrap() + 1;
            if let Some(e) = response_chunk[s..].find('<') {
                current_href = Some(response_chunk[s..s + e].to_string());
            }
        }
        // Check for collection
        if response_chunk.contains("<D:collection") || response_chunk.contains("<d:collection") {
            current_is_collection = true;
        }
        // Extract content length
        if let Some(start) = response_chunk
            .find("<D:getcontentlength>")
            .or_else(|| response_chunk.find("<d:getcontentlength>"))
        {
            let s = start + response_chunk[start..].find('>').unwrap() + 1;
            if let Some(e) = response_chunk[s..].find('<') {
                current_size = response_chunk[s..s + e].parse().unwrap_or(0);
            }
        }

        if let Some(href) = current_href.take() {
            let normalized = href.trim_end_matches('/');
            let parent_normalized = parent_path.trim_end_matches('/');
            if normalized != parent_normalized {
                children.push((href, current_is_collection, current_size));
            }
        }
    }
    children
}

impl Filesystem for FerroFs {
    fn init(&mut self, _req: &Request, _config: &mut KernelConfig) -> io::Result<()> {
        info!("FerroFS mounted");
        Ok(())
    }

    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        let parent_path = match self.inodes.read().unwrap().get(&u64::from(parent)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        // PROPFIND the parent to discover children, then find by name
        let children = match self.webdav_propfind_children(&parent_path) {
            Ok(c) => c,
            Err(e) => {
                reply.error(e);
                return;
            }
        };
        for (href, is_dir, size) in &children {
            let child_name = href.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
            if child_name == name_str {
                let ino = self.get_or_create_inode(&href, *is_dir, *size);
                let entry = self.inodes.read().unwrap().get(&ino).cloned().unwrap();
                let attr = entry.to_file_attr(self.uid, self.gid);
                reply.entry(&TTL, &attr, fuser::Generation(0));
                return;
            }
        }

        reply.error(Errno::ENOENT);
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, _fh: Option<FileHandle>, reply: ReplyAttr) {
        match self.inodes.read().unwrap().get(&u64::from(ino)) {
            Some(entry) => {
                let attr = entry.to_file_attr(self.uid, self.gid);
                reply.attr(&TTL, &attr);
            }
            None => {
                reply.error(Errno::ENOENT);
            }
        }
    }

    fn readdir(&self, _req: &Request, ino: INodeNo, _fh: FileHandle, _offset: u64, mut reply: ReplyDirectory) {
        let parent_path = match self.inodes.read().unwrap().get(&u64::from(ino)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        tracing::debug!("readdir: ino={} parent_path={}", u64::from(ino), parent_path);
        let children = match self.webdav_propfind_children(&parent_path) {
            Ok(c) => c,
            Err(e) => {
                reply.error(e);
                return;
            }
        };
        tracing::debug!("readdir: got {} children", children.len());
        for (href, is_dir, size) in &children {
            tracing::debug!("  child: href={} is_dir={} size={}", href, is_dir, size);
        }

        for (href, is_dir, size) in &children {
            self.get_or_create_inode(href, *is_dir, *size);
        }

        let mut entries: Vec<(u64, FileType, String)> = Vec::new();
        entries.push((1, FileType::Directory, ".".to_string()));
        entries.push((1, FileType::Directory, "..".to_string()));

        for (href, is_dir, _size) in &children {
            let name = href.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }
            let child_ino = self.path_index.read().unwrap().get(href.as_str()).copied().unwrap_or(0);
            let kind = if *is_dir {
                FileType::Directory
            } else {
                FileType::RegularFile
            };
            entries.push((child_ino, kind, name));
        }

        for (i, (entry_ino, kind, name)) in entries.iter().enumerate() {
            if (i as u64) < _offset {
                continue;
            }
            if reply.add(INodeNo(*entry_ino), i as u64 + 1, *kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn open(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        let path = match self.inodes.read().unwrap().get(&u64::from(ino)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let fh = self.fh_counter.fetch_add(1, Ordering::SeqCst);
        self.file_handles.write().unwrap().insert(fh, path);
        reply.opened(FileHandle(fh), FopenFlags::empty());
    }

    fn read(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        size: u32,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyData,
    ) {
        let fh_id = u64::from(fh);
        let path = match self.file_handles.read().unwrap().get(&fh_id) {
            Some(p) => p.clone(),
            None => {
                reply.error(Errno::EBADF);
                return;
            }
        };
        match self.webdav_get(&path, offset, size) {
            Some(data) => reply.data(&data),
            None => reply.error(Errno::EIO),
        }
    }

    fn write(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: FileHandle,
        offset: u64,
        data: &[u8],
        _write_flags: WriteFlags,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        reply: ReplyWrite,
    ) {
        let fh_id = u64::from(fh);
        let path = match self.file_handles.read().unwrap().get(&fh_id) {
            Some(p) => p.clone(),
            None => {
                reply.error(Errno::EBADF);
                return;
            }
        };

        // For offset writes: read existing content, splice in the new data, PUT the result
        // For append (offset == file size): just PUT the new data
        if offset == 0 && data.len() as u64 >= self.get_file_size(&path).unwrap_or(0) {
            // Simple case: writing from start, new data is larger than existing
            if self.webdav_put(&path, data) {
                reply.written(data.len() as u32);
            } else {
                reply.error(Errno::EIO);
            }
        } else {
            // Complex case: need to read existing content and splice
            match self.read_full(&path) {
                Some(mut existing) => {
                    let end = (offset as usize) + data.len();
                    if end > existing.len() {
                        existing.resize(end, 0);
                    }
                    existing[offset as usize..end].copy_from_slice(data);
                    if self.webdav_put(&path, &existing) {
                        reply.written(data.len() as u32);
                    } else {
                        reply.error(Errno::EIO);
                    }
                }
                None => {
                    // File doesn't exist yet, just PUT the new data
                    if self.webdav_put(&path, data) {
                        reply.written(data.len() as u32);
                    } else {
                        reply.error(Errno::EIO);
                    }
                }
            }
        }
    }

    fn mkdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, _mode: u32, _umask: u32, reply: ReplyEntry) {
        let name_str = name.to_string_lossy().to_string();
        let parent_path = match self.inodes.read().unwrap().get(&u64::from(parent)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let child_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path.trim_end_matches('/'), name_str)
        };

        if self.webdav_mkcol(&child_path) {
            let ino = self.get_or_create_inode(&child_path, true, 0);
            let entry = self.inodes.read().unwrap().get(&ino).cloned().unwrap();
            let attr = entry.to_file_attr(self.uid, self.gid);
            reply.entry(&TTL, &attr, fuser::Generation(0));
        } else {
            reply.error(Errno::EIO);
        }
    }

    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let name_str = name.to_string_lossy().to_string();
        let parent_path = match self.inodes.read().unwrap().get(&u64::from(parent)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let child_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path.trim_end_matches('/'), name_str)
        };

        if self.webdav_delete(&child_path) {
            if let Some(ino) = self.path_index.write().unwrap().remove(&child_path) {
                self.inodes.write().unwrap().remove(&ino);
            }
            reply.ok();
        } else {
            reply.error(Errno::EIO);
        }
    }

    fn rmdir(&self, req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        self.unlink(req, parent, name, reply);
    }

    fn rename(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        new_parent: INodeNo,
        new_name: &OsStr,
        _flags: RenameFlags,
        reply: ReplyEmpty,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let new_name_str = new_name.to_string_lossy().to_string();

        let old_parent_path = match self.inodes.read().unwrap().get(&u64::from(parent)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let new_parent_path = match self.inodes.read().unwrap().get(&u64::from(new_parent)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };

        let old_path = if old_parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", old_parent_path.trim_end_matches('/'), name_str)
        };
        let new_path = if new_parent_path == "/" {
            format!("/{}", new_name_str)
        } else {
            format!("{}/{}", new_parent_path.trim_end_matches('/'), new_name_str)
        };

        if self.webdav_move(&old_path, &new_path) {
            if let Some(ino) = self.path_index.write().unwrap().remove(&old_path) {
                self.inodes.write().unwrap().remove(&ino);
            }
            reply.ok();
        } else {
            reply.error(Errno::EIO);
        }
    }

    fn create(
        &self,
        _req: &Request,
        parent: INodeNo,
        name: &OsStr,
        _mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        let name_str = name.to_string_lossy().to_string();
        let parent_path = match self.inodes.read().unwrap().get(&u64::from(parent)) {
            Some(e) => e.path.clone(),
            None => {
                reply.error(Errno::ENOENT);
                return;
            }
        };
        let child_path = if parent_path == "/" {
            format!("/{}", name_str)
        } else {
            format!("{}/{}", parent_path.trim_end_matches('/'), name_str)
        };

        if self.webdav_put(&child_path, b"") {
            let ino = self.get_or_create_inode(&child_path, false, 0);
            let entry = self.inodes.read().unwrap().get(&ino).cloned().unwrap();
            let attr = entry.to_file_attr(self.uid, self.gid);
            let fh = self.fh_counter.fetch_add(1, Ordering::SeqCst);
            self.file_handles.write().unwrap().insert(fh, child_path);
            reply.created(&TTL, &attr, fuser::Generation(0), FileHandle(fh), FopenFlags::empty());
        } else {
            reply.error(Errno::EIO);
        }
    }

    fn release(
        &self,
        _req: &Request,
        _ino: INodeNo,
        fh: FileHandle,
        _flags: OpenFlags,
        _lock_owner: Option<LockOwner>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        self.file_handles.write().unwrap().remove(&u64::from(fh));
        reply.ok();
    }

    fn forget(&self, _req: &Request, ino: INodeNo, _nlookup: u64) {
        // When the kernel drops all references to an inode, remove it from cache
        // to prevent unbounded memory growth.
        let ino_val = u64::from(ino);
        if let Some(entry) = self.inodes.write().unwrap().remove(&ino_val) {
            self.path_index.write().unwrap().remove(&entry.path);
        }
    }

    fn setattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        _size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<FileHandle>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<fuser::BsdFileFlags>,
        reply: ReplyAttr,
    ) {
        // For truncate (size: Some(0)), we just acknowledge it — the actual content
        // is managed by the server via PUT. For other attributes, return current attrs.
        match self.inodes.read().unwrap().get(&u64::from(ino)) {
            Some(entry) => {
                let attr = entry.to_file_attr(self.uid, self.gid);
                reply.attr(&TTL, &attr);
            }
            None => {
                reply.error(Errno::ENOENT);
            }
        }
    }

    fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        reply.statfs(0, 0, 0, 0, 0, BLOCK_SIZE, 255, 4096);
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".parse().unwrap()),
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
    let token = matches.get_one::<String>("token").cloned();
    let allow_root = matches.get_flag("allow-root");

    info!("Mounting Ferro at {} from {}", mount_point, server_url);

    std::fs::create_dir_all(&mount_point)?;

    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };

    let auth_header = token.map(|t| format!("Bearer {}", t));

    let fs = FerroFs::new(server_url, auth_header, uid, gid);

    let mut options = vec![
        MountOption::RW,
        MountOption::FSName("ferro".to_string()),
        MountOption::Subtype("ferro".to_string()),
    ];
    if allow_root {
        options.push(MountOption::CUSTOM("allow_other".to_string()));
    }

    let mut config = Config::default();
    config.mount_options = options;

    info!("Mounting filesystem...");
    fuser::mount(fs, &mount_point, &config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_propfind_children_empty() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:"></D:multistatus>"#;
        let children = parse_propfind_children(xml, "/");
        assert!(children.is_empty());
    }

    #[test]
    fn test_parse_propfind_children_single() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/file.txt</D:href><D:propstat><D:prop><D:getcontentlength>100</D:getcontentlength><D:resourcetype></D:resourcetype></D:prop></D:propstat></D:response></D:multistatus>"#;
        let children = parse_propfind_children(xml, "/");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, "/file.txt");
        assert!(!children[0].1); // not a collection
        assert_eq!(children[0].2, 100);
    }

    #[test]
    fn test_parse_propfind_children_collection() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/mydir</D:href><D:propstat><D:prop><D:getcontentlength>0</D:getcontentlength><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:propstat></D:response></D:multistatus>"#;
        let children = parse_propfind_children(xml, "/");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, "/mydir");
        assert!(children[0].1); // is a collection
    }

    #[test]
    fn test_parse_propfind_children_filter_parent() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/users/abc</D:href><D:propstat><D:prop><D:getcontentlength>0</D:getcontentlength><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:propstat></D:response><D:response><D:href>/users/abc/file.txt</D:href><D:propstat><D:prop><D:getcontentlength>50</D:getcontentlength><D:resourcetype></D:resourcetype></D:prop></D:propstat></D:response></D:multistatus>"#;
        let children = parse_propfind_children(xml, "/users/abc");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, "/users/abc/file.txt");
    }

    #[test]
    fn test_parse_propfind_children_single_line() {
        // Test that single-line XML (no newlines) works
        let xml = r#"<D:multistatus xmlns:D="DAV:"><D:response><D:href>/test</D:href><D:propstat><D:prop><D:getcontentlength>0</D:getcontentlength><D:resourcetype><D:collection/></D:resourcetype></D:prop></D:propstat></D:response></D:multistatus>"#;
        let children = parse_propfind_children(xml, "/");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].0, "/test");
        assert!(children[0].1);
    }

    #[test]
    fn test_dav_url_strips_user_prefix() {
        let fs = FerroFs::new("https://example.com".to_string(), None, 0, 0);
        let url = fs.dav_url("/users/abc/file.txt");
        assert!(url.starts_with("https://example.com/"));
        assert!(url.contains("file.txt"));
        // URL has trailing slash
        assert!(url.ends_with('/'));
    }

    #[test]
    fn test_dav_url_root() {
        let fs = FerroFs::new("https://example.com".to_string(), None, 0, 0);
        assert_eq!(fs.dav_url("/"), "https://example.com/");
    }
}
