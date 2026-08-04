//! Injected dependency faults, shared between the event handler that creates
//! them and the health prober that reports them.
//!
//! The registry lives here, and not in the server, because the `health-fault`
//! handler runs in this crate while the prober runs in the shell. Both hold the
//! same cheap handle.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use courses_core::Dependency;
use tokio::sync::RwLock;

/// One injected outage. `until` is `None` while it lasts until cleared by hand,
/// which is what `POST /health/simulate` creates.
#[derive(Debug, Clone, Copy)]
struct Fault {
    until: Option<Instant>,
}

impl Fault {
    fn is_active(self, now: Instant) -> bool {
        match self.until {
            None => true,
            Some(deadline) => now < deadline,
        }
    }
}

/// Cheap, cloneable handle over the injected faults.
#[derive(Clone, Default)]
pub struct HealthFaults {
    inner: Arc<RwLock<HashMap<Dependency, Fault>>>,
}

impl HealthFaults {
    /// Breaks a dependency. `duration` of `None` lasts until it is cleared.
    ///
    /// A second injection replaces the first, so a later, longer outage always
    /// wins over one already running.
    pub async fn inject(&self, dependency: Dependency, duration: Option<Duration>) {
        let until = duration.map(|d| Instant::now() + d);
        self.inner.write().await.insert(dependency, Fault { until });
    }

    /// Restores a dependency, whatever its deadline was.
    pub async fn clear(&self, dependency: Dependency) {
        self.inner.write().await.remove(&dependency);
    }

    /// Restores a dependency only if its deadline already passed. Returns
    /// whether it removed one.
    ///
    /// This is what a timer calls when it wakes: an outage injected after the
    /// timer started has a later deadline, so the stale timer leaves it alone.
    pub async fn expire(&self, dependency: Dependency) -> bool {
        let now = Instant::now();
        let mut faults = self.inner.write().await;
        match faults.get(&dependency) {
            Some(fault) if !fault.is_active(now) => {
                faults.remove(&dependency);
                true
            }
            _ => false,
        }
    }

    /// Whether this dependency is currently forced to report failure.
    pub async fn is_active(&self, dependency: Dependency) -> bool {
        let now = Instant::now();
        self.inner
            .read()
            .await
            .get(&dependency)
            .is_some_and(|fault| fault.is_active(now))
    }
}
