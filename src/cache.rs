use crate::*;

/// DO NOT CALL THIS.
///
/// Clears the cache -- used for benchmarking and testing purposes to clear the
/// cache. Calling this will invalidate any previously created `UStr`s and
/// probably cause your house to burn down. DO NOT CALL THIS.
///
/// # Safety
///
/// DO NOT CALL THIS.
#[doc(hidden)]
pub unsafe fn _clear_cache() {
    for m in STRING_CACHE.0.iter() {
        m.lock().clear();
    }
}

/// Returns the total amount of memory allocated and in use by the cache in
/// bytes.
pub fn total_allocated() -> usize {
    STRING_CACHE
        .0
        .iter()
        .map(|sc| {
            let t = sc.lock().total_allocated();

            t
        })
        .sum()
}

/// Returns the total amount of memory reserved by the cache in bytes.
pub fn total_capacity() -> usize {
    STRING_CACHE
        .0
        .iter()
        .map(|sc| {
            let t = sc.lock().total_capacity();
            t
        })
        .sum()
}

/// Utility function to get a reference to the main cache object for use with
/// serialization.
///
/// # Examples
///
/// ```
/// # use ustr::{Ustr, ustr, ustr as u};
/// # #[cfg(feature="serde")]
/// # {
/// # unsafe { ustr::_clear_cache() };
/// ustr("Send me to JSON and back");
/// let json = serde_json::to_string(ustr::cache()).unwrap();
/// # }
pub fn cache() -> &'static Bins {
    &STRING_CACHE
}

/// Returns the number of unique strings in the cache.
///
/// This may be an underestimate if other threads are writing to the cache
/// concurrently.
///
/// # Examples
///
/// ```
/// use ustr::ustr as u;
///
/// let _ = u("Hello");
/// let _ = u(", World!");
/// assert_eq!(ustr::num_entries(), 2);
/// ```
pub fn num_entries() -> usize {
    STRING_CACHE
        .0
        .iter()
        .map(|sc| {
            let t = sc.lock().num_entries();
            t
        })
        .sum()
}

#[doc(hidden)]
pub fn num_entries_per_bin() -> Vec<usize> {
    STRING_CACHE
        .0
        .iter()
        .map(|sc| {
            let t = sc.lock().num_entries();
            t
        })
        .collect::<Vec<_>>()
}

/// Return an iterator over the entire string cache.
///
/// If another thread is adding strings concurrently to this call then they
/// might not show up in the view of the cache presented by this iterator.
///
/// # Safety
///
/// This returns an iterator to the state of the cache at the time when
/// `string_cache_iter()` was called. It is of course possible that another
/// thread will add more strings to the cache after this, but since we never
/// destroy the strings, they remain valid, meaning it's safe to iterate over
/// them, the list just might not be completely up to date.
pub fn string_cache_iter() -> StringCacheIterator {
    let mut allocs = Vec::new();
    for m in STRING_CACHE.0.iter() {
        let sc = m.lock();
        // the start of the allocator's data is actually the ptr, start() just
        // points to the beginning of the allocated region. The first bytes will
        // be uninitialized since we're bumping down
        for a in &sc.old_allocs {
            allocs.push((a.ptr(), a.end()));
        }
        let ptr = sc.alloc.ptr();
        let end = sc.alloc.end();
        if ptr != end {
            allocs.push((sc.alloc.ptr(), sc.alloc.end()));
        }
    }

    let current_ptr =
        allocs.first().map(|s| s.0).unwrap_or_else(std::ptr::null);

    StringCacheIterator {
        allocs,
        current_alloc: 0,
        current_ptr,
    }
}

/// The type used for the global string cache.
///
/// This is exposed to allow e.g. serialization of the data returned by the
/// [`cache()`] function.
#[repr(transparent)]
pub struct Bins(pub(crate) [Mutex<StringCache>; NUM_BINS]);
