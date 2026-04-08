use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ripweb::fetch::cache::{Cache, MAX_CACHE_AGE};
use tempfile::TempDir;

fn temp_cache() -> (TempDir, Cache) {
    let dir = TempDir::new().unwrap();
    let cache = Cache::new(dir.path().to_path_buf());
    (dir, cache)
}

// ── Round-trip ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn put_then_get_returns_same_bytes() {
    let (_dir, cache) = temp_cache();
    let url = "https://example.com/page";
    let data = b"hello cached world";

    cache.put(url, data).await.unwrap();
    let got = cache.get(url).await.unwrap();
    assert_eq!(got, data);
}

#[tokio::test]
async fn get_returns_none_for_missing_entry() {
    let (_dir, cache) = temp_cache();
    assert!(cache.get("https://example.com/missing").await.is_none());
}

// ── Cache-key deduplication ───────────────────────────────────────────────────

/// Two URLs that are identical modulo fragment must share the same cache slot.
#[tokio::test]
async fn fragment_variants_share_cache_slot() {
    let (_dir, cache) = temp_cache();
    cache
        .put("https://docs.rs/tokio", b"tokio docs content")
        .await
        .unwrap();

    // The fragment variant must hit the same entry.
    let got = cache.get("https://docs.rs/tokio#structs").await;
    assert!(got.is_some(), "fragment variant did not hit cache");
    assert_eq!(got.unwrap(), b"tokio docs content");
}

// ── Staleness ─────────────────────────────────────────────────────────────────

/// A cache entry whose file modification time is > 24 hours ago must be treated
/// as missing (returns None).
#[tokio::test]
async fn stale_cache_entry_returns_none() {
    let (dir, cache) = temp_cache();
    let url = "https://example.com/stale";

    cache.put(url, b"old content").await.unwrap();

    // Backdate the file's mtime by 25 hours using filetime.
    // We reach into the temp dir to find the file, then set its mtime.
    let path = cache.cache_path(url);
    let old_time = SystemTime::now() - (MAX_CACHE_AGE + Duration::from_secs(3600));
    let ft = filetime::FileTime::from_system_time(old_time);
    filetime::set_file_mtime(&path, ft).unwrap();

    assert!(
        cache.get(url).await.is_none(),
        "stale entry was returned instead of None"
    );
}

/// A cache entry < 24 hours old must still be returned.
#[tokio::test]
async fn fresh_cache_entry_is_returned() {
    let (_dir, cache) = temp_cache();
    let url = "https://example.com/fresh";
    cache.put(url, b"fresh content").await.unwrap();
    // File was just written — mtime is now — must be fresh.
    assert!(cache.get(url).await.is_some());
}
