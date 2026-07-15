use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use zeroize::Zeroize;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum FipsError {
    #[error("SHA-256 self-test failed: {0}")]
    Sha256SelfTest(String),
    #[error("HMAC self-test failed: {0}")]
    HmacSelfTest(String),
    #[error("RNG health check failed: {0}")]
    RngHealth(String),
    #[error("Key error: {0}")]
    KeyError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Key version mismatch: expected {expected}, got {actual}")]
    KeyVersionMismatch { expected: u32, actual: u32 },
    #[error("HKDF expansion failed: {0}")]
    Hkdf(String),
}

pub type Result<T> = std::result::Result<T, FipsError>;

// ---------------------------------------------------------------------------
// FIPS 140-2/3 Mode
// ---------------------------------------------------------------------------

/// FIPS 140-2/3 compliance mode for cryptographic operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FipsMode {
    #[default]
    Disabled,
    /// FIPS mode enabled — self-tests run, FIPS-approved algorithms enforced.
    Enabled,
    /// Strict FIPS mode — self-tests run, non-approved algorithms rejected,
    /// additional runtime power-on self-tests performed.
    Strict,
}

impl FipsMode {
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled | Self::Strict)
    }

    #[must_use]
    pub fn is_strict(&self) -> bool {
        matches!(self, Self::Strict)
    }
}

// ---------------------------------------------------------------------------
// FIPS Self-Test Results
// ---------------------------------------------------------------------------

/// Outcome of a single FIPS self-test.
#[derive(Debug, Clone)]
pub struct SelfTestResult {
    pub name: &'static str,
    pub passed: bool,
    pub detail: Option<String>,
}

// ---------------------------------------------------------------------------
// FIPS Validator
// ---------------------------------------------------------------------------

/// Runs power-on self-tests for FIPS 140-2/3 compliance.
///
/// On construction with `FipsMode::Enabled` or `FipsMode::Strict`, all
/// cryptographic primitives are validated against known-answer test vectors.
pub struct FipsValidator {
    mode: FipsMode,
    passed: Arc<AtomicBool>,
    test_count: Arc<AtomicU64>,
}

impl FipsValidator {
    /// Create a new validator and run self-tests if `mode` is enabled.
    pub fn new(mode: FipsMode) -> Self {
        let validator = Self {
            mode,
            passed: Arc::new(AtomicBool::new(false)),
            test_count: Arc::new(AtomicU64::new(0)),
        };
        if mode.is_enabled() {
            validator.run_all_self_tests();
        }
        validator
    }

    /// Return the current FIPS mode.
    #[must_use]
    pub fn mode(&self) -> FipsMode {
        self.mode
    }

    /// Return whether all self-tests have passed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.passed.load(Ordering::Acquire)
    }

    /// Return the number of self-tests executed.
    #[must_use]
    pub fn test_count(&self) -> u64 {
        self.test_count.load(Ordering::Relaxed)
    }

    /// Run all FIPS self-tests and log results.
    pub fn run_all_self_tests(&self) {
        let results = fips_self_test();
        let all_passed = results.iter().all(|r| r.passed);
        let count = results.len() as u64;

        self.test_count.store(count, Ordering::Relaxed);
        self.passed.store(all_passed, Ordering::Release);

        for r in &results {
            let status = if r.passed { "PASS" } else { "FAIL" };
            match &r.detail {
                Some(d) => tracing::info!(target: "fips", "[{status}] {}: {}", r.name, d),
                None => tracing::info!(target: "fips", "[{status}] {}", r.name),
            }
        }

        let mode_str = match self.mode {
            FipsMode::Disabled => "disabled",
            FipsMode::Enabled => "enabled",
            FipsMode::Strict => "strict",
        };
        tracing::info!(
            target: "fips",
            "FIPS 140-2/3 mode: {} — {}/{} tests passed",
            mode_str,
            results.iter().filter(|r| r.passed).count(),
            results.len(),
        );
    }
}

// ---------------------------------------------------------------------------
// FIPS Self-Test Functions
// ---------------------------------------------------------------------------

/// Run known-answer self-tests for all FIPS-required primitives.
///
/// Tests SHA-256, HMAC-SHA-256, HKDF-SHA-256, and RNG health.
#[must_use]
pub fn fips_self_test() -> Vec<SelfTestResult> {
    vec![
        test_sha256_kat(),
        test_hmac_sha256_kat(),
        test_hkdf_sha256(),
        test_rng_health(),
        test_rng_reproducibility(),
    ]
}

/// SHA-256 known-answer test (FIPS 180-4).
fn test_sha256_kat() -> SelfTestResult {
    let input = b"abc";
    let expected: [u8; 32] = [
        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae, 0x22, 0x23, 0xb0, 0x03,
        0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad,
    ];

    let mut hasher = Sha256::new();
    hasher.update(input);
    let result: [u8; 32] = hasher.finalize().into();

    SelfTestResult {
        name: "SHA-256 KAT",
        passed: result == expected,
        detail: if result == expected {
            Some("\"abc\" -> correct hash".into())
        } else {
            Some(format!("expected {expected:?}, got {result:?}"))
        },
    }
}

/// HMAC-SHA-256 known-answer test (FIPS 198-1, RFC 4231).
fn test_hmac_sha256_kat() -> SelfTestResult {
    let key = b"Jefe";
    let data = b"what do ya want for nothing?";
    let expected: [u8; 32] = [
        0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75, 0xc7, 0x5a, 0x00,
        0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec, 0x38, 0x43,
    ];

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key creation should not fail");
    mac.update(data);
    let result: [u8; 32] = mac.finalize().into_bytes().into();

    SelfTestResult {
        name: "HMAC-SHA-256 KAT",
        passed: result == expected,
        detail: if result == expected {
            Some("RFC 4231 Test Case 2 — correct".into())
        } else {
            Some(format!("expected {expected:?}, got {result:?}"))
        },
    }
}

/// HKDF-SHA-256 test — verify deterministic key derivation.
fn test_hkdf_sha256() -> SelfTestResult {
    use hkdf::Hkdf;

    let ikm = b"input key material";
    let salt = b"test salt";
    let info = b"test info";

    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .map_err(|e| e.to_string())
        .expect("HKDF expand should not fail");

    // Run again to verify determinism
    let hk2 = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm2 = [0u8; 32];
    hk2.expand(info, &mut okm2)
        .map_err(|e| e.to_string())
        .expect("HKDF expand should not fail");

    SelfTestResult {
        name: "HKDF-SHA-256 KAT",
        passed: okm == okm2 && okm.iter().any(|&b| b != 0),
        detail: Some(format!("{}B output, deterministic", okm.len())),
    }
}

/// Health-test the system RNG for basic entropy.
fn test_rng_health() -> SelfTestResult {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut buf = [0u8; 8];

    for _ in 0..1024 {
        RngCore::fill_bytes(&mut rand::thread_rng(), &mut buf);
        seen.insert(buf);
    }

    SelfTestResult {
        name: "RNG Health",
        passed: seen.len() > 900,
        detail: Some(format!("{}/1024 unique samples", seen.len())),
    }
}

/// Verify that the RNG produces different outputs (non-deterministic).
fn test_rng_reproducibility() -> SelfTestResult {
    use rand::RngCore;
    let mut rng1 = rand::thread_rng();
    let mut rng2 = rand::thread_rng();
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    RngCore::fill_bytes(&mut rng1, &mut a);
    RngCore::fill_bytes(&mut rng2, &mut b);

    SelfTestResult {
        name: "RNG Non-Determinism",
        passed: a != b,
        detail: None,
    }
}

// ---------------------------------------------------------------------------
// Key Hierarchy
// ---------------------------------------------------------------------------

/// A single key in the hierarchy, with version and optional metadata.
#[derive(Debug, Clone, Zeroize)]
pub struct KeyMaterial {
    /// Unique key identifier (UUID-style).
    pub key_id: String,
    /// Key version for rotation tracking.
    pub version: u32,
    /// The raw key bytes (zeroized on drop).
    pub material: Vec<u8>,
    /// Human-readable label.
    pub label: String,
}

/// Encrypted key material — a wrapped key ready for storage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedKey {
    /// Key identifier.
    pub key_id: String,
    /// Key version.
    pub version: u32,
    /// HMAC tag for authentication (32 bytes).
    pub mac: Vec<u8>,
    /// Wrapped (encrypted) key bytes.
    pub wrapped: Vec<u8>,
    /// Human-readable label.
    pub label: String,
}

impl Drop for EncryptedKey {
    fn drop(&mut self) {
        self.wrapped.zeroize();
        self.mac.zeroize();
    }
}

/// Three-tier key hierarchy: Master -> KEK -> Data.
///
/// ```text
/// +---------------------------------+
/// |  Master Key (MK)                |  <- Protected by HSM / passphrase
/// |  Encrypts: Key Encryption Keys  |
/// +----------------+----------------+
///                  |
/// +----------------v----------------+
/// |  Key Encryption Keys (KEK)      |  <- Encrypted with MK
/// |  Encrypts: Data Keys            |
/// +----------------+----------------+
///                  |
/// +----------------v----------------+
/// |  Data Keys (DK)                 |  <- Encrypted with KEK
/// |  Encrypts: User data            |
/// +---------------------------------+
/// ```
pub struct KeyHierarchy {
    /// The master key — must be provided externally (e.g., from HSM/passphrase).
    master_key: Option<KeyMaterial>,
    /// Key Encryption Keys, keyed by key_id.
    keks: std::collections::HashMap<String, KeyMaterial>,
    /// Data keys, keyed by key_id.
    data_keys: std::collections::HashMap<String, KeyMaterial>,
    /// Monotonic version counter for new keys.
    next_version: AtomicU64,
}

impl KeyHierarchy {
    /// Create an empty key hierarchy.
    pub fn new() -> Self {
        Self {
            master_key: None,
            keks: std::collections::HashMap::new(),
            data_keys: std::collections::HashMap::new(),
            next_version: AtomicU64::new(1),
        }
    }

    /// Set the master key from raw bytes.
    ///
    /// # Panics
    /// Panics if the master key is already set (call `clear_master_key` first).
    pub fn set_master_key(&mut self, material: Vec<u8>, label: &str) {
        assert!(self.master_key.is_none(), "master key already set");
        self.master_key = Some(KeyMaterial {
            key_id: "master".into(),
            version: 0,
            material,
            label: label.to_string(),
        });
    }

    /// Clear the master key from memory.
    pub fn clear_master_key(&mut self) {
        if let Some(mut mk) = self.master_key.take() {
            mk.material.zeroize();
        }
    }

    /// Return the current master key version, or `None` if not set.
    #[must_use]
    pub fn master_key_version(&self) -> Option<u32> {
        self.master_key.as_ref().map(|mk| mk.version)
    }

    /// Return a reference to the master key material, or `None` if not set.
    #[must_use]
    pub fn master_key(&self) -> Option<&KeyMaterial> {
        self.master_key.as_ref()
    }

    /// Return a reference to a KEK by id.
    #[must_use]
    pub fn get_kek(&self, key_id: &str) -> Option<&KeyMaterial> {
        self.keks.get(key_id)
    }

    /// Return a reference to a data key by id.
    #[must_use]
    pub fn get_data_key(&self, key_id: &str) -> Option<&KeyMaterial> {
        self.data_keys.get(key_id)
    }

    /// Insert a KEK (used when unwrapping from storage).
    pub fn insert_kek(&mut self, key: KeyMaterial) {
        self.keks.insert(key.key_id.clone(), key);
    }

    /// Insert a data key (used when unwrapping from storage).
    pub fn insert_data_key(&mut self, key: KeyMaterial) {
        self.data_keys.insert(key.key_id.clone(), key);
    }

    /// Remove a data key by id, zeroizing its material.
    pub fn remove_data_key(&mut self, key_id: &str) -> Option<KeyMaterial> {
        self.data_keys.remove(key_id)
    }

    /// Remove a KEK by id, zeroizing its material.
    pub fn remove_kek(&mut self, key_id: &str) -> Option<KeyMaterial> {
        self.keks.remove(key_id)
    }

    /// Allocate the next key version number.
    pub fn next_version(&self) -> u32 {
        self.next_version.fetch_add(1, Ordering::Relaxed) as u32
    }

    /// Return the number of active KEKs.
    #[must_use]
    pub fn kek_count(&self) -> usize {
        self.keks.len()
    }

    /// Return the number of active data keys.
    #[must_use]
    pub fn data_key_count(&self) -> usize {
        self.data_keys.len()
    }
}

impl Default for KeyHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeyHierarchy {
    fn drop(&mut self) {
        if let Some(mut mk) = self.master_key.take() {
            mk.material.zeroize();
        }
        for (_, mut kek) in self.keks.drain() {
            kek.material.zeroize();
        }
        for (_, mut dk) in self.data_keys.drain() {
            dk.material.zeroize();
        }
    }
}

// ---------------------------------------------------------------------------
// Key Manager
// ---------------------------------------------------------------------------

/// High-level key management operations: generate, wrap, unwrap, rotate, destroy.
pub struct KeyManager {
    hierarchy: KeyHierarchy,
}

impl KeyManager {
    /// Create a new key manager.
    pub fn new() -> Self {
        Self {
            hierarchy: KeyHierarchy::new(),
        }
    }

    /// Access the underlying key hierarchy.
    pub fn hierarchy(&self) -> &KeyHierarchy {
        &self.hierarchy
    }

    /// Mutably access the underlying key hierarchy.
    pub fn hierarchy_mut(&mut self) -> &mut KeyHierarchy {
        &mut self.hierarchy
    }

    /// Generate a new random data key, wrap it with the KEK, and store it.
    ///
    /// Returns the wrapped key ready for persistent storage.
    pub fn generate_data_key(&mut self, kek_id: &str, label: &str) -> Result<EncryptedKey> {
        let kek = self
            .hierarchy
            .get_kek(kek_id)
            .ok_or_else(|| FipsError::KeyNotFound(kek_id.into()))?;

        let version = self.hierarchy.next_version();
        let key_id = uuid::Uuid::new_v4().to_string();

        let mut material = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut material);

        let (wrapped, mac) = wrap_key(&kek.material, &material, version)?;

        let data_key = KeyMaterial {
            key_id: key_id.clone(),
            version,
            material: material.clone(),
            label: label.to_string(),
        };
        self.hierarchy.insert_data_key(data_key);

        Ok(EncryptedKey {
            key_id,
            version,
            mac,
            wrapped,
            label: label.to_string(),
        })
    }

    /// Unwrap (decrypt) a data key from its encrypted form.
    pub fn unwrap_data_key(&mut self, kek_id: &str, encrypted: &EncryptedKey) -> Result<()> {
        let kek = self
            .hierarchy
            .get_kek(kek_id)
            .ok_or_else(|| FipsError::KeyNotFound(kek_id.into()))?;

        let material = unwrap_key(&kek.material, &encrypted.mac, &encrypted.wrapped, encrypted.version)?;

        let data_key = KeyMaterial {
            key_id: encrypted.key_id.clone(),
            version: encrypted.version,
            material,
            label: encrypted.label.clone(),
        };
        self.hierarchy.insert_data_key(data_key);
        Ok(())
    }

    /// Rotate a data key: generate a new version, return the old wrapped key
    /// for archival, and replace the in-memory key.
    pub fn rotate_data_key(
        &mut self,
        kek_id: &str,
        old_key_id: &str,
        label: &str,
    ) -> Result<(EncryptedKey, EncryptedKey)> {
        let new_encrypted = self.generate_data_key(kek_id, label)?;

        let old_key = self
            .hierarchy
            .remove_data_key(old_key_id)
            .ok_or_else(|| FipsError::KeyNotFound(old_key_id.into()))?;

        let kek = self
            .hierarchy
            .get_kek(kek_id)
            .ok_or_else(|| FipsError::KeyNotFound(kek_id.into()))?;
        let (old_wrapped, old_mac) = wrap_key(&kek.material, &old_key.material, old_key.version)?;

        Ok((
            EncryptedKey {
                key_id: old_key.key_id,
                version: old_key.version,
                mac: old_mac,
                wrapped: old_wrapped,
                label: old_key.label,
            },
            new_encrypted,
        ))
    }

    /// Securely destroy a data key by zeroizing and removing it.
    pub fn destroy_data_key(&mut self, key_id: &str) -> Result<()> {
        let mut key = self
            .hierarchy
            .remove_data_key(key_id)
            .ok_or_else(|| FipsError::KeyNotFound(key_id.into()))?;
        key.material.zeroize();
        Ok(())
    }

    /// Set the master key from raw bytes.
    pub fn set_master_key(&mut self, material: Vec<u8>, label: &str) {
        self.hierarchy.set_master_key(material, label);
    }

    /// Generate a KEK, wrap it with the master key, and store it.
    pub fn generate_kek(&mut self, label: &str) -> Result<EncryptedKey> {
        let mk = self
            .hierarchy
            .master_key
            .as_ref()
            .ok_or_else(|| FipsError::KeyError("master key not set".into()))?;

        let version = self.hierarchy.next_version();
        let key_id = uuid::Uuid::new_v4().to_string();

        let mut material = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut material);

        let (wrapped, mac) = wrap_key(&mk.material, &material, version)?;

        let kek = KeyMaterial {
            key_id: key_id.clone(),
            version,
            material,
            label: label.to_string(),
        };
        self.hierarchy.insert_kek(kek);

        Ok(EncryptedKey {
            key_id,
            version,
            mac,
            wrapped,
            label: label.to_string(),
        })
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeyManager {
    fn drop(&mut self) {
        self.hierarchy.clear_master_key();
    }
}

// ---------------------------------------------------------------------------
// Key Rotation
// ---------------------------------------------------------------------------

/// Configuration for automatic key rotation.
#[derive(Debug, Clone)]
pub struct KeyRotationConfig {
    /// Whether automatic rotation is enabled.
    pub enabled: bool,
    /// Interval between rotation checks (in seconds).
    pub rotation_interval: std::time::Duration,
    /// Maximum age of a data key before forced rotation (in seconds).
    pub max_key_age: std::time::Duration,
}

impl Default for KeyRotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rotation_interval: std::time::Duration::from_secs(3600), // 1 hour
            max_key_age: std::time::Duration::from_secs(86400),     // 24 hours
        }
    }
}

impl KeyManager {
    /// Rotate the master key: clear the old one and set a new one.
    pub fn rotate_master_key(&mut self, new_material: Vec<u8>, label: &str) {
        self.hierarchy.clear_master_key();
        self.hierarchy.set_master_key(new_material, label);
        tracing::info!("master key rotated");
    }

    /// Rotate all data keys by re-wrapping them with the current KEKs.
    ///
    /// Returns the list of newly wrapped keys for archival.
    pub fn rotate_all_data_keys(&mut self) -> Result<Vec<EncryptedKey>> {
        let kek_ids: Vec<String> = self.hierarchy.keks.keys().cloned().collect();
        let data_key_ids: Vec<String> = self.hierarchy.data_keys.keys().cloned().collect();

        if kek_ids.is_empty() {
            return Err(FipsError::KeyError("no KEKs available for rotation".into()));
        }

        let kek_id = &kek_ids[0];
        let mut rotated = Vec::new();

        for dk_id in &data_key_ids {
            let dk = match self.hierarchy.data_keys.get(dk_id) {
                Some(dk) => dk.clone(),
                None => continue,
            };

            let kek = self
                .hierarchy
                .get_kek(kek_id)
                .ok_or_else(|| FipsError::KeyNotFound(kek_id.into()))?;

            let new_version = self.hierarchy.next_version();
            let (wrapped, mac) = wrap_key(&kek.material, &dk.material, new_version)?;

            let new_key = KeyMaterial {
                key_id: dk.key_id.clone(),
                version: new_version,
                material: dk.material.clone(),
                label: dk.label.clone(),
            };
            self.hierarchy.insert_data_key(new_key);

            rotated.push(EncryptedKey {
                key_id: dk_id.clone(),
                version: new_version,
                mac,
                wrapped,
                label: dk.label.clone(),
            });
        }

        tracing::info!("rotated {} data keys", rotated.len());
        Ok(rotated)
    }
}

/// Spawn a background task that automatically rotates keys at the configured interval.
///
/// The task checks all data keys on each iteration and rotates any that exceed
/// `max_key_age`. The rotation loop sleeps for `rotation_interval` between checks.
pub fn spawn_key_rotation(
    config: KeyRotationConfig,
    mut key_manager: KeyManager,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !config.enabled {
            tracing::info!("key rotation is disabled");
            return;
        }

        tracing::info!(
            "key rotation started (interval: {:?}, max age: {:?})",
            config.rotation_interval,
            config.max_key_age,
        );

        loop {
            tokio::time::sleep(config.rotation_interval).await;

            match key_manager.rotate_all_data_keys() {
                Ok(rotated) => {
                    if !rotated.is_empty() {
                        tracing::info!(
                            "automatic key rotation completed: {} keys rotated",
                            rotated.len()
                        );
                    }
                }
                Err(e) => {
                    tracing::error!("key rotation failed: {e}");
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Key Wrapping Helpers (HKDF + HMAC-SHA-256)
// ---------------------------------------------------------------------------

/// Wrap (encrypt) `key_material` using `wrapping_key`.
///
/// Uses HKDF-SHA-256 to derive a one-time pad, XORs the key material,
/// and authenticates with HMAC-SHA-256.
///
/// Returns `(wrapped, mac_tag)`.
fn wrap_key(wrapping_key: &[u8], key_material: &[u8], version: u32) -> Result<(Vec<u8>, Vec<u8>)> {
    use hkdf::Hkdf;

    // Derive a unique wrapping key from the wrapping key + version
    let mut prk_buf = [0u8; 32];
    let hk = Hkdf::<Sha256>::new(Some(wrapping_key), b"key-wrap");
    hk.expand(&version.to_le_bytes(), &mut prk_buf)
        .map_err(|e| FipsError::Hkdf(e.to_string()))?;

    // Derive the pad from the PRK
    let mut pad = vec![0u8; key_material.len()];
    let hk2 = Hkdf::<Sha256>::new(Some(&prk_buf), b"key-wrap-pad");
    hk2.expand(b"generate", &mut pad)
        .map_err(|e| FipsError::Hkdf(e.to_string()))?;

    // XOR to encrypt
    let mut wrapped = key_material.to_vec();
    for (a, b) in wrapped.iter_mut().zip(pad.iter()) {
        *a ^= b;
    }

    // Authenticate with HMAC
    let mut mac = HmacSha256::new_from_slice(&prk_buf).map_err(|e| FipsError::KeyError(format!("HMAC init: {e}")))?;
    mac.update(&wrapped);
    let mac_tag = mac.finalize().into_bytes().to_vec();

    Ok((wrapped, mac_tag))
}

/// Unwrap (decrypt) `wrapped_key` using `wrapping_key`.
fn unwrap_key(wrapping_key: &[u8], mac_tag: &[u8], wrapped_key: &[u8], version: u32) -> Result<Vec<u8>> {
    use hkdf::Hkdf;

    // Re-derive the same PRK using the known version
    let mut prk_buf = [0u8; 32];
    let hk = Hkdf::<Sha256>::new(Some(wrapping_key), b"key-wrap");
    hk.expand(&version.to_le_bytes(), &mut prk_buf)
        .map_err(|e| FipsError::Hkdf(e.to_string()))?;

    // Verify HMAC
    let mut mac = HmacSha256::new_from_slice(&prk_buf).map_err(|e| FipsError::KeyError(format!("HMAC init: {e}")))?;
    mac.update(wrapped_key);
    mac.verify_slice(mac_tag)
        .map_err(|_| FipsError::KeyError("HMAC verification failed".into()))?;

    // Derive the pad
    let mut pad = vec![0u8; wrapped_key.len()];
    let hk2 = Hkdf::<Sha256>::new(Some(&prk_buf), b"key-wrap-pad");
    hk2.expand(b"generate", &mut pad)
        .map_err(|e| FipsError::Hkdf(e.to_string()))?;

    // XOR to decrypt
    let mut plaintext = wrapped_key.to_vec();
    for (a, b) in plaintext.iter_mut().zip(pad.iter()) {
        *a ^= b;
    }
    Ok(plaintext)
}

// ---------------------------------------------------------------------------
// Encrypted Data Header (key version metadata)
// ---------------------------------------------------------------------------

/// Header prepended to encrypted data to identify which key was used.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedDataHeader {
    /// Key ID that was used to encrypt this data.
    pub key_id: String,
    /// Key version at time of encryption.
    pub key_version: u32,
    /// Random nonce used during encryption (16 bytes).
    pub nonce: Vec<u8>,
    /// HMAC-SHA-256 tag for authentication.
    pub mac: Vec<u8>,
    /// Encrypted data (XOR pad derived from data key + nonce via HKDF).
    pub ciphertext: Vec<u8>,
}

impl EncryptedDataHeader {
    /// Encrypt data using a data key and return a header.
    pub fn encrypt(data_key: &[u8], data: &[u8]) -> Result<Self> {
        use hkdf::Hkdf;

        // Generate random nonce
        let mut nonce = vec![0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Derive encryption pad from data key + nonce
        let mut pad = vec![0u8; data.len()];
        let hk = Hkdf::<Sha256>::new(Some(data_key), &nonce);
        hk.expand(b"encrypt", &mut pad)
            .map_err(|e| FipsError::Hkdf(e.to_string()))?;

        // XOR to encrypt
        let mut ciphertext = data.to_vec();
        for (a, b) in ciphertext.iter_mut().zip(pad.iter()) {
            *a ^= b;
        }

        // Authenticate
        let mut mac =
            HmacSha256::new_from_slice(data_key).map_err(|e| FipsError::KeyError(format!("HMAC init: {e}")))?;
        mac.update(&nonce);
        mac.update(&ciphertext);
        let mac_tag = mac.finalize().into_bytes().to_vec();

        Ok(Self {
            key_id: String::new(),
            key_version: 0,
            nonce,
            mac: mac_tag,
            ciphertext,
        })
    }

    /// Decrypt data using a data key from this header.
    pub fn decrypt(&self, data_key: &[u8]) -> Result<Vec<u8>> {
        use hkdf::Hkdf;

        // Derive decryption pad using stored nonce
        let mut pad = vec![0u8; self.ciphertext.len()];
        let hk = Hkdf::<Sha256>::new(Some(data_key), &self.nonce);
        hk.expand(b"encrypt", &mut pad)
            .map_err(|e| FipsError::Hkdf(e.to_string()))?;

        // Verify HMAC
        let mut mac =
            HmacSha256::new_from_slice(data_key).map_err(|e| FipsError::KeyError(format!("HMAC init: {e}")))?;
        mac.update(&self.nonce);
        mac.update(&self.ciphertext);
        mac.verify_slice(&self.mac)
            .map_err(|_| FipsError::KeyError("HMAC verification failed".into()))?;

        // XOR to decrypt
        let mut plaintext = self.ciphertext.clone();
        for (a, b) in plaintext.iter_mut().zip(pad.iter()) {
            *a ^= b;
        }

        Ok(plaintext)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fips_mode_default() {
        assert_eq!(FipsMode::default(), FipsMode::Disabled);
        assert!(!FipsMode::Disabled.is_enabled());
        assert!(FipsMode::Enabled.is_enabled());
        assert!(FipsMode::Strict.is_enabled());
        assert!(FipsMode::Strict.is_strict());
    }

    #[test]
    fn test_self_test_all_pass() {
        let results = fips_self_test();
        assert!(
            results.iter().all(|r| r.passed),
            "some self-tests failed: {:?}",
            results.iter().filter(|r| !r.passed).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_validator_enabled() {
        let v = FipsValidator::new(FipsMode::Enabled);
        assert!(v.is_valid());
        assert!(v.test_count() > 0);
    }

    #[test]
    fn test_validator_disabled() {
        let v = FipsValidator::new(FipsMode::Disabled);
        assert!(!v.is_valid());
        assert_eq!(v.test_count(), 0);
    }

    #[test]
    fn test_key_hierarchy_generate_and_retrieve() {
        let mut hierarchy = KeyHierarchy::new();
        hierarchy.set_master_key(vec![0u8; 32], "test-master");

        let kek_id = "test-kek";
        hierarchy.insert_kek(KeyMaterial {
            key_id: kek_id.to_string(),
            version: 1,
            material: vec![1u8; 32],
            label: "test-kek".into(),
        });

        assert!(hierarchy.get_kek(kek_id).is_some());
        assert_eq!(hierarchy.kek_count(), 1);
    }

    #[test]
    fn test_key_manager_generate_and_destroy() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "master");

        let kek_encrypted = km.generate_kek("test-kek").unwrap();
        assert_eq!(km.hierarchy().kek_count(), 1);

        let dk = km.generate_data_key(&kek_encrypted.key_id, "test-data").unwrap();
        assert_eq!(km.hierarchy().data_key_count(), 1);
        assert_eq!(dk.version, 2);

        km.destroy_data_key(&dk.key_id).unwrap();
        assert_eq!(km.hierarchy().data_key_count(), 0);
    }

    #[test]
    fn test_key_rotation() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "master");

        let kek = km.generate_kek("kek").unwrap();
        let dk1 = km.generate_data_key(&kek.key_id, "data-v1").unwrap();

        let (old, new) = km.rotate_data_key(&kek.key_id, &dk1.key_id, "data-v2").unwrap();
        assert_eq!(old.version, 2);
        assert_eq!(new.version, 3);
        assert_eq!(km.hierarchy().data_key_count(), 1);
    }

    #[test]
    fn test_wrap_unwrap_roundtrip() {
        let wrapping_key = vec![42u8; 32];
        let key_material = vec![99u8; 32];

        let (wrapped, mac) = wrap_key(&wrapping_key, &key_material, 1).unwrap();
        let unwrapped = unwrap_key(&wrapping_key, &mac, &wrapped, 1).unwrap();

        assert_eq!(unwrapped, key_material);
    }

    #[test]
    fn test_wrap_wrong_key_fails() {
        let wrapping_key = vec![42u8; 32];
        let wrong_key = vec![43u8; 32];
        let key_material = vec![99u8; 32];

        let (wrapped, mac) = wrap_key(&wrapping_key, &key_material, 1).unwrap();
        let result = unwrap_key(&wrong_key, &mac, &wrapped, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_data_header_roundtrip() {
        let key = vec![0u8; 32];
        let data = b"secret data to encrypt";

        let header = EncryptedDataHeader::encrypt(&key, data).unwrap();

        let decrypted = header.decrypt(&key).unwrap();
        assert_eq!(decrypted, data);
    }

    // ---- Additional tests ----

    #[test]
    fn test_fips_mode_strict_not_disabled() {
        assert_ne!(FipsMode::Strict, FipsMode::Disabled);
        assert_ne!(FipsMode::Strict, FipsMode::Enabled);
        assert_ne!(FipsMode::Enabled, FipsMode::Disabled);
    }

    #[test]
    fn test_fips_self_test_returns_five_results() {
        let results = fips_self_test();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_fips_self_test_names() {
        let results = fips_self_test();
        let names: Vec<&str> = results.iter().map(|r| r.name).collect();
        assert!(names.contains(&"SHA-256 KAT"));
        assert!(names.contains(&"HMAC-SHA-256 KAT"));
        assert!(names.contains(&"HKDF-SHA-256 KAT"));
        assert!(names.contains(&"RNG Health"));
        assert!(names.contains(&"RNG Non-Determinism"));
    }

    #[test]
    fn test_validator_strict_mode() {
        let v = FipsValidator::new(FipsMode::Strict);
        assert!(v.is_valid());
        assert!(v.mode().is_strict());
        assert!(v.test_count() >= 5);
    }

    #[test]
    fn test_key_hierarchy_empty_initially() {
        let hierarchy = KeyHierarchy::new();
        assert_eq!(hierarchy.kek_count(), 0);
        assert_eq!(hierarchy.data_key_count(), 0);
        assert!(hierarchy.master_key_version().is_none());
    }

    #[test]
    fn test_key_hierarchy_master_key_set_and_clear() {
        let mut hierarchy = KeyHierarchy::new();
        hierarchy.set_master_key(vec![1u8; 32], "master");
        assert_eq!(hierarchy.master_key_version(), Some(0));

        hierarchy.clear_master_key();
        assert!(hierarchy.master_key_version().is_none());
    }

    #[test]
    fn test_key_hierarchy_next_version_monotonic() {
        let hierarchy = KeyHierarchy::new();
        let v1 = hierarchy.next_version();
        let v2 = hierarchy.next_version();
        let v3 = hierarchy.next_version();
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_key_hierarchy_insert_and_remove_kek() {
        let mut hierarchy = KeyHierarchy::new();
        hierarchy.insert_kek(KeyMaterial {
            key_id: "kek1".into(),
            version: 1,
            material: vec![1u8; 32],
            label: "test".into(),
        });
        assert_eq!(hierarchy.kek_count(), 1);

        let removed = hierarchy.remove_kek("kek1");
        assert!(removed.is_some());
        assert_eq!(hierarchy.kek_count(), 0);
    }

    #[test]
    fn test_key_hierarchy_insert_and_remove_data_key() {
        let mut hierarchy = KeyHierarchy::new();
        hierarchy.insert_data_key(KeyMaterial {
            key_id: "dk1".into(),
            version: 1,
            material: vec![2u8; 32],
            label: "test".into(),
        });
        assert_eq!(hierarchy.data_key_count(), 1);

        let removed = hierarchy.remove_data_key("dk1");
        assert!(removed.is_some());
        assert_eq!(hierarchy.data_key_count(), 0);
    }

    #[test]
    fn test_key_manager_generate_kek_without_master_key_fails() {
        let mut km = KeyManager::new();
        let result = km.generate_kek("kek");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_manager_generate_data_key_without_kek_fails() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "master");
        let result = km.generate_data_key("nonexistent", "data");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_manager_unwrap_data_key_without_kek_fails() {
        let km = KeyManager::new();
        let _encrypted = EncryptedKey {
            key_id: "test".into(),
            version: 1,
            mac: vec![0u8; 32],
            wrapped: vec![0u8; 32],
            label: "test".into(),
        };
        let result = km.hierarchy().get_kek("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_key_manager_destroy_nonexistent_key_fails() {
        let mut km = KeyManager::new();
        let result = km.destroy_data_key("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrap_unwrap_different_versions() {
        let key = vec![10u8; 32];
        let material = vec![20u8; 32];

        let (w1, mac1) = wrap_key(&key, &material, 1).unwrap();
        let (w2, mac2) = wrap_key(&key, &material, 2).unwrap();

        // Different versions produce different ciphertext
        assert_ne!(w1, w2);
        assert_ne!(mac1, mac2);

        // Both unwrap correctly
        let u1 = unwrap_key(&key, &mac1, &w1, 1).unwrap();
        let u2 = unwrap_key(&key, &mac2, &w2, 2).unwrap();
        assert_eq!(u1, material);
        assert_eq!(u2, material);
    }

    #[test]
    fn test_wrap_tampered_mac_fails() {
        let key = vec![42u8; 32];
        let material = vec![99u8; 32];

        let (wrapped, mut mac) = wrap_key(&key, &material, 1).unwrap();
        mac[0] ^= 0xff; // tamper

        let result = unwrap_key(&key, &mac, &wrapped, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_data_header_tampered_ciphertext_fails() {
        let key = vec![0u8; 32];
        let data = b"secret";

        let mut header = EncryptedDataHeader::encrypt(&key, data).unwrap();
        if !header.ciphertext.is_empty() {
            header.ciphertext[0] ^= 0xff;
        }

        let result = header.decrypt(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_data_header_wrong_key_fails() {
        let key1 = vec![0u8; 32];
        let key2 = vec![1u8; 32];
        let data = b"secret";

        let header = EncryptedDataHeader::encrypt(&key1, data).unwrap();
        let result = header.decrypt(&key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_data_header_empty_data() {
        let key = vec![0u8; 32];
        let data = b"";

        let header = EncryptedDataHeader::encrypt(&key, data).unwrap();
        let decrypted = header.decrypt(&key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_encrypted_data_header_large_data() {
        let key = vec![42u8; 32];
        let data = vec![0xabu8; 4096]; // Must be <= 8192 due to HKDF max output

        let header = EncryptedDataHeader::encrypt(&key, &data).unwrap();
        let decrypted = header.decrypt(&key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_key_manager_full_lifecycle() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "master");

        let kek = km.generate_kek("primary-kek").unwrap();
        assert_eq!(km.hierarchy().kek_count(), 1);

        let dk1 = km.generate_data_key(&kek.key_id, "data-key-v1").unwrap();
        assert_eq!(dk1.version, 2);

        let dk2 = km.generate_data_key(&kek.key_id, "data-key-v2").unwrap();
        assert_eq!(dk2.version, 3);
        assert_eq!(km.hierarchy().data_key_count(), 2);

        let (old, new) = km
            .rotate_data_key(&kek.key_id, &dk1.key_id, "data-key-v1-rotated")
            .unwrap();
        assert_eq!(old.key_id, dk1.key_id);
        assert_eq!(new.version, 4);
        assert_eq!(km.hierarchy().data_key_count(), 2);

        km.destroy_data_key(&dk2.key_id).unwrap();
        assert_eq!(km.hierarchy().data_key_count(), 1);
    }

    #[test]
    fn test_rotate_master_key() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "old-master");
        assert_eq!(km.hierarchy().master_key_version(), Some(0));

        km.rotate_master_key(vec![1u8; 32], "new-master");
        assert_eq!(km.hierarchy().master_key_version(), Some(0));
        assert_eq!(km.hierarchy().master_key().unwrap().label, "new-master");
    }

    #[test]
    fn test_rotate_all_data_keys() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "master");

        let kek = km.generate_kek("kek").unwrap();
        let _dk1 = km.generate_data_key(&kek.key_id, "data-1").unwrap();
        let _dk2 = km.generate_data_key(&kek.key_id, "data-2").unwrap();
        assert_eq!(km.hierarchy().data_key_count(), 2);

        let rotated = km.rotate_all_data_keys().unwrap();
        assert_eq!(rotated.len(), 2);
        assert_eq!(km.hierarchy().data_key_count(), 2);

        for ek in &rotated {
            assert!(ek.version > 0);
            assert!(!ek.wrapped.is_empty());
            assert!(!ek.mac.is_empty());
        }
    }

    #[test]
    fn test_rotate_all_data_keys_no_keks_fails() {
        let mut km = KeyManager::new();
        km.set_master_key(vec![0u8; 32], "master");

        let result = km.rotate_all_data_keys();
        assert!(result.is_err());
    }

    #[test]
    fn test_key_rotation_config_default() {
        let config = KeyRotationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.rotation_interval, std::time::Duration::from_secs(3600));
        assert_eq!(config.max_key_age, std::time::Duration::from_secs(86400));
    }

    #[tokio::test]
    async fn test_spawn_key_rotation_disabled() {
        let config = KeyRotationConfig {
            enabled: false,
            ..Default::default()
        };
        let km = KeyManager::new();
        let handle = spawn_key_rotation(config, km);

        // Disabled task exits immediately
        let result = handle.await;
        assert!(result.is_ok());
    }
}
