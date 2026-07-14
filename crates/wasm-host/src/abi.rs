/// Stable WASM Plugin ABI for Ferro.
///
/// This module defines the versioned contract between the Ferro host runtime
/// and WASM plugins. All plugins compiled against this ABI version are
/// guaranteed to be loadable and callable by a compatible host.
pub const ABI_VERSION: u32 = 1;

/// Capabilities a plugin may declare in its manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Capability {
    /// Plugin can read files from the host filesystem (sandboxed).
    ReadFile = 0x01,
    /// Plugin can write files to the host filesystem (sandboxed).
    WriteFile = 0x02,
    /// Plugin can emit log messages through the host logger.
    Log = 0x04,
    /// Plugin can query file/object metadata.
    GetMetadata = 0x08,
    /// Plugin can respond to storage events.
    StorageAccess = 0x10,
}

impl Capability {
    /// Return the bit flag value for this capability.
    pub const fn flag(self) -> u32 {
        self as u32
    }

    /// Create a bitmask from a list of capabilities.
    pub const fn mask(caps: &[Capability]) -> u32 {
        let mut mask = 0u32;
        let mut i = 0;
        while i < caps.len() {
            mask |= caps[i].flag();
            i += 1;
        }
        mask
    }

    /// Check whether `mask` includes the given capability.
    pub const fn has(mask: u32, cap: Capability) -> bool {
        mask & cap.flag() != 0
    }
}

/// Permissions a plugin requests from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Permission {
    /// Access to the network (HTTP, sockets).
    Network = 0x01,
    /// Access to the filesystem.
    Filesystem = 0x02,
    /// Access to environment variables.
    EnvVars = 0x04,
    /// Ability to spawn child processes.
    Spawn = 0x08,
}

/// Describes a WASM plugin to the host before instantiation.
///
/// The manifest is the first piece of data the host reads from a plugin
/// binary. It tells the host which capabilities the plugin needs and which
/// permissions it requests, enabling least-privilege enforcement.
#[derive(Debug, Clone)]
pub struct PluginManifest {
    /// Human-readable plugin name.
    pub name: String,
    /// Semantic version string (e.g. "1.2.3").
    pub version: String,
    /// Bitmask of [`Capability`] flags this plugin provides to the host.
    pub capabilities: u32,
    /// Bitmask of [`Permission`] flags this plugin requires from the host.
    pub permissions: u32,
    /// Optional free-form description.
    pub description: String,
}

/// Host-side error codes that can be returned to guest plugins.
///
/// These correspond to the values returned from host functions when an
/// operation fails. Guest code should check the return value and match
/// against these codes to decide on recovery or abort.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum AbiError {
    /// Operation succeeded.
    Success = 0,
    /// The requested file was not found.
    FileNotFound = 1,
    /// Permission denied for the requested operation.
    PermissionDenied = 2,
    /// An I/O error occurred on the host side.
    IoError = 3,
    /// The provided arguments were invalid (null pointer, zero length, etc.).
    InvalidArgument = 4,
    /// The guest attempted to exceed its allocated memory.
    OutOfMemory = 5,
    /// The operation timed out.
    Timeout = 6,
    /// An internal host error with no specific classification.
    Internal = 7,
}

impl AbiError {
    /// Convert a raw integer code back into an [`AbiError`].
    pub fn from_u32(code: u32) -> Option<Self> {
        match code {
            0 => Some(AbiError::Success),
            1 => Some(AbiError::FileNotFound),
            2 => Some(AbiError::PermissionDenied),
            3 => Some(AbiError::IoError),
            4 => Some(AbiError::InvalidArgument),
            5 => Some(AbiError::OutOfMemory),
            6 => Some(AbiError::Timeout),
            7 => Some(AbiError::Internal),
            _ => None,
        }
    }
}

/// Functions the host exports to the guest plugin.
///
/// These are the `extern "C"` functions that a WASM plugin may import from
/// the `"ferro_host"` namespace. All pointer/length pairs refer to memory
/// owned by the guest; the host reads/writes through the guest's linear
/// memory.
pub mod host_exports {
    /// Read up to `max_len` bytes from `path_ptr`/`path_len` into the
    /// buffer at `buf_ptr`.
    ///
    /// Returns the number of bytes actually read, or an [`super::AbiError`]
    /// code on failure (negative values indicate error).
    pub const READ_FILE: &str = "read_file";

    /// Write `data_len` bytes from `data_ptr` to the file at
    /// `path_ptr`/`path_len`.
    ///
    /// Returns 0 on success or an [`super::AbiError`] code.
    pub const WRITE_FILE: &str = "write_file";

    /// Emit a log message. `level` follows the same semantics as the
    /// existing `host::log` function (0 = trace, 1 = debug, …).
    pub const LOG: &str = "log";

    /// Retrieve metadata for the object at `path_ptr`/`path_len`.
    ///
    /// Writes a `Metadata` struct into `out_ptr`. Returns 0 on success.
    pub const GET_METADATA: &str = "get_metadata";
}

/// Functions the guest plugin must export for the host to call.
///
/// These are the `extern "C"` functions that the host expects to find in
/// every plugin module. The host will call them during the plugin lifecycle.
pub mod guest_imports {
    /// Called when the host delivers an event to the plugin.
    ///
    /// `event_type` / `event_type_len` describe the event kind;
    /// `data_ptr` / `data_len` carry the event payload.
    ///
    /// Returns 0 on success or an error code.
    pub const ON_EVENT: &str = "on_event";

    /// Called when a file upload is directed to this plugin.
    ///
    /// `file_path_ptr` / `file_path_len` identify the uploaded file.
    ///
    /// Returns 0 on success or an error code.
    pub const ON_UPLOAD: &str = "on_upload";

    /// Called when the host notifies the plugin that a file has been deleted.
    ///
    /// `file_path_ptr` / `file_path_len` identify the removed file.
    ///
    /// Returns 0 on success or an error code.
    pub const ON_DELETE: &str = "on_delete";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_is_one() {
        assert_eq!(ABI_VERSION, 1);
    }

    #[test]
    fn capability_flags_are_distinct() {
        let flags = [
            Capability::ReadFile.flag(),
            Capability::WriteFile.flag(),
            Capability::Log.flag(),
            Capability::GetMetadata.flag(),
            Capability::StorageAccess.flag(),
        ];
        for (i, a) in flags.iter().enumerate() {
            for b in &flags[i + 1..] {
                assert_ne!(a, b, "duplicate capability flag");
            }
        }
    }

    #[test]
    fn capability_mask_roundtrip() {
        let caps = [Capability::ReadFile, Capability::WriteFile, Capability::Log];
        let mask = Capability::mask(&caps);
        assert!(Capability::has(mask, Capability::ReadFile));
        assert!(Capability::has(mask, Capability::WriteFile));
        assert!(Capability::has(mask, Capability::Log));
        assert!(!Capability::has(mask, Capability::GetMetadata));
        assert!(!Capability::has(mask, Capability::StorageAccess));
    }

    #[test]
    fn abi_error_from_u32_roundtrip() {
        for code in 0..=7 {
            let err = AbiError::from_u32(code).expect("valid error code");
            assert_eq!(err as u32, code);
        }
        assert!(AbiError::from_u32(8).is_none());
        assert!(AbiError::from_u32(u32::MAX).is_none());
    }

    #[test]
    fn manifest_creation() {
        let manifest = PluginManifest {
            name: "test-plugin".to_string(),
            version: "0.1.0".to_string(),
            capabilities: Capability::mask(&[Capability::ReadFile, Capability::Log]),
            permissions: Permission::Filesystem as u32,
            description: "A test plugin".to_string(),
        };
        assert_eq!(manifest.name, "test-plugin");
        assert!(Capability::has(manifest.capabilities, Capability::ReadFile));
        assert!(Capability::has(manifest.capabilities, Capability::Log));
        assert!(!Capability::has(manifest.capabilities, Capability::WriteFile));
    }

    #[test]
    fn host_export_names_are_nonempty() {
        assert!(!host_exports::READ_FILE.is_empty());
        assert!(!host_exports::WRITE_FILE.is_empty());
        assert!(!host_exports::LOG.is_empty());
        assert!(!host_exports::GET_METADATA.is_empty());
    }

    #[test]
    fn guest_import_names_are_nonempty() {
        assert!(!guest_imports::ON_EVENT.is_empty());
        assert!(!guest_imports::ON_UPLOAD.is_empty());
        assert!(!guest_imports::ON_DELETE.is_empty());
    }
}
