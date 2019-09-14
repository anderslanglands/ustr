use std::cmp::Eq;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};

lazy_static::lazy_static! {
    static ref STRING_CACHE: spin::RwLock<StringCache> = spin::RwLock::new(StringCache::new());
}

pub struct UString {
    i: usize,
}

impl UString {
    pub fn new(string: &str) -> UString {
        {
            let sc = STRING_CACHE.read();
            if let Some(i) = sc.set.get(string) {
                return UString { i: *i };
            }
        }

        let mut hasher = DefaultHasher::new();
        string.hash(&mut hasher);
        let hash = hasher.finish();
        let mut s = String::from(string);
        s.shrink_to_fit();
        let len = s.len();
        let mut s = s.into_bytes();
        s.push(0);
        let ptr = s.as_ptr();
        std::mem::forget(s);

        let mut sc = STRING_CACHE.write();
        sc.vec.push(StringCacheEntry { hash, ptr, len });
        let i = sc.vec.len() - 1;
        sc.set.insert(string.into(), i);
        UString { i }
    }

    pub fn as_str(&self) -> &str {
        let sc = STRING_CACHE.read();
        unsafe {
            let sce = sc.vec.get_unchecked(self.i);
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(sce.ptr, sce.len))
        }
    }

    pub fn as_c_str(&self) -> *const std::os::raw::c_char {
        let sc = STRING_CACHE.read();
        unsafe {
            let sce = sc.vec.get_unchecked(self.i);
            sce.ptr as *const std::os::raw::c_char
        }
    }

    pub fn len(&self) -> usize {
        let sc = STRING_CACHE.read();
        unsafe {
            let sce = sc.vec.get_unchecked(self.i);
            sce.len
        }
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
        let sc = STRING_CACHE.read();
        unsafe {
            let sce = sc.vec.get_unchecked(self.i);
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(sce.ptr, sce.len))
        }
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
    set: HashMap<String, usize>,
    vec: Vec<StringCacheEntry>,
}

impl StringCache {
    pub fn new() -> StringCache {
        StringCache {
            set: HashMap::new(),
            vec: Vec::new(),
        }
    }
}

struct StringCacheEntry {
    hash: u64,
    ptr: *const u8,
    len: usize,
}

unsafe impl Send for StringCacheEntry {}
unsafe impl Sync for StringCacheEntry {}

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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        use super::UString;

        let u_hello = UString::new("hello");
        assert_eq!(u_hello, "hello");
        let u_world = UString::new("world");
        assert_eq!(u_world, String::from("world"));

        println!("{}, {}!", u_hello, u_world);
    }
}
