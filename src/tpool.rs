use std::sync::{Arc, Mutex};

pub use crate::errors::{Error, Result};
use crate::htslib;

/// An HTSlib thread pool. Create a thread pool and use `set_thread_pool()` methods
/// to share a thread pool across multiple BAM readers & writers.
/// The Rust wrapper holds the htslib thread pool behind an `Arc<Mutex<_>>`, and an
/// `Arc` reference
/// to the thread pool is held by each reader / writer so you don't need to
/// explicitly manage the lifetime of the `ThreadPool`.
#[derive(Clone, Debug)]
pub struct ThreadPool {
    pub(crate) handle: Arc<Mutex<InnerThreadPool>>,
}

impl ThreadPool {
    /// Create a new thread pool with `n_threads` threads.
    pub fn new(n_threads: u32) -> Result<ThreadPool> {
        if n_threads == 0 || n_threads >= 256 {
            return Err(Error::ThreadPool);
        }
        let ret = unsafe { htslib::hts_tpool_init(n_threads as i32) };

        if ret.is_null() {
            Err(Error::ThreadPool)
        } else {
            let inner = htslib::htsThreadPool {
                pool: ret,
                // this matches the default size
                // used in hts_set_threads.
                qsize: n_threads as i32 * 2,
            };
            let inner = InnerThreadPool { inner };

            let handle = Arc::new(Mutex::new(inner));
            Ok(ThreadPool { handle })
        }
    }
}

/// Internal htsThreadPool
#[derive(Debug)]
pub struct InnerThreadPool {
    pub(crate) inner: htslib::htsThreadPool,
}

// HTSlib permits one hts_tpool to be attached to multiple independent file
// handles. Readers and writers retain an Arc until they close; the mutex only
// serializes access to the descriptor while a handle is being attached.
unsafe impl Send for InnerThreadPool {}

impl Drop for InnerThreadPool {
    fn drop(&mut self) {
        if !self.inner.pool.is_null() {
            unsafe {
                htslib::hts_tpool_destroy(self.inner.pool);
            }
        }

        self.inner.pool = std::ptr::null_mut();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn rejects_thread_counts_outside_supported_range() {
        assert_eq!(ThreadPool::new(0).unwrap_err(), Error::ThreadPool);
        assert_eq!(ThreadPool::new(256).unwrap_err(), Error::ThreadPool);
    }

    #[test]
    fn shared_pool_can_cross_thread_boundaries() {
        assert_send_sync::<ThreadPool>();

        let pool = ThreadPool::new(1).unwrap();
        let pool_on_other_thread = pool.clone();
        std::thread::spawn(move || drop(pool_on_other_thread))
            .join()
            .unwrap();
    }
}
