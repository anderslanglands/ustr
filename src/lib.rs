use spin::Mutex;
use std::cmp::Eq;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};

lazy_static::lazy_static! {
    static ref STRING_CACHE: Mutex<StringCache> = Mutex::new(StringCache::with_capacity(INITIAL_CAPACITY));
}

pub struct UString {
    sce: *const StringCacheEntry,
}

impl UString {
    pub fn new(string: &str) -> UString {
        let hash = fasthash::city::hash64(string.as_bytes());
        let mut sc = STRING_CACHE.lock();
        UString {
            sce: sc.insert(string, hash),
        }
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(
                (*self.sce).ptr,
                (*self.sce).len,
            ))
        }
    }

    pub fn as_c_str(&self) -> *const std::os::raw::c_char {
        unsafe { (*self.sce).ptr as *const std::os::raw::c_char }
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.sce).len }
    }

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
    vec: Vec<StringCacheEntry>,
    num_entries: usize,
    capacity: usize,
    mask: usize,
}

const INITIAL_CAPACITY: usize = 1 << 20;

impl StringCache {
    pub fn with_capacity(capacity: usize) -> StringCache {
        StringCache {
            vec: vec![StringCacheEntry::default(); capacity],
            num_entries: 0,
            capacity,
            mask: capacity - 1,
        }
    }

    fn insert(&mut self, string: &str, hash: u64) -> *const StringCacheEntry {
        let mut pos = self.mask & hash as usize;
        let mut dist = 0;
        loop {
            let entry = unsafe { self.vec.get_unchecked(pos) };
            if entry.ptr.is_null() {
                // found empty slot to insert
                break;
            }

            if entry.hash == hash
                && entry.len == string.len()
                && unsafe {
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(entry.ptr, entry.len))
                } == string
            {
                // found matching string in the cache already, return it
                return entry as *const StringCacheEntry;
            }

            // keep looking
            dist += 1;
            pos = (pos + dist) & self.mask;
        }

        // insert the new string
        let entry = unsafe { self.vec.get_unchecked_mut(pos) };
        *entry = StringCacheEntry::new(string, hash);

        self.num_entries += 1;
        if self.num_entries * 2 > self.mask {
            // TODO:
            // grow storage to maintain 0.5 load factor
        }

        entry as *const StringCacheEntry
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
}

pub fn _clear_cache() {
    STRING_CACHE.lock().clear();
}

#[repr(C)]
#[derive(Clone)]
struct StringCacheEntry {
    ptr: *const u8,
    len: usize,
    hash: u64,
}

impl StringCacheEntry {
    pub fn new(string: &str, hash: u64) -> StringCacheEntry {
        let mut s = String::from(string);
        s.shrink_to_fit();
        let len = s.len();
        let mut s = s.into_bytes();
        s.push(0);
        let ptr = s.as_ptr();
        // leak the string
        let s = s.into_boxed_slice();
        Box::leak(s);

        StringCacheEntry { ptr, len, hash }
    }
}

impl Default for StringCacheEntry {
    fn default() -> StringCacheEntry {
        StringCacheEntry {
            ptr: std::ptr::null(),
            len: 0,
            hash: 0,
        }
    }
}

unsafe impl Send for StringCacheEntry {}
unsafe impl Sync for StringCacheEntry {}

unsafe impl Send for StringCache {}
unsafe impl Sync for StringCache {}

impl PartialEq for StringCacheEntry {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}
impl Eq for StringCacheEntry {}

impl Hash for StringCacheEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

#[macro_export]
macro_rules! u {
    ($s:expr) => {
        UString::new($s);
    };
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

        let mut us = Vec::new();
        let mut ss = Vec::new();

        for s in blns.split_whitespace() {
            let u = u!(s);
            us.push(u);
            ss.push(s.to_owned());
        }

        for (u, s) in us.iter().zip(ss.iter()) {
            assert_eq!(u, s);
        }
    }
}
