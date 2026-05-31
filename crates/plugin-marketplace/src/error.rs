use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarketplaceError {
    #[error("plugin not found: {id}")]
    PluginNotFound { id: String },
    #[error("version not found: {plugin_id} v{version}")]
    VersionNotFound { plugin_id: String, version: String },
    #[error("plugin already installed: {id}")]
    AlreadyInstalled { id: String },
    #[error("incompatible ABI: required {required}, plugin provides {plugin}")]
    IncompatibleAbi { required: String, plugin: String },
    #[error("download failed from {url}: {reason}")]
    DownloadFailed { url: url::Url, reason: String },
    #[error("verification failed for plugin {plugin_id}: {reason}")]
    VerificationFailed { plugin_id: String, reason: String },
    #[error("plugin quota exceeded: max {max_plugins} plugins")]
    QuotaExceeded { max_plugins: usize },
}
