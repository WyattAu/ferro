use bytes::Bytes;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{debug, warn};

pub const DEFAULT_STREAMING_THRESHOLD: u64 = 65536;

pub struct StreamingUploadWriter {
    file: Option<tokio::fs::File>,
    path: PathBuf,
    bytes_written: u64,
}

impl StreamingUploadWriter {
    pub async fn new(temp_dir: Option<&str>) -> std::io::Result<Self> {
        let temp_dir = match temp_dir {
            Some(d) => PathBuf::from(d),
            None => std::env::temp_dir(),
        };
        tokio::fs::create_dir_all(&temp_dir).await?;

        let path = temp_dir.join(format!(
            "ferro_upload_{}.tmp",
            uuid::Uuid::new_v4().simple()
        ));

        let file = tokio::fs::File::create(&path).await?;
        debug!(?path, "Created streaming upload temp file");

        Ok(Self {
            file: Some(file),
            path,
            bytes_written: 0,
        })
    }

    pub async fn write_chunk(&mut self, data: &[u8]) -> std::io::Result<()> {
        if let Some(ref mut f) = self.file {
            f.write_all(data).await?;
            self.bytes_written += data.len() as u64;
        }
        Ok(())
    }

    pub async fn finalize(mut self) -> std::io::Result<Bytes> {
        if let Some(ref mut f) = self.file {
            f.flush().await?;
            f.sync_all().await?;
        }
        self.file = None;

        let path = self.path.clone();
        let data = tokio::fs::read(&path).await?;
        let _ = tokio::fs::remove_file(&path).await;

        debug!(written = data.len(), ?path, "Streaming upload finalized");

        std::mem::forget(self);
        Ok(Bytes::from(data))
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

impl Drop for StreamingUploadWriter {
    fn drop(&mut self) {
        let path = self.path.clone();
        tokio::spawn(async move {
            if let Err(e) = tokio::fs::remove_file(&path).await {
                warn!(?path, error = %e, "Failed to cleanup temp upload file");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_writer_small_data() {
        let tmp = tempfile::tempdir().unwrap();
        let mut writer = StreamingUploadWriter::new(Some(tmp.path().to_str().unwrap()))
            .await
            .unwrap();

        writer.write_chunk(b"hello world").await.unwrap();
        assert_eq!(writer.bytes_written(), 11);

        let bytes = writer.finalize().await.unwrap();
        assert_eq!(&bytes[..], b"hello world");
    }

    #[tokio::test]
    async fn test_streaming_writer_multiple_chunks() {
        let tmp = tempfile::tempdir().unwrap();
        let mut writer = StreamingUploadWriter::new(Some(tmp.path().to_str().unwrap()))
            .await
            .unwrap();

        for i in 0..10 {
            writer.write_chunk(&[i; 1024]).await.unwrap();
        }
        assert_eq!(writer.bytes_written(), 10 * 1024);

        let bytes = writer.finalize().await.unwrap();
        assert_eq!(bytes.len(), 10 * 1024);
    }

    #[tokio::test]
    async fn test_streaming_writer_empty_data() {
        let tmp = tempfile::tempdir().unwrap();
        let writer = StreamingUploadWriter::new(Some(tmp.path().to_str().unwrap()))
            .await
            .unwrap();

        let bytes = writer.finalize().await.unwrap();
        assert!(bytes.is_empty());
    }

    #[tokio::test]
    async fn test_streaming_writer_large_data() {
        let tmp = tempfile::tempdir().unwrap();
        let mut writer = StreamingUploadWriter::new(Some(tmp.path().to_str().unwrap()))
            .await
            .unwrap();

        let chunk = vec![0xAB_u8; 64 * 1024];
        for _ in 0..16 {
            writer.write_chunk(&chunk).await.unwrap();
        }

        let bytes = writer.finalize().await.unwrap();
        assert_eq!(bytes.len(), 64 * 1024 * 16);
        assert!(bytes.iter().all(|&b| b == 0xAB));
    }

    #[tokio::test]
    async fn test_streaming_writer_cleanup_on_drop() {
        let tmp = tempfile::tempdir().unwrap();
        let path = {
            let writer = StreamingUploadWriter::new(Some(tmp.path().to_str().unwrap()))
                .await
                .unwrap();
            writer.path.clone()
        };

        assert!(path.exists());

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(!path.exists(), "Temp file should be cleaned up on drop");
    }
}
