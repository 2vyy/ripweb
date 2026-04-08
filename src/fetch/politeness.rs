//! Domain Politeness
//!
//! Enforces concurrent request limits per host to avoid overwhelming
//! target servers or triggering WAF blocks.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Semaphore;

/// A concurrent map of per-domain `Semaphore`s that enforces a maximum number
/// of in-flight requests per host.  Cloning is cheap — the inner map is
/// reference-counted.
#[derive(Clone)]
pub struct DomainSemaphores {
    map: Arc<DashMap<String, Arc<Semaphore>>>,
    max_per_host: usize,
}

impl DomainSemaphores {
    pub fn new(max_per_host: usize) -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            max_per_host,
        }
    }

    /// Acquire one permit for `host`, blocking until a slot is available.
    ///
    /// The host key is normalised to lowercase so that `Example.Com` and
    /// `EXAMPLE.COM` share the same semaphore.
    pub async fn acquire(&self, host: &str) -> Result<OwnedDomainPermit, crate::error::RipwebError> {
        let key = host.to_ascii_lowercase();
        let sem = self
            .map
            .entry(key)
            .or_insert_with(|| Arc::new(Semaphore::new(self.max_per_host)))
            .clone();

        // `acquire_owned` returns a permit that carries the Arc, keeping the
        // semaphore alive even if the map entry is evicted.
        let permit = Arc::clone(&sem)
            .acquire_owned()
            .await
            .map_err(|_| crate::error::RipwebError::Config("Semaphore closed".into()))?;

        Ok(OwnedDomainPermit { _permit: permit })
    }
}

/// RAII guard returned by `DomainSemaphores::acquire`.  Dropping it releases
/// the slot back to the semaphore.
pub struct OwnedDomainPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}
