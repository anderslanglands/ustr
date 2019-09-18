//! Fast, FFI-friendly string interning. A `Ustr` (**U**nique **Str**) is a lightweight handle representing a static, immutable entry in a global string cache, allowing for:
//! * Extremely fast string assignment and comparisons - it's just a pointer comparison.
//! * Efficient storage -  only one copy of the string is held in memory, and getting access to it is just a pointer indirection.
//! * Fast hashing - the precomputed hash is stored with the string
//! * Fast FFI - the string is stored with a terminating null byte so can be passed to C directly without doing the CString dance.
//!
//! The downside is no strings are ever freed, so if you're creating lots and lots of strings, you might run out of memory. On the other hand, War and Peace
//! is only 3MB, so it's probably fine.
//!
//! This crate is based on [OpenImageIO's ustring](https://github.com/OpenImageIO/oiio/blob/master/src/include/OpenImageIO/ustring.h) but it is NOT binary-compatible (yet). The underlying hash map implementation is directy ported from OIIO.
//!
//! # Usage
//!
//! ```rust
//! use ustr::{Ustr, u};
//!
//! // Creation is quick and easy using either `Ustr::from` or the `u!` macro
//! // and only one copy of any string is stored
//! let h1 = Ustr::from("hello");
//! let h2 = u!("hello");
//!
//! // Comparisons and copies are extremely cheap
//! let h3 = h1;
//! assert_eq!(h2, h3);
//!
//! // You can pass straight to FFI
//! let len = unsafe {
//!     libc::strlen(h1.as_c_str())
//! };
//! assert_eq!(len, 5);
//! ```
//!
//! ## Why?
//! It is common in certain types of applications to use strings as identifiers,
//! but not really do any processing with them.
//! To paraphrase from OIIO's Ustring documentation -
//! Compared to standard strings, Ustrs have several advantages:
//!
//!   - Each individual Ustr is very small -- in fact, we guarantee that
//!     a Ustr is the same size and memory layout as an ordinary *u8.
//!   - Storage is frugal, since there is only one allocated copy of each
//!     unique character sequence, throughout the lifetime of the program.
//!   - Assignment from one Ustr to another is just copy of the pointer;
//!     no allocation, no character copying, no reference counting.
//!   - Equality testing (do the strings contain the same characters) is
//!     a single operation, the comparison of the pointer.
//!   - Memory allocation only occurs when a new Ustr is constructed from
//!     raw characters the FIRST time -- subsequent constructions of the
//!     same string just finds it in the canonial string set, but doesn't
//!     need to allocate new storage.  Destruction of a Ustr is trivial,
//!     there is no de-allocation because the canonical version stays in
//!     the set.  Also, therefore, no user code mistake can lead to
//!     memory leaks.
//!   - Creating a new Ustr is faster than String::from()
//!
//! But there are some problems, too.  Canonical strings are never freed
//! from the table.  So in some sense all the strings "leak", but they
//! only leak one copy for each unique string that the program ever comes
//! across.
//!
//! On the whole, Ustrs are a really great string representation
//!   - if you tend to have (relatively) few unique strings, but many
//!     copies of those strings;
//!   - if the creation of strings from raw characters is relatively
//!     rare compared to copying or comparing to existing strings;
//!   - if you tend to make the same strings over and over again, and
//!     if it's relatively rare that a single unique character sequence
//!     is used only once in the entire lifetime of the program;
//!   - if your most common string operations are assignment and equality
//!     testing and you want them to be as fast as possible;
//!   - if you are doing relatively little character-by-character assembly
//!     of strings, string concatenation, or other "string manipulation"
//!     (other than equality testing).
//!
//! Ustrs are not so hot
//!   - if your program tends to have very few copies of each character
//!     sequence over the entire lifetime of the program;
//!   - if your program tends to generate a huge variety of unique
//!     strings over its lifetime, each of which is used only a short
//!     time and then discarded, never to be needed again;
//!   - if you don't need to do a lot of string assignment or equality
//!     testing, but lots of more complex string manipulation.
//!
//! ## Safety and Compatibility
//! This crate has been tested (a little) on x86_64 ONLY. It might well do
//! horrible, horrible things on other architectures.
use spin::Mutex;
use std::fmt;

mod stringcache;
pub use stringcache::*;
mod bumpalloc;

/// A handle representing a string in the global string cache.
///
/// To use, create one using `Ustr::from` or the `u!` macro. You can freely
/// copy, destroy or send Ustrs to other threads: the underlying string is
/// always valid in memory (and is never destroyed).
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct Ustr {
    char_ptr: *const u8,
}

impl Ustr {
    /// Create a new Ustr from the given &str.
    ///
    /// You can also use the `u!` macro as a shorthand
    /// ```
    /// use ustr::{Ustr, u};
    ///
    /// let u1 = Ustr::from("constant-time comparisons rule");
    /// let u2 = u!("constant-time comparisons rule");
    /// assert_eq!(u1, u2);
    /// ```
    pub fn from(string: &str) -> Ustr {
        let hash = fasthash::city::hash64(string.as_bytes());
        let mut sc = STRING_CACHE[whichbin(hash)].lock();
        Ustr {
            char_ptr: sc.insert(string, hash),
        }
    }

    /// Get the cached string as a &str
    pub fn as_str(&self) -> &str {
        // This is safe if:
        // 1) self.char_ptr points to a valid address
        // 2) len is a usize stored usize aligned usize bytes before char_ptr
        // 3) char_ptr points to a valid UTF-8 string of len bytes.
        // All these are guaranteed by StringCache::insert() and by the fact
        // we can only construct a Ustr from a valid &str.
        unsafe {
            let len_ptr = (self.char_ptr as *const usize).offset(-1isize);
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                self.char_ptr,
                std::ptr::read(len_ptr),
            ))
        }
    }

    /// Get the cached string as a C char*.
    ///
    /// This includes the null terminator so is safe to pass straight to FFI.
    ///
    /// # Safety
    /// This is just passing a raw byte array with a null terminator to C.
    /// If your source string contains non-ascii bytes then this will pass them
    /// straight along with no checking.
    pub unsafe fn as_c_str(&self) -> *const std::os::raw::c_char {
        self.char_ptr as *const std::os::raw::c_char
    }

    /// Get the length (in bytes) of this string.
    pub fn len(&self) -> usize {
        // This is safe if:
        // 1) len is a usize stored usize aligned usize bytes before char_ptr
        // This is guaranteed by StringCache::insert()
        unsafe {
            let len_ptr = (self.char_ptr as *const usize).offset(-1isize);
            std::ptr::read(len_ptr)
        }
    }

    /// Get the precomputed hash for this string
    pub fn precomputed_hash(&self) -> u64 {
        // This is safe if:
        // 1) hash is a u64 stored 2*u64 aligned usize bytes before char_ptr
        // This is guaranteed by StringCache::insert()
        unsafe {
            let hash_ptr = (self.char_ptr as *const u64).offset(-2isize);
            std::ptr::read(hash_ptr)
        }
    }

    /// Get an owned String copy of this string.
    pub fn to_owned(&self) -> String {
        self.as_str().to_owned()
    }
}

// We're safe to impl these because the strings they reference are immutable
// and for all intents and purposes 'static since they're never deleted after
// being created
unsafe impl Send for Ustr {}
unsafe impl Sync for Ustr {}

impl PartialEq<&str> for Ustr {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Ustr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl AsRef<str> for Ustr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Ustr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for Ustr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "u!(\"{}\")", self.as_str())
    }
}

/// Shorthand macro for creating a Ustr.
///
/// ```
/// use ustr::{u, Ustr};
/// let u_hello = u!("Hello");
/// let u_world = u!("world");
/// println!("{}, {}!", u_hello, u_world);
/// // > Hello, world!
/// ```
#[macro_export]
macro_rules! u {
    ($s:expr) => {
        Ustr::from($s);
    };
}

// Clears the hash map. Used for benchmarking purposes. Do not call this.
#[doc(hidden)]
pub fn _clear_cache() {
    unsafe {
        for m in STRING_CACHE.iter() {
            m.lock().clear();
        }
    }
}

/// Returns the total amount of memory allocated and in use by the cache in bytes
pub fn total_allocated() -> usize {
    STRING_CACHE
        .iter()
        .map(|sc| sc.lock().total_allocated())
        .sum()
}

/// Returns the number of unique strings in the cache
///
/// This may be an underestimate if other threads are writing to the cache
/// concurrently.
///
/// ```
/// use ustr::{u, Ustr};
///
/// let _ = u!("Hello");
/// let _ = u!(", World!");
/// assert_eq!(ustr::num_entries(), 2);
/// ```
pub fn num_entries() -> usize {
    STRING_CACHE.iter().map(|sc| sc.lock().num_entries()).sum()
}

#[doc(hidden)]
pub fn num_entries_per_bin() -> Vec<usize> {
    STRING_CACHE
        .iter()
        .map(|sc| sc.lock().num_entries())
        .collect::<Vec<_>>()
}

/// Return an iterator over the entire string cache.
///
/// If another thread is adding strings concurrently to this call then they might
/// not show up in the view of the cache presented by this iterator.
///
/// # Safety
/// This returns an iterator to the state of the cache at the time when
/// `string_cache_iter()` was called. It is of course possible that another
/// thread will add more strings to the cache after this, but since we never
/// destroy the strings, they remain valid, meaning it's safe to iterate over
/// them, the list jsut might not be completely up to date.
pub fn string_cache_iter() -> StringCacheIterator {
    let mut allocs = Vec::new();
    for m in STRING_CACHE.iter() {
        let sc = m.lock();
        for a in &sc.old_allocs {
            allocs.push((a.start(), a.end()));
        }
        allocs.push((sc.alloc.start(), sc.alloc.end()));
    }

    let current_ptr = allocs[0].0;
    StringCacheIterator {
        allocs,
        current_alloc: 0,
        current_ptr,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::Ustr;

        let u_hello = u!("hello");
        assert_eq!(u_hello, "hello");
        let u_world = u!("world");
        assert_eq!(u_world, String::from("world"));

        println!("{}", std::mem::size_of::<spin::Mutex<super::StringCache>>());
    }

    #[test]
    fn c_str_works() {
        use super::Ustr;
        use std::ffi::CStr;

        let s_fox = "The quick brown fox jumps over the lazy dog.";
        let u_fox = u!(s_fox);
        let fox = unsafe { CStr::from_ptr(u_fox.as_c_str()) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(fox, s_fox);

        let s_odys = "Τη γλώσσα μου έδωσαν ελληνική";
        let u_odys = u!(s_odys);
        let odys = unsafe { CStr::from_ptr(u_odys.as_c_str()) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(odys, s_odys);
    }

    #[test]
    fn blns() {
        use super::{string_cache_iter, Ustr};
        use std::collections::HashSet;

        // clear the cache first or our results will be wrong
        super::_clear_cache();

        let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("data")
            .join("blns.txt");
        let blns = std::fs::read_to_string(path).unwrap();

        let mut hs = HashSet::new();
        for s in blns.split_whitespace() {
            hs.insert(s);
        }

        let mut us = Vec::new();
        let mut ss = Vec::new();

        for s in blns.split_whitespace().cycle().take(100_000) {
            let u = u!(s);
            us.push(u);
            ss.push(s.to_owned());
        }

        let mut hs_u = HashSet::new();
        for s in string_cache_iter() {
            hs_u.insert(s);
        }
        let diff: HashSet<_> = hs.difference(&hs_u).collect();

        // check that the number of entries is the same
        assert_eq!(super::num_entries(), hs.len());

        // check that we have the exact same (unique) strings in the cache as in
        // the source data
        assert_eq!(diff.iter().count(), 0);

        let nbs = super::num_entries_per_bin();
        println!("{:?}", nbs);
    }

    #[test]
    fn raft() {
        use super::{u, Ustr};
        use std::sync::Arc;

        let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("data")
            .join("raft-large-directories.txt");
        let raft = std::fs::read_to_string(path).unwrap();
        let raft = Arc::new(
            raft.split_whitespace()
                .collect::<Vec<_>>()
                .chunks(3)
                .map(|s| {
                    if s.len() == 3 {
                        format!("{}/{}/{}", s[0], s[1], s[2])
                    } else {
                        s[0].to_owned()
                    }
                })
                .collect::<Vec<_>>(),
        );

        let s = raft.clone();
        for _ in 0..600 {
            let mut v = Vec::with_capacity(20_000);
            super::_clear_cache();
            for s in s.iter().cycle().take(20_000) {
                v.push(u!(s));
            }
        }
    }
}

lazy_static::lazy_static! {
    // There's got to be a better way of doing this - macro?
    static ref STRING_CACHE: [Mutex<StringCache>; NUM_BINS] = [
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
        Mutex::new(StringCache::new()),
    ];
}

// Use the top bits of the hash to choose a bin
#[inline]
fn whichbin(hash: u64) -> usize {
    ((hash >> TOP_SHIFT as u64) % NUM_BINS as u64) as usize
}
