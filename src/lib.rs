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
//! use ustr::{Ustr, ustr, ustr as u};
//!
//! # unsafe { ustr::_clear_cache() };
//! // Creation is quick and easy using either `Ustr::from` or the ustr function
//! // and only one copy of any string is stored
//! let u1 = Ustr::from("the quick brown fox");
//! let u2 = ustr("the quick brown fox");
//!
//! // Comparisons and copies are extremely cheap
//! let u3 = u1;
//! assert_eq!(u2, u3);
//!
//! // You can pass straight to FFI
//! let len = unsafe {
//!     libc::strlen(u1.as_char_ptr())
//! };
//! assert_eq!(len, 19);
//!
//! // Use as_str() to get a &str
//! let words: Vec<&str> = u1.as_str().split_whitespace().collect();
//! assert_eq!(words, ["the", "quick", "brown", "fox"]);
//!
//! // For best performance when using Ustr as key for a HashMap or HashSet,
//! // you'll want to use the precomputed hash. To make this easier, just use
//! // the UstrMap and UstrSet exports:
//! use ustr::UstrMap;
//!
//! // Key type is always Ustr
//! let mut map: UstrMap<usize> = UstrMap::default();
//! map.insert(u1, 17);
//! assert_eq!(*map.get(&u1).unwrap(), 17);
//! ```
//!
//!
//! By enabling the `"serialize"` feature you can serialize individual `Ustr`s or the whole cache with serde.
//!
//! ```rust
//! use ustr::{Ustr, ustr};
//! let u_ser = ustr("serialization is fun!");
//! let json = serde_json::to_string(&u_ser).unwrap();
//! let u_de : Ustr = serde_json::from_str(&json).unwrap();
//! assert_eq!(u_ser, u_de);
//! ```
//!
//! Since the cache is global, use the `ustr::DeserializedCache` dummy object to drive the deserialization.
//!
//! ```rust
//! use ustr::{Ustr, ustr};
//! ustr("Send me to JSON and back");
//! let json = serde_json::to_string(ustr::get_cache()).unwrap();
//!
//! // ... some time later ...
//! let _: ustr::DeserializedCache = serde_json::from_str(&json).unwrap();
//! assert_eq!(ustr::num_entries(), 1);
//! assert_eq!(ustr::string_cache_iter().collect::<Vec<_>>(), vec!["Send me to JSON and back"]);
//!
//! ```
//!
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
//! This crate has been tested on x86_64 ONLY. Compilation will fail with a
//! static assert if the pointer size is not 64 bits.
#[cfg(not(feature = "spinlock"))]
use parking_lot::Mutex;
#[cfg(feature = "spinlock")]
use spin::Mutex;

use std::fmt;

mod stringcache;
pub use stringcache::*;
#[cfg(feature = "serialization")]
pub mod serialization;
#[cfg(feature = "serialization")]
pub use serialization::DeserializedCache;

mod bumpalloc;

mod hash;
pub use hash::*;
use std::hash::{Hash, Hasher};

/// A handle representing a string in the global string cache.
///
/// To use, create one using `Ustr::from` or the `ustr` function. You can freely
/// copy, destroy or send Ustrs to other threads: the underlying string is
/// always valid in memory (and is never destroyed).
#[derive(Copy, Clone, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Ustr {
    char_ptr: *const u8,
}

impl Ustr {
    /// Create a new Ustr from the given &str.
    ///
    /// You can also use the ustr function
    /// ```
    /// use ustr::{Ustr, ustr as u};
    /// # unsafe { ustr::_clear_cache() };
    ///
    /// let u1 = Ustr::from("the quick brown fox");
    /// let u2 = u("the quick brown fox");
    /// assert_eq!(u1, u2);
    /// assert_eq!(ustr::num_entries(), 1);
    /// ```
    pub fn from(string: &str) -> Ustr {
        #[cfg(feature = "hashcity")]
        let hash = fasthash::city::hash64(string.as_bytes());
        #[cfg(not(feature = "hashcity"))]
        let hash = {
            let mut hasher = ahash::AHasher::new_with_keys(123, 456);
            hasher.write(string.as_bytes());
            hasher.finish()
        };
        let mut sc = STRING_CACHE.0[whichbin(hash)].lock();
        Ustr {
            char_ptr: sc.insert(string, hash),
        }
    }

    /// Get the cached string as a &str
    /// ```
    /// use ustr::ustr as u;
    /// # unsafe { ustr::_clear_cache() };
    ///
    /// let u_fox = u("the quick brown fox");
    /// let words: Vec<&str> = u_fox.as_str().split_whitespace().collect();
    /// assert_eq!(words, ["the", "quick", "brown", "fox"]);
    /// ```
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
    /// ```
    /// use ustr::ustr as u;
    /// # unsafe { ustr::_clear_cache() };
    ///
    /// let u_fox = u("the quick brown fox");
    /// let len = unsafe {
    ///     libc::strlen(u_fox.as_char_ptr())
    /// };
    /// assert_eq!(len, 19);
    /// ```
    ///
    /// # Safety
    /// This is just passing a raw byte array with a null terminator to C.
    /// If your source string contains non-ascii bytes then this will pass them
    /// straight along with no checking.
    /// The string is **immutable**. That means that if you modify it across the
    /// FFI boundary then all sorts of terrible things will happen.
    pub unsafe fn as_char_ptr(&self) -> *const std::os::raw::c_char {
        self.char_ptr as *const std::os::raw::c_char
    }

    /// Get the length (in bytes) of this string.
    pub fn len(&self) -> usize {
        // This is safe if:
        // 1) len is a usize stored usize-aligned usize bytes before char_ptr
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

impl Eq for Ustr {}

impl AsRef<str> for Ustr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for Ustr {
    fn from(s: &str) -> Ustr {
        Ustr::from(s)
    }
}

impl From<String> for Ustr {
    fn from(s: String) -> Ustr {
        Ustr::from(&s)
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

// Just feed the precomputed hash into the Hasher. Note that this will of course
// be terrible unless the Hasher in question is expecting a precomputed hash.
impl Hash for Ustr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.precomputed_hash().hash(state);
    }
}

/// DO NOT CALL THIS.
///
/// Clears the cache - used for benchmarking and testing purposes to clear the
/// cache. Calling this will invalidate any previously created `UStr`s and
/// probably cause your house to burn down. DO NOT CALL THIS.
///
/// # Safety
/// DO NOT CALL THIS.
#[doc(hidden)]
pub unsafe fn _clear_cache() {
    for m in STRING_CACHE.0.iter() {
        m.lock().clear();
    }
}

/// Returns the total amount of memory allocated and in use by the cache in bytes
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

/// Returns the total amount of memory reserved by the cache in bytes
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

/// Create a new Ustr from the given &str.
///
/// ```
/// use ustr::ustr;
/// # unsafe { ustr::_clear_cache() };
///
/// let u1 = ustr("the quick brown fox");
/// let u2 = ustr("the quick brown fox");
/// assert_eq!(u1, u2);
/// assert_eq!(ustr::num_entries(), 1);
/// ```
pub fn ustr(s: &str) -> Ustr {
    Ustr::from(s)
}

/// Utility function to get a reference to the main cache object for use with
/// serialization.
///
/// # Examples
/// ```
/// # use ustr::{Ustr, ustr, ustr as u};
/// # #[cfg(feature="serialization")]
/// # {
/// # unsafe { ustr::_clear_cache() };
/// ustr("Send me to JSON and back");
/// let json = serde_json::to_string(ustr::get_cache()).unwrap();
/// # }
pub fn get_cache() -> &'static Bins {
    &*STRING_CACHE
}

/// Returns the number of unique strings in the cache
///
/// This may be an underestimate if other threads are writing to the cache
/// concurrently.
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
/// If another thread is adding strings concurrently to this call then they might
/// not show up in the view of the cache presented by this iterator.
///
/// # Safety
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

    let current_ptr = allocs[0].0;
    StringCacheIterator {
        allocs,
        current_alloc: 0,
        current_ptr,
    }
}

#[repr(transparent)]
pub struct Bins(pub(crate) [Mutex<StringCache>; NUM_BINS]);

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::ustr as u;

        let u_hello = u("hello");
        assert_eq!(u_hello, "hello");
        let u_world = u("world");
        assert_eq!(u_world, String::from("world"));
    }

    #[test]
    fn empty_string() {
        use super::ustr as u;

        unsafe {
            super::_clear_cache();
        }

        let _empty = u("");
        let empty = u("");

        assert_eq!(empty.as_str().is_empty(), true);
        assert_eq!(super::num_entries(), 1);
    }

    #[test]
    fn c_str_works() {
        use super::ustr as u;
        use std::ffi::CStr;

        let s_fox = "The quick brown fox jumps over the lazy dog.";
        let u_fox = u(s_fox);
        let fox = unsafe { CStr::from_ptr(u_fox.as_char_ptr()) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(fox, s_fox);

        let s_odys = "Τη γλώσσα μου έδωσαν ελληνική";
        let u_odys = u(s_odys);
        let odys = unsafe { CStr::from_ptr(u_odys.as_char_ptr()) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(odys, s_odys);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn blns() {
        use super::{string_cache_iter, ustr as u};
        use std::collections::HashSet;

        // clear the cache first or our results will be wrong
        unsafe { super::_clear_cache() };

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
            let u = u(s);
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

        println!("Total allocated: {}", super::total_allocated());
        println!("Total capacity: {}", super::total_capacity());

        println!(
            "size of StringCache: {}",
            std::mem::size_of::<super::StringCache>()
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn raft() {
        use super::ustr as u;
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
            unsafe { super::_clear_cache() };
            for s in s.iter().cycle().take(20_000) {
                v.push(u(s));
            }
        }
    }

    // This test is to have miri check the allocation code paths, but miri
    // can't open files so it's not usable right now
    // #[test]
    // fn words() {
    //     use super::ustr as u;
    //     use std::sync::Arc;

    //     let path = std::path::Path::new("/usr/share/dict/words");
    //     let wordlist = std::fs::read_to_string(path).unwrap();
    //     let wordlist = Arc::new(
    //         wordlist
    //             .split_whitespace()
    //             .collect::<Vec<_>>()
    //             .chunks(7)
    //             .cycle()
    //             .take(4_000_000)
    //             .enumerate()
    //             .map(|(i, s)| u(&format!("{}{}", i, s.join("-"))))
    //             .collect::<Vec<_>>(),
    //     );
    // }

    #[cfg(feature = "serialization")]
    #[test]
    fn serialization() {
        use super::{string_cache_iter, ustr as u};
        use std::collections::HashSet;

        // clear the cache first or our results will be wrong
        unsafe { super::_clear_cache() };

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
            let u = u(s);
            us.push(u);
            ss.push(s.to_owned());
        }

        let json = serde_json::to_string(super::get_cache()).unwrap();
        unsafe {
            super::_clear_cache();
        }
        let _: super::DeserializedCache = serde_json::from_str(&json).unwrap();

        // now check that we've got the same data in the cache still
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
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn serialization_ustr() {
        use super::{ustr, Ustr};

        let u_hello = ustr("hello");

        let json = serde_json::to_string(&u_hello).unwrap();
        let me_hello: Ustr = serde_json::from_str(&json).unwrap();

        assert_eq!(u_hello, me_hello);
    }
}

lazy_static::lazy_static! {
    static ref STRING_CACHE: Bins = {
        use std::mem::{self, MaybeUninit};
        // This deeply unsafe feeling dance allows us to initialize an array of
        // arbitrary size and will have to tide us over until const generics
        // land. See:
        // https://doc.rust-lang.org/beta/std/mem/union.MaybeUninit.html#initializing-an-array-element-by-element

        // Create an uninitialized array of `MaybeUninit`. The `assume_init` is
        // safe because the type we are claiming to have initialized here is a
        // bunch of `MaybeUninit`s, which do not require initialization.
        let mut bins: [MaybeUninit<Mutex<StringCache>>; NUM_BINS] = unsafe {
            MaybeUninit::uninit().assume_init()
        };

        // Dropping a `MaybeUninit` does nothing. Thus using raw pointer
        // assignment instead of `ptr::write` does not cause the old
        // uninitialized value to be dropped. Also if there is a panic during
        // this loop, we have a memory leak, but there is no memory safety
        // issue.
        for bin in &mut bins[..] {
            *bin = MaybeUninit::new(Mutex::new(StringCache::default()));
        }

        // Everything is initialized. Transmute the array to the
        // initialized type.
        unsafe { mem::transmute::<_, Bins>(bins) }
    };
}

// Use the top bits of the hash to choose a bin
#[inline]
fn whichbin(hash: u64) -> usize {
    ((hash >> TOP_SHIFT as u64) % NUM_BINS as u64) as usize
}
