//! The site every page handler reads, behind one handle shared by both modes.

use std::sync::{Arc, RwLock};

use courses_core::RenderedSite;

/// The site currently served.
///
/// `Broken` carries the message from the last failed dev reload. In production
/// nothing ever writes the handle, so the variant cannot be reached there; the
/// handler arm still matches it, and answers with a loud 500 instead of
/// panicking or silently serving stale content.
#[derive(Debug)]
pub enum SiteState {
    Ready(RenderedSite),
    Broken(String),
}

/// A cheap, cloneable handle over the current [`SiteState`].
///
/// Readers clone the inner `Arc` out of the lock and drop the guard at once, so
/// no guard is ever held across an `await`. Production never takes the write
/// path, leaving one uncontended read per request.
#[derive(Clone)]
pub struct SiteHandle(Arc<RwLock<Arc<SiteState>>>);

impl SiteHandle {
    pub fn new(state: SiteState) -> Self {
        Self(Arc::new(RwLock::new(Arc::new(state))))
    }

    /// Clones the current state out of the lock.
    ///
    /// A poisoned lock is recovered rather than propagated: the only writer is
    /// the dev watcher, and a panic there must not take the server down.
    pub fn load(&self) -> Arc<SiteState> {
        match self.0.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poison) => {
                tracing::error!("SiteHandle read lock poisoned — recovering");
                Arc::clone(&poison.into_inner())
            }
        }
    }

    /// Replaces the current state.
    pub fn store(&self, state: SiteState) {
        let next = Arc::new(state);
        match self.0.write() {
            Ok(mut guard) => *guard = next,
            Err(poison) => {
                tracing::error!("SiteHandle write lock poisoned — recovering");
                *poison.into_inner() = next;
            }
        }
    }
}
