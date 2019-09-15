//! Fast, FFI-friendly string interning. A UString is a lightweight handle
//! representing an entry in a global string cache, allowing for: 
//! * Extremely fast string comparisons - it's just a pointer comparison.
//! * Amortized storage -  only one copy of the string is held in memory, and 
//! getting access to it is just a pointer indirection.
//! * Fast hashing - the precomputed hash is stored with the string
//! * Fast FFI - the string is stored with a terminating null byte so can be 
//! passed to C directly without doing the CString dance.
//! 
//! The downside is no strings are ever freed, so if you're creating lots and 
//! lots of strings, you might run out of memory. On the other hand, War and Peace
//! is only 3MB, so it's probably fine.
//! 
//! This crate is directly inspired by [OpenImageIO's ustring](https://github.com/OpenImageIO/oiio/blob/master/src/include/OpenImageIO/ustring.h)
//! but it is NOT binary-compatible (yet). The underlying hash map implementation
//! is directy ported from OIIO (but without the binning).
//! 
//! ```
//! use ustring::{UString, u};
//! let h1 = u!("hello");
//! let h2 = u!("hello");
//! assert_eq!(h1, h2); //< just a pointer comparison
//! ```
//! 
//! # NOTICE
//! This crate is pre-alpha. It has been tested (barely) on x86-64. Whatever
//! your architecture, there's probably undefined behaviour lurking in here, so
//! be warned. It also requires nightly.
//! 
//! ## Why?
//! It is common in certain types of applications to use strings as identifiers,
//! but not really do any processing with them. 
//! To paraphrase from OIIO's ustring documentation - 
//! Compared to standard strings, ustrings have several advantages:
//!
//!   - Each individual ustring is very small -- in fact, we guarantee that
//!     a ustring is the same size and memory layout as an ordinary *u8.
//!   - Storage is frugal, since there is only one allocated copy of each
//!     unique character sequence, throughout the lifetime of the program.
//!   - Assignment from one ustring to another is just copy of the pointer;
//!     no allocation, no character copying, no reference counting.
//!   - Equality testing (do the strings contain the same characters) is
//!     a single operation, the comparison of the pointer.
//!   - Memory allocation only occurs when a new ustring is constructed from
//!     raw characters the FIRST time -- subsequent constructions of the
//!     same string just finds it in the canonial string set, but doesn't
//!     need to allocate new storage.  Destruction of a ustring is trivial,
//!     there is no de-allocation because the canonical version stays in
//!     the set.  Also, therefore, no user code mistake can lead to
//!     memory leaks.
//!   - Creating a new UString is faster than String::from()
//!
//! But there are some problems, too.  Canonical strings are never freed
//! from the table.  So in some sense all the strings "leak", but they
//! only leak one copy for each unique string that the program ever comes
//! across.
//!
//! On the whole, ustrings are a really great string representation
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
//! ustrings are not so hot
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

#![feature(allocator_api)]
use spin::Mutex;
use std::cmp::Eq;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::alloc::{System, Alloc};

lazy_static::lazy_static! {
    static ref STRING_CACHE: Mutex<StringCache> = Mutex::new(StringCache::with_capacity(INITIAL_CAPACITY));
}

/// A handle representing a string in the global string cache.
/// 
/// To use, create one using `UString::from` or the `u!` macro. You can freely
/// copy, destroy or send UStrings to other threads: the underlying string is
/// always valid in memory (and is never destroyed).
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct UString {
    char_ptr: *const u8,
}

impl UString {
    /// Create a new UString from the given &str.
    /// 
    /// You can also use the `u!` macro as a shorthand
    /// ```
    /// use ustring::{UString, u};
    /// 
    /// let u1 = UString::from("constant-time comparisons rule");
    /// let u2 = u!("constant-time comparisons rule");
    /// assert_eq!(u1, u2);
    /// ```
    pub fn from(string: &str) -> UString {
        let hash = fasthash::city::hash64(string.as_bytes());
        let mut sc = STRING_CACHE.lock();
        UString {
            char_ptr: sc.insert(string, hash),
        }
    }

    /// Get the cached string as a &str
    pub fn as_str(&self) -> &str {
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
    /// straight along with no checking. If your C function can't handle them 
    /// then there's no telling what will happen.
    pub unsafe fn as_c_str(&self) -> *const std::os::raw::c_char {
        self.char_ptr as *const std::os::raw::c_char
    }

    /// Get the length (in bytes) of this string.
    pub fn len(&self) -> usize {
        unsafe {
            let len_ptr = (self.char_ptr as *const usize).offset(-1isize);
            std::ptr::read(len_ptr)
        }
    }

    /// Get an owned String copy of this string.
    pub fn to_owned(&self) -> String {
        self.as_str().to_owned()
    }
}

impl PartialEq<&str> for UString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for UString {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl AsRef<str> for UString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for UString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for UString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UString({})", self.as_str())
    }
}

struct StringCache {
    alloc: LeakyBumpAlloc,
    vec: Vec<*mut StringCacheEntry>,
    num_entries: usize,
    capacity: usize,
    mask: usize,
    total_allocated: usize,
}

const INITIAL_CAPACITY: usize = 1 << 20;

impl StringCache {
    pub fn with_capacity(capacity: usize) -> StringCache {
        StringCache {
            alloc: LeakyBumpAlloc::new(capacity),
            vec: vec![std::ptr::null_mut(); capacity],
            num_entries: 0,
            capacity,
            mask: capacity - 1,
            total_allocated: capacity,
        }
    }

    // Insert the given string with its given hash into the cache
    fn insert(&mut self, string: &str, hash: u64) -> *const u8 {
        let mut pos = self.mask & hash as usize;
        let mut dist = 0;
        loop {
            let entry = unsafe { self.vec.get_unchecked(pos) };
            if entry.is_null() {
                // found empty slot to insert
                break;
            }

            unsafe {
                let entry_chars = entry.offset(1isize) as *const u8;
                if (**entry).hash == hash
                    && (**entry).len == string.len()
                    && std::str::from_utf8_unchecked(
                        std::slice::from_raw_parts(entry_chars, (**entry).len)
                        ) == string
                {
                    // found matching string in the cache already, return it
                    return entry_chars;
                }
            }

            // keep looking
            dist += 1;
            pos = (pos + dist) & self.mask;
        }

        // insert the new string
        unsafe {
            let mut entry_ptr = self.vec.get_unchecked_mut(pos);
            
            // add one to length for null byte
            let byte_len = string.len() + 1;
            let alloc_size = std::mem::size_of::<StringCacheEntry>() + byte_len;

            // if our new allocation would spill over the allocator, make a new
            // allocator and let the old one leak
            let capacity = self.alloc.capacity();
            let allocated = self.alloc.allocated();
            if alloc_size + allocated > capacity {
                self.alloc = LeakyBumpAlloc::new(capacity * 2);
                self.total_allocated += capacity * 2;
            }

            *entry_ptr = self.alloc.allocate(alloc_size, std::mem::align_of::<StringCacheEntry>()) as *mut StringCacheEntry;
            // write the header
            let write_ptr = (*entry_ptr) as *mut u64;
            std::ptr::write(write_ptr, hash);
            let write_ptr = write_ptr.offset(1isize) as *mut usize;
            std::ptr::write(write_ptr, string.len());
            // write the characters
            let char_ptr = write_ptr.offset(1isize) as *mut u8;
            std::ptr::copy(string.as_bytes().as_ptr(), char_ptr, string.len());
            // write the trailing null
            let write_ptr = char_ptr.offset(string.len() as isize);
            std::ptr::write(write_ptr, 0u8);

            self.num_entries += 1;
            if self.num_entries * 2 > self.mask {
                // TODO:
                // grow storage to maintain 0.5 load factor
                panic!("MUST GROW");
            }

            char_ptr
        }
    }

    fn clear(&mut self) {
        unsafe {
            libc::memset(
                self.vec.as_mut_ptr() as *mut std::os::raw::c_void,
                0,
                self.num_entries,
            );
        }
    }

    pub(crate) fn total_allocated(&self) -> usize {
        self.total_allocated + self.alloc.allocated()
    }

    pub(crate) fn num_entries(&self) -> usize {
        self.num_entries
    }
}

// Clears the hash map. Used for benchmarking purposes. Do not call this.
#[doc(hidden)]
pub fn _clear_cache() {
    STRING_CACHE.lock().clear();
}

/// Returns the total amount of memory allocated and in use by the cache in bytes
pub fn total_allocated() -> usize {
    STRING_CACHE.lock().total_allocated()
}

/// Returns the number of unique strings in the cache
pub fn num_entries() -> usize {
    STRING_CACHE.lock().num_entries()
}

#[repr(C)]
#[derive(Clone)]
struct StringCacheEntry {
    hash: u64,
    len: usize,
}

unsafe impl Send for StringCacheEntry {}
unsafe impl Sync for StringCacheEntry {}

unsafe impl Send for StringCache {}
unsafe impl Sync for StringCache {}

unsafe impl Send for UString {}
unsafe impl Sync for UString {}

/// Shorthand macro for creating a UString.
/// 
/// ```
/// use ustring::{u, UString};
/// let u_hello = u!("Hello");
/// let u_world = u!("world");
/// println!("{}, {}!", u_hello, u_world);
/// // > Hello, world!
/// ```
#[macro_export]
macro_rules! u {
    ($s:expr) => {
        UString::from($s);
    };
}

struct LeakyBumpAlloc {
    data: *mut u8,
    allocated: usize,
    capacity: usize,
}

fn round_up_to(n: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (n + align - 1) & !(align - 1)
}

impl LeakyBumpAlloc {
    pub fn new(capacity: usize) -> LeakyBumpAlloc {
        let data = System.alloc_array::<u8>(capacity).unwrap().as_ptr();
        debug_assert!(data.align_offset(8) == 0);
        LeakyBumpAlloc {
            data,
            allocated: 0,
            capacity,
        }
    }

    pub unsafe fn allocate(&mut self, num_bytes: usize, alignment: usize) -> *mut u8 {
        let aligned_size = round_up_to(num_bytes, alignment);

        if self.allocated + aligned_size > self.capacity {
            panic!("Bumped over capacity");
        }

        let alloc_ptr = self.data;
        self.data = self.data.offset(aligned_size as isize);
        self.allocated += aligned_size;

        alloc_ptr
    }

    pub fn allocated(&self) -> usize {
        self.allocated
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::UString;

        let u_hello = u!("hello");
        assert_eq!(u_hello, "hello");
        let u_world = u!("world");
        assert_eq!(u_world, String::from("world"));

        println!("{}, {}!", u_hello, u_world);
    }

    #[test]
    fn c_str_works() {
        use super::UString;
        use std::ffi::CStr;

        let u_fox = u!("The quick brown fox jumps over the lazy dog.");
        let fox = unsafe { CStr::from_ptr(u_fox.as_c_str()) }
            .to_string_lossy()
            .into_owned();
        println!("{}", fox);

        let u_odys = u!("Τη γλώσσα μου έδωσαν ελληνική");
        let odys = unsafe { CStr::from_ptr(u_odys.as_c_str()) }
            .to_string_lossy()
            .into_owned();
        println!("{}", odys);
    }

    #[test]
    fn blns() {
        use super::UString;
        let path = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("data")
            .join("blns.txt");
        let blns = std::fs::read_to_string(path).unwrap();

        println!("Num strings: {}", blns.split_whitespace().count());
        let mut hs = std::collections::HashSet::new();
        for s in blns.split_whitespace() {
            hs.insert(s);
        }
        println!("Num unique strings: {}", hs.len());

        let mut us = Vec::new();
        let mut ss = Vec::new();

        for s in blns.split_whitespace().cycle().take(100_000) {
            let u = u!(s);
            us.push(u);
            ss.push(s.to_owned());
        }

        for (u, s) in us.iter().zip(ss.iter()) {
            assert_eq!(u, s);
        }

        println!("Total allocated: {}", super::total_allocated());
        println!("Num entries: {}", super::num_entries());
    }
}
