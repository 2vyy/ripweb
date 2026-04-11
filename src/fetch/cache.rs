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

// MAX_CACHE_AGE is now configurable via RipwebConfig or CLI.

/// Disk-backed URL response cache.  Cache files are named by SHA-256 of the
/// normalised URL, keeping the filesystem flat and collision-free.
pub struct Cache {
    dir: PathBuf,
    ttl: Duration,
}

impl Cache {
    /// Create a cache rooted at `dir` with a specific `ttl`.
    /// The directory is created on first write.
    pub fn new(dir: PathBuf, ttl: Duration) -> Self {
        Self { dir, ttl }
    }

    /// Build a `Cache` using the XDG-compliant cache directory for ripweb
    /// (`~/.cache/ripweb` on Linux) and a specific `ttl`.
    pub fn xdg(ttl: Duration) -> Option<Self> {
        let dirs = ProjectDirs::from("", "", "ripweb")?;
        Some(Self::new(dirs.cache_dir().to_path_buf(), ttl))
    }

    /// Return cached bytes for `url` if the entry exists and is < 24 hours old.
    pub async fn get(&self, url: &str) -> Option<Vec<u8>> {
        let path = self.cache_path(url);

        let meta = tokio::fs::metadata(&path).await.ok()?;
        let modified = meta.modified().ok()?;
        let age = SystemTime::now().duration_since(modified).ok()?;
        if age > self.ttl {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_cache_ttl() {
        let dir = tempdir().unwrap();
        let ttl = Duration::from_secs(1);
        let cache = Cache::new(dir.path().to_path_buf(), ttl);

        let url = "https://example.com/ttl";
        let content = b"test content";

        // Initial put
        cache.put(url, content).await.unwrap();

        // Immediate get -> HIT
        assert!(cache.get(url).await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Get after expiry -> MISS
        assert!(cache.get(url).await.is_none());
    }
}
