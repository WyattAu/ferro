pub mod backend;
pub mod composite;
pub mod error;
pub mod local;
pub mod memory;
pub mod nfs;
pub mod s3_mock;
pub mod smb;
#[cfg(feature = "smb")]
pub mod smb2_backend;

pub use backend::{BackendType, ObjectInfo, ObjectMetadata, StorageBackend};
pub use composite::CompositeBackend;
pub use error::StorageAdapterError;
pub use local::LocalFsBackend;
pub use memory::InMemoryBackend;
pub use nfs::{MockNfsBackend, MountInfo, NfsBackend};
pub use s3_mock::MockS3Backend;
pub use smb::{MockSmbBackend, SmbBackend, SmbCredentials};
#[cfg(feature = "smb")]
pub use smb2_backend::Smb2StorageBackend;
