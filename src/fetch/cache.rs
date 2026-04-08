//! File-based Response Cache
//!
//! Implements a simple XDG-compliant disk cache for HTTP responses
//! to avoid redundant network requests and improve speed for repeated
//! queries.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use directories::ProjectDirs;
use sha2::{Digest, Sha256};

use super::normalize::normalize;

pub const MAX_CACHE_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Disk-backed URL response cache.  Cache files are named by SHA-256 of the
/// normalised URL, keeping the filesystem flat and collision-free.
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    /// Create a cache rooted at `dir`.  The directory is created on first write.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Build a `Cache` using the XDG-compliant cache directory for ripweb
    /// (`~/.cache/ripweb` on Linux).  Returns `None` if the path cannot be
    /// determined.
    pub fn xdg() -> Option<Self> {
        let dirs = ProjectDirs::from("", "", "ripweb")?;
        Some(Self::new(dirs.cache_dir().to_path_buf()))
    }

    /// Return cached bytes for `url` if the entry exists and is < 24 hours old.
    pub async fn get(&self, url: &str) -> Option<Vec<u8>> {
        let path = self.cache_path(url);

        let meta = tokio::fs::metadata(&path).await.ok()?;
        let modified = meta.modified().ok()?;
        let age = SystemTime::now().duration_since(modified).ok()?;
        if age > MAX_CACHE_AGE {
            return None;
        }

        tokio::fs::read(&path).await.ok()
    }

    /// Persist `bytes` as the cached response for `url`.
    pub async fn put(&self, url: &str, bytes: &[u8]) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.dir).await?;
        tokio::fs::write(self.cache_path(url), bytes).await
    }

    /// Compute the filesystem path for the cache entry of `url`.
    ///
    /// The URL is normalised (fragment stripped, trailing slash stripped) then
    /// SHA-256 hashed to produce a flat, collision-free filename.
    pub fn cache_path(&self, url: &str) -> PathBuf {
        let canonical = normalize(url).unwrap_or_else(|| url.to_owned());
        let hash = hex::encode(Sha256::digest(canonical.as_bytes()));
        self.dir.join(hash)
    }
}
