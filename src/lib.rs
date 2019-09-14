use std::cmp::Eq;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};

lazy_static::lazy_static! {
    static ref STRING_CACHE: spin::RwLock<StringCache> = spin::RwLock::new(StringCache::new());
}

pub struct UString {
    sce: *const StringCacheEntry,
}

impl UString {
    pub fn new(string: &str) -> UString {
        {
            let sc = STRING_CACHE.read();
            if let Some(p) = sc.set.get(string) {
                return UString { sce: *p };
            }
        }

        // create the cached hash
        let mut hasher = DefaultHasher::new();
        string.hash(&mut hasher);
        let hash = hasher.finish();

        // shrink the string's storage, convert it to u8, then push a '\0' on
        // the end so we can pass it to C.
        let mut s = String::from(string);
        s.shrink_to_fit();
        let len = s.len();
        let mut s = s.into_bytes();
        s.push(0);
        let ptr = s.as_ptr();
        // leak the string
        let s = s.into_boxed_slice();
        Box::leak(s);

        // write a new entry into the string cache
        let sce = StringCacheEntry { hash, ptr, len };
        let mut sc = STRING_CACHE.write();
        if sc.vec.len() == sc.vec.capacity() - 1 {
            // leak the current storage and allocate some more
            let mut v = Vec::<StringCacheEntry>::with_capacity(sc.vec.capacity() * 2);
            std::mem::swap(&mut sc.vec, &mut v);
            Box::leak(v.into_boxed_slice());
        }
        sc.vec.push(sce);
        let p = sc.vec.last().unwrap() as *const StringCacheEntry;
        sc.set.insert(string.into(), p);
        UString { sce: p }
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
    set: HashMap<String, *const StringCacheEntry>,
    vec: Vec<StringCacheEntry>,
}

impl StringCache {
    pub fn new() -> StringCache {
        StringCache {
            set: HashMap::new(),
            vec: Vec::with_capacity(1024),
        }
    }
}

pub fn _clear_cache() {
    let mut sc = STRING_CACHE.write();
    *sc = StringCache::new();
}

struct StringCacheEntry {
    hash: u64,
    ptr: *const u8,
    len: usize,
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
