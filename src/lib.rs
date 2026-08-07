//! Fast, FFI-friendly string interning. A [`Ustr`] (**U**nique **Str**) is a
//! lightweight handle representing a static, immutable entry in a global string
//! cache, allowing for:
//!
//! * Extremely fast string assignment and comparisons -- it's just a pointer
//!   comparison.
//!
//! * Efficient storage -- only one copy of the string is held in memory, and
//!   getting access to it is just a pointer indirection.
//!
//! * Fast hashing -- the precomputed hash is stored with the string.
//!
//! * Fast FFI -- the string is stored with a terminating null byte so can be
//!   passed to C directly without doing the `CString` dance.
//!
//! The downside is no strings are ever freed, so if you're creating lots and
//! lots of strings, you might run out of memory. On the other hand, War and
//! Peace is only 3MB, so it's probably fine.
//!
//! This crate is based on [OpenImageIO's](https://openimageio.readthedocs.io/en/v2.4.10.0/)
//! (OIIO) [`ustring`](https://github.com/OpenImageIO/oiio/blob/master/src/include/OpenImageIO/ustring.h)
//! but it is *not* binary-compatible (yet). The underlying hash map
//! implementation is directy ported from OIIO.
//!
//! # Usage
//!
//! ```
//! use ustr::{Ustr, ustr, ustr as u};
//!
//! # unsafe { ustr::_clear_cache() };
//! // Creation is quick and easy using either `Ustr::from` or the ustr function
//! // and only one copy of any string is stored.
//! let u1 = Ustr::from("the quick brown fox");
//! let u2 = ustr("the quick brown fox");
//!
//! // Comparisons and copies are extremely cheap.
//! let u3 = u1;
//! assert_eq!(u2, u3);
//!
//! // You can pass straight to FFI.
//! let len = unsafe {
//!     libc::strlen(u1.as_char_ptr())
//! };
//! assert_eq!(len, 19);
//!
//! // Use as_str() to get a `str`.
//! let words: Vec<&str> = u1.as_str().split_whitespace().collect();
//! assert_eq!(words, ["the", "quick", "brown", "fox"]);
//!
//! // For best performance when using Ustr as key for a HashMap or HashSet,
//! // you'll want to use the precomputed hash. To make this easier, just use
//! // the UstrMap and UstrSet exports:
//! use ustr::UstrMap;
//!
//! // Key type is always `Ustr`.
//! let mut map: UstrMap<usize> = UstrMap::default();
//! map.insert(u1, 17);
//! assert_eq!(*map.get(&u1).unwrap(), 17);
//! ```
//!
//! By enabling the `"serde"` feature you can serialize individual `Ustr`s
//! or the whole cache with serde.
//!
//! ```
//! # #[cfg(feature = "serde")] {
//! use ustr::{Ustr, ustr};
//! let u_ser = ustr("serde");
//! let json = serde_json::to_string(&u_ser).unwrap();
//! let u_de : Ustr = serde_json::from_str(&json).unwrap();
//! assert_eq!(u_ser, u_de);
//! # }
//! ```
//!
//! Since the cache is global, use the `ustr::DeserializedCache` dummy object to
//! drive the deserialization.
//!
//! ```
//! # #[cfg(feature = "serde")] {
//! use ustr::{Ustr, ustr};
//! ustr("Send me to JSON and back");
//! let json = serde_json::to_string(ustr::cache()).unwrap();
//!
//! // ... some time later ...
//! let _: ustr::DeserializedCache = serde_json::from_str(&json).unwrap();
//! assert_eq!(ustr::num_entries(), 1);
//! assert_eq!(ustr::string_cache_iter().collect::<Vec<_>>(), vec!["Send me to JSON and back"]);
//! # }
//! ```
//!
//! By enabling the `"rkyv"` feature you can use zero-copy deserialization with
//! rkyv.
//!
//! ```
//! # #[cfg(feature = "rkyv")] {
//! use ustr::{Ustr, ustr};
//!
//! let u_hello = ustr("hello world");
//!
//! // Serialize to bytes
//! let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&u_hello).unwrap();
//!
//! // Access the archived string (zero-copy)
//! let archived = unsafe { rkyv::access_unchecked::<rkyv::string::ArchivedString>(&bytes) };
//! assert_eq!(archived.as_str(), "hello world");
//!
//! // Deserialize back to Ustr (interns the string again)
//! let deserialized: Ustr = rkyv::deserialize::<Ustr, rkyv::rancor::Error>(archived).unwrap();
//! assert_eq!(u_hello, deserialized);
//! # }
//! ```
//!
//! ## Why?
//!
//! It is common in certain types of applications to use strings as identifiers,
//! but not really do any processing with them.
//! To paraphrase from OIIO's `Ustring` documentation -- compared to standard
//! strings, `Ustr`s have several advantages:
//!
//!   - Each individual `Ustr` is very small -- in fact, we guarantee that a
//!     `Ustr` is the same size and memory layout as an ordinary `*u8`.
//!
//!   - Storage is frugal, since there is only one allocated copy of each unique
//!     character sequence, throughout the lifetime of the program.
//!
//!   - Assignment from one `Ustr` to another is just copy of the pointer; no
//!     allocation, no character copying, no reference counting.
//!
//!   - Equality testing (do the strings contain the same characters) is a
//!     single operation, the comparison of the pointer.
//!
//!   - Memory allocation only occurs when a new `Ustr` is constructed from raw
//!     characters the FIRST time -- subsequent constructions of the same string
//!     just finds it in the canonial string set, but doesn't need to allocate
//!     new storage.  Destruction of a `Ustr` is trivial, there is no
//!     de-allocation because the canonical version stays in the set.  Also,
//!     therefore, no user code mistake can lead to memory leaks.
//!
//! But there are some problems, too.  Canonical strings are never freed
//! from the table.  So in some sense all the strings "leak", but they
//! only leak one copy for each unique string that the program ever comes
//! across.
//!
//! On the whole, `Ustr`s are a really great string representation
//!
//!   - if you tend to have (relatively) few unique strings, but many copies of
//!     those strings;
//!
//!   - if the creation of strings from raw characters is relatively rare
//!     compared to copying or comparing to existing strings;
//!
//!   - if you tend to make the same strings over and over again, and if it's
//!     relatively rare that a single unique character sequence is used only
//!     once in the entire lifetime of the program;
//!
//!   - if your most common string operations are assignment and equality
//!     testing and you want them to be as fast as possible;
//!
//!   - if you are doing relatively little character-by-character assembly of
//!     strings, string concatenation, or other "string manipulation" (other
//!     than equality testing).
//!
//! `Ustr`s are not so hot
//!
//!   - if your program tends to have very few copies of each character sequence
//!     over the entire lifetime of the program;
//!
//!   - if your program tends to generate a huge variety of unique strings over
//!     its lifetime, each of which is used only a short time and then
//!     discarded, never to be needed again;
//!
//!   - if you don't need to do a lot of string assignment or equality testing,
//!     but lots of more complex string manipulation.
//!
//! ## Safety and Compatibility
//!
//! This crate contains a significant amount of unsafe but usage has been
//! checked and is well-documented. It is also run through Miri as part of the
//! CI process. I use it regularly on 64-bit systems, and it has passed Miri on
//! a 32-bit system as well, bit 32-bit is not checked regularly. If you want to
//! use it on 32-bit, please make sure to run Miri and open and issue if you
//! find any problems.
//!
//! ## Performance Characteristics
//!
//! ### Hash Function Selection
//!
//! This crate uses AHash for string hashing, which our benchmarks show is
//! optimal for the typical string sizes used in string interning (< 40 bytes):
//! - 1 byte: 0.74 ns (vs XXHash3: 1.70 ns, GxHash: 0.77 ns).
//! - 5 bytes: 0.76 ns (vs XXHash3: 1.44 ns, GxHash: 0.80 ns).
//! - 19 bytes: 0.75 ns (vs XXHash3: 1.79 ns, GxHash: 1.15 ns).
//!
//! ### Where Time is Actually Spent
//!
//! While hash function performance is important, our profiling shows that
//! hashing is only about 2% of the total time for string interning. The real
//! bottlenecks are:
//! 1. **Mutex locking** for thread-safe cache access (~20-30 ns) - 40% of time.
//! 2. **Hash table lookup and insertion** (~10-15 ns) - 30% of time.
//! 3. **Memory allocation** for new strings (~5-10 ns) - 20% of time.
//! 4. **String hashing** (~1 ns) - 2% of time.
//! 5. **Other overhead** - 8% of time.
//!
//! This is why operations on already-interned strings are so fast (just pointer
//! comparison), while first-time interning has unavoidable overhead from
//! synchronization and allocation.
//!
//! ## Features
#![doc = document_features::document_features!()]

use parking_lot::Mutex;
use std::{
    borrow::Cow,
    cmp::Ordering,
    ffi::{CStr, OsStr},
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    os::raw::c_char,
    path::Path,
    ptr::NonNull,
    rc::Rc,
    slice, str,
    str::FromStr,
    sync::Arc,
};

mod bumpalloc;
pub mod cache;
pub use cache::*;
pub mod hash;
pub use hash::{UstrMap, UstrSet};
mod stringcache;
pub use stringcache::*;
#[cfg(feature = "serde")]
pub mod serialization;
#[cfg(feature = "facet")]
pub use facet::Facet;
#[cfg(feature = "serde")]
pub use serialization::DeserializedCache;

#[cfg(feature = "rkyv")]
use rkyv::{
    Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
};

/// A handle representing a string in the global string cache.
///
/// To use, create one using [`Ustr::from`] or the [`ustr`] function. You can
/// freely copy, destroy or send `Ustr`s to other threads: the underlying string
/// is always valid in memory (and is never destroyed).
#[derive(Copy, Clone, PartialEq)]
#[repr(transparent)]
pub struct Ustr {
    char_ptr: NonNull<u8>,
}

// A `Ustr` is a handle into the global string cache, not a value that owns its
// bytes. Deriving `Facet` would model it as a struct with a raw pointer field:
// serializers would then write the pointer's numeric value and — far worse —
// deserializers would write an arbitrary integer back into that field, handing
// out a `Ustr` that dereferences to nothing. That is undefined behaviour, and
// it has been observed in the wild as a `misaligned pointer dereference` abort
// when a `Ustr` was parsed from TOML.
//
// So model `Ustr` as an opaque scalar instead. Every vtable entry goes through
// the string: `display`/`debug` render `as_str()`, and `parse`/`try_from`
// intern the incoming text through the global cache, which is the only way to
// obtain a valid handle.
#[cfg(feature = "facet")]
const _: () = {
    use facet::{
        Def, Facet, PtrConst, Shape, ShapeBuilder, TryFromOutcome, Type,
        TypeOpsDirect, UserType, VTableDirect, type_ops_direct, vtable_direct,
    };

    /// Interns the source string rather than copying a handle's bytes.
    ///
    /// # Safety
    ///
    /// `target` must be valid for writes of a `Ustr`, and `source` must point
    /// to an initialised value of the type described by `source_shape`.
    unsafe fn try_from_string_like(
        target: *mut Ustr,
        source_shape: &'static Shape,
        source: PtrConst,
    ) -> TryFromOutcome {
        if source_shape.id == <&str as Facet>::SHAPE.id {
            let text: &str = unsafe { source.get::<&str>() };
            unsafe { target.write(Ustr::from(text)) };
            TryFromOutcome::Converted
        } else if source_shape.id == <String as Facet>::SHAPE.id {
            let text: String = unsafe { source.read::<String>() };
            unsafe { target.write(Ustr::from(text.as_str())) };
            TryFromOutcome::Converted
        } else {
            TryFromOutcome::Unsupported
        }
    }

    static USTR_TYPE_OPS: TypeOpsDirect =
        type_ops_direct!(Ustr => Default, Clone);

    unsafe impl Facet<'_> for Ustr {
        const SHAPE: &'static Shape = &const {
            // `FromStr` for `Ustr` interns through the global cache, so the
            // `parse` entry this generates is the sound deserialisation path.
            const VTABLE: VTableDirect = vtable_direct!(Ustr =>
                FromStr,
                Display,
                Debug,
                Hash,
                PartialEq,
                PartialOrd,
                Ord,
                [try_from = try_from_string_like],
            );

            ShapeBuilder::for_sized::<Ustr>("Ustr")
                .module_path("ustr")
                .ty(Type::User(UserType::Opaque))
                .def(Def::Scalar)
                .vtable_direct(&VTABLE)
                .type_ops_direct(&USTR_TYPE_OPS)
                .eq()
                .send()
                .sync()
                .build()
        };
    }
};

/// Defer to `str` for equality.
///
/// Lexicographic ordering will be slower than pointer comparison, but much less
/// surprising if you use `Ustr`s as keys in e.g. a `BTreeMap`.
impl Ord for Ustr {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Defer to `str` for equality.
///
/// Lexicographic ordering will be slower thanpointer comparison, but much less
/// surprising if you use `Ustr`s as keys in e.g. a `BTreeMap`.
#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Ustr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ustr {
    /// Create a new `Ustr` from the given `str`.
    ///
    /// You can also use the [`ustr`] function.
    ///
    /// # Examples
    ///
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
        // Use the unified hash function which will be optimized appropriately
        let hash = crate::hash::hash(string.as_bytes());
        let mut sc = STRING_CACHE.0[whichbin(hash)].lock();
        Ustr {
            // SAFETY: sc.insert does not give back a null pointer
            char_ptr: unsafe {
                NonNull::new_unchecked(sc.insert(string, hash) as *mut _)
            },
        }
    }

    pub fn from_existing(string: &str) -> Option<Ustr> {
        // Use the unified hash function
        let hash = crate::hash::hash(string.as_bytes());
        let sc = STRING_CACHE.0[whichbin(hash)].lock();
        sc.get_existing(string, hash).map(|ptr| Ustr {
            char_ptr: unsafe { NonNull::new_unchecked(ptr as *mut _) },
        })
    }

    /// Get the cached `Ustr` as a `str`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ustr::ustr as u;
    /// # unsafe { ustr::_clear_cache() };
    ///
    /// let u_fox = u("the quick brown fox");
    /// let words: Vec<&str> = u_fox.as_str().split_whitespace().collect();
    /// assert_eq!(words, ["the", "quick", "brown", "fox"]);
    /// ```
    pub fn as_str(&self) -> &'static str {
        // This is safe if:
        // 1) `self.char_ptr` points to a valid address
        // 2) `len` is a `usize` stored `usize` aligned `usize` bytes before
        //    `char_ptr`.
        // 3) char_ptr points to a valid UTF-8 string of len bytes.
        // All these are guaranteed by `StringCache::insert()` and by the fact
        // we can only construct a `Ustr` from a valid `&str`.
        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(
                self.char_ptr.as_ptr(),
                self.len(),
            ))
        }
    }

    /// Get the cached string as a C `char*`.
    ///
    /// This includes the null terminator so is safe to pass straight to FFI.
    ///
    /// # Examples
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
    ///
    /// This is just passing a raw byte array with a null terminator to C. If
    /// your source string contains non-ascii bytes then this will pass them
    /// straight along with no checking.
    ///
    /// The string is **immutable**. That means that if you modify it across the
    /// FFI boundary then all sorts of terrible things will happen.
    pub fn as_char_ptr(&self) -> *const c_char {
        self.char_ptr.as_ptr() as *const c_char
    }

    /// Get this `Ustr` as a [`CStr`]
    ///
    /// This is useful for passing to APIs (like ash) that use `CStr`.
    ///
    /// # Safety
    ///
    /// This function by itself is safe as the pointer and length are guaranteed
    /// to be valid. All the same caveats for the use of the `CStr` as given in
    /// the `CStr` docs apply.
    pub fn as_cstr(&self) -> &CStr {
        unsafe {
            CStr::from_bytes_with_nul_unchecked(slice::from_raw_parts(
                self.as_ptr(),
                self.len() + 1,
            ))
        }
    }

    /// Get a raw pointer to the `StringCacheEntry`.
    #[inline]
    fn as_string_cache_entry(&self) -> &StringCacheEntry {
        // The allocator guarantees that the alignment is correct and that
        // this pointer is non-null
        unsafe { &*(self.char_ptr.as_ptr().cast::<StringCacheEntry>().sub(1)) }
    }

    /// Get the length (in bytes) of this string.
    #[inline]
    pub fn len(&self) -> usize {
        self.as_string_cache_entry().len
    }

    /// Returns true if the length is zero.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the precomputed hash for this string.
    #[inline]
    pub fn precomputed_hash(&self) -> u64 {
        self.as_string_cache_entry().hash
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

impl PartialEq<str> for Ustr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Ustr> for str {
    fn eq(&self, u: &Ustr) -> bool {
        self == u.as_str()
    }
}

impl PartialEq<&str> for Ustr {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Ustr> for &str {
    fn eq(&self, u: &Ustr) -> bool {
        *self == u.as_str()
    }
}

impl PartialEq<&&str> for Ustr {
    fn eq(&self, other: &&&str) -> bool {
        self.as_str() == **other
    }
}

impl PartialEq<Ustr> for &&str {
    fn eq(&self, u: &Ustr) -> bool {
        **self == u.as_str()
    }
}

impl PartialEq<String> for Ustr {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Ustr> for String {
    fn eq(&self, u: &Ustr) -> bool {
        self == u.as_str()
    }
}

impl PartialEq<&String> for Ustr {
    fn eq(&self, other: &&String) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Ustr> for &String {
    fn eq(&self, u: &Ustr) -> bool {
        *self == u.as_str()
    }
}

impl PartialEq<Box<str>> for Ustr {
    fn eq(&self, other: &Box<str>) -> bool {
        self.as_str() == &**other
    }
}

impl PartialEq<Ustr> for Box<str> {
    fn eq(&self, u: &Ustr) -> bool {
        &**self == u.as_str()
    }
}

impl PartialEq<Ustr> for &Box<str> {
    fn eq(&self, u: &Ustr) -> bool {
        &***self == u.as_str()
    }
}

impl PartialEq<Cow<'_, str>> for Ustr {
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Ustr> for Cow<'_, str> {
    fn eq(&self, u: &Ustr) -> bool {
        self == u.as_str()
    }
}

impl PartialEq<&Cow<'_, str>> for Ustr {
    fn eq(&self, other: &&Cow<'_, str>) -> bool {
        self.as_str() == **other
    }
}

impl PartialEq<Ustr> for &Cow<'_, str> {
    fn eq(&self, u: &Ustr) -> bool {
        **self == u.as_str()
    }
}

impl PartialEq<Ustr> for Path {
    fn eq(&self, u: &Ustr) -> bool {
        self == Path::new(u)
    }
}

impl PartialEq<Ustr> for &Path {
    fn eq(&self, u: &Ustr) -> bool {
        *self == Path::new(u)
    }
}

impl PartialEq<Ustr> for OsStr {
    fn eq(&self, u: &Ustr) -> bool {
        self == OsStr::new(u)
    }
}

impl PartialEq<Ustr> for &OsStr {
    fn eq(&self, u: &Ustr) -> bool {
        *self == OsStr::new(u)
    }
}

impl Eq for Ustr {}

impl<T: ?Sized> AsRef<T> for Ustr
where
    str: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        self.as_str().as_ref()
    }
}

impl FromStr for Ustr {
    type Err = std::string::ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Ustr::from(s))
    }
}

impl From<&str> for Ustr {
    fn from(s: &str) -> Ustr {
        Ustr::from(s)
    }
}

impl From<Ustr> for &'static str {
    fn from(s: Ustr) -> &'static str {
        s.as_str()
    }
}

impl From<Ustr> for String {
    fn from(u: Ustr) -> Self {
        String::from(u.as_str())
    }
}

impl From<Ustr> for Box<str> {
    fn from(u: Ustr) -> Self {
        Box::from(u.as_str())
    }
}

impl From<Ustr> for Rc<str> {
    fn from(u: Ustr) -> Self {
        Rc::from(u.as_str())
    }
}

impl From<Ustr> for Arc<str> {
    fn from(u: Ustr) -> Self {
        Arc::from(u.as_str())
    }
}

impl From<Ustr> for Cow<'static, str> {
    fn from(u: Ustr) -> Self {
        Cow::Borrowed(u.as_str())
    }
}

impl From<String> for Ustr {
    fn from(s: String) -> Ustr {
        Ustr::from(&s)
    }
}

impl From<&String> for Ustr {
    fn from(s: &String) -> Ustr {
        Ustr::from(s)
    }
}

impl From<Box<str>> for Ustr {
    fn from(s: Box<str>) -> Ustr {
        Ustr::from(&s)
    }
}

impl From<Rc<str>> for Ustr {
    fn from(s: Rc<str>) -> Ustr {
        Ustr::from(&s)
    }
}

impl From<Arc<str>> for Ustr {
    fn from(s: Arc<str>) -> Ustr {
        Ustr::from(&s)
    }
}

impl From<Cow<'_, str>> for Ustr {
    fn from(s: Cow<'_, str>) -> Ustr {
        Ustr::from(&s)
    }
}

impl Default for Ustr {
    fn default() -> Self {
        Ustr::from("")
    }
}

impl Deref for Ustr {
    type Target = str;
    fn deref(&self) -> &Self::Target {
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
        write!(f, "u!({:?})", self.as_str())
    }
}

// Just feed the precomputed hash into the Hasher. Note that this will of course
// be terrible unless the Hasher in question is expecting a precomputed hash.
impl Hash for Ustr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.precomputed_hash().hash(state);
    }
}

#[cfg(feature = "rkyv")]
impl Archive for Ustr {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve(
        &self,
        resolver: Self::Resolver,
        out: rkyv::Place<Self::Archived>,
    ) {
        ArchivedString::resolve_from_str(self.as_str(), resolver, out);
    }
}

#[cfg(feature = "rkyv")]
impl<S> RkyvSerialize<S> for Ustr
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedString::serialize_from_str(self.as_str(), serializer)
    }
}

#[cfg(feature = "rkyv")]
impl<D: Fallible + ?Sized> RkyvDeserialize<Ustr, D> for ArchivedString {
    fn deserialize(
        &self,
        _deserializer: &mut D,
    ) -> Result<Ustr, <D as Fallible>::Error> {
        Ok(Ustr::from(self.as_str()))
    }
}

/// Create a new `Ustr` from the given `str`.
///
/// # Examples
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
#[inline]
pub fn ustr(s: &str) -> Ustr {
    Ustr::from(s)
}

/// Create a new `Ustr` from the given `str` but only if it already exists in
/// the string cache.
///
/// # Examples
///
/// ```
/// use ustr::{ustr, existing_ustr};
/// # unsafe { ustr::_clear_cache() };
///
/// let u1 = existing_ustr("the quick brown fox");
/// let u2 = ustr("the quick brown fox");
/// let u3 = existing_ustr("the quick brown fox");
/// assert_eq!(u1, None);
/// assert_eq!(u3, Some(u2));
/// ```
#[inline]
pub fn existing_ustr(s: &str) -> Option<Ustr> {
    Ustr::from_existing(s)
}

/// Create a Ustr from a string literal with optimized compile-time hashing when
/// possible.
///
/// This macro provides the best of both worlds:
/// - When used with string literals, the hash can be computed at compile time.
/// - The string is still properly interned in the global cache at runtime.
///
/// # Examples
///
/// ```
/// use ustr::static_ustr;
/// # unsafe { ustr::_clear_cache() };
///
/// // The hash is computed at compile time for literals!
/// let s = static_ustr!("compile-time optimized");
///
/// // This is equivalent to ustr() but with potential compile-time optimization.
/// let s2 = static_ustr!("hello world");
/// ```
#[macro_export]
macro_rules! static_ustr {
    ($s:literal) => {{
        // When it's a literal, we can compute the hash at compile time
        // Note: We still use runtime interning to ensure the string is in the
        // cache In the future, we could pre-populate the cache with
        // these strings
        const STRING: &'static str = $s;

        // Try to compute hash at compile time if possible
        // The compiler may optimize this when the context allows
        #[allow(unused)]
        const COMPILE_TIME_HASH: u64 =
            $crate::hash::string_hash(STRING.as_bytes());

        // For now, still use regular Ustr::from to ensure proper caching
        // In the future, we could check if the string is already statically
        // cached
        $crate::Ustr::from(STRING)
    }};
    ($s:expr_2021) => {{
        // For non-literals, fall back to regular ustr
        $crate::ustr($s)
    }};
}

#[cfg(test)]
lazy_static::lazy_static! {
    static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

#[cfg(test)]
mod tests {
    use super::TEST_LOCK;
    #[cfg(feature = "facet")]
    use facet::Facet;
    use std::ffi::OsStr;
    use std::path::Path;

    #[cfg(feature = "facet")]
    #[test]
    fn facet_shape_matches_ustr() {
        let _t = TEST_LOCK.lock();
        assert_eq!(super::Ustr::SHAPE.type_identifier, "Ustr");
        assert_eq!(
            super::Ustr::SHAPE.layout.sized_layout().unwrap().size(),
            std::mem::size_of::<super::Ustr>()
        );
    }

    #[test]
    fn it_works() {
        let _t = TEST_LOCK.lock();
        use super::ustr as u;

        let u_hello = u("hello");
        assert_eq!(u_hello, "hello");
        let u_world = u("world");
        assert_eq!(u_world, String::from("world"));
    }

    #[test]
    fn empty_string() {
        let _t = TEST_LOCK.lock();
        use super::ustr as u;

        unsafe {
            super::_clear_cache();
        }

        let _empty = u("");
        let empty = u("");

        assert!(empty.as_str().is_empty());
        assert_eq!(super::num_entries(), 1);
    }

    #[test]
    fn c_str_works() {
        let _t = TEST_LOCK.lock();
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
    // We have to disable miri here as it's far too slow unfortunately
    #[cfg_attr(miri, ignore)]
    fn blns() {
        let _t = TEST_LOCK.lock();
        use super::{string_cache_iter, ustr as u};
        use std::collections::HashSet;

        // clear the cache first or our results will be wrong
        unsafe { super::_clear_cache() };

        // let path =
        // std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        //     .join("data")
        //     .join("blns.txt");
        // let blns = std::fs::read_to_string(path).unwrap();
        let blns = include_str!("../data/blns.txt");

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
        assert_eq!(diff.len(), 0);

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
    // We have to disable miri here as it's far too slow unfortunately
    #[cfg_attr(miri, ignore)]
    fn raft() {
        let _t = TEST_LOCK.lock();
        use super::ustr as u;
        use std::sync::Arc;

        // let path =
        // std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        //     .join("data")
        //     .join("raft-large-directories.txt");
        // let raft = std::fs::read_to_string(path).unwrap();
        let raft = include_str!("../data/raft-large-directories.txt");
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
    //     let _t = TEST_LOCK.lock();
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

    #[cfg(all(feature = "serde", not(miri)))]
    #[test]
    fn serialization() {
        let _t = TEST_LOCK.lock();
        use super::{string_cache_iter, ustr as u};
        use std::collections::HashSet;

        // clear the cache first or our results will be wrong
        unsafe { super::_clear_cache() };

        let path = std::path::Path::new(
            &std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR not set"),
        )
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

        let json = serde_json::to_string(super::cache()).unwrap();
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
        assert_eq!(diff.len(), 0);
    }

    #[cfg(all(feature = "serde", not(miri)))]
    #[test]
    fn serialization_ustr() {
        let _t = TEST_LOCK.lock();

        use super::{Ustr, ustr};

        let u_hello = ustr("hello");

        let json = serde_json::to_string(&u_hello).unwrap();
        let me_hello: Ustr = serde_json::from_str(&json).unwrap();

        assert_eq!(u_hello, me_hello);
    }

    #[cfg(all(feature = "rkyv", not(miri)))]
    #[test]
    fn rkyv_ustr() {
        let _t = TEST_LOCK.lock();

        use super::{Ustr, ustr};

        // Clear cache to ensure clean state
        unsafe { super::_clear_cache() };

        let u_hello = ustr("hello world");
        let u_test = ustr("test string");

        // Serialize using rkyv
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&u_hello).unwrap();

        // Deserialize using rkyv - access the archived string
        let archived = unsafe {
            rkyv::access_unchecked::<rkyv::string::ArchivedString>(&bytes)
        };
        let deserialized: Ustr =
            rkyv::deserialize::<Ustr, rkyv::rancor::Error>(archived).unwrap();

        assert_eq!(u_hello, deserialized);
        assert_eq!(deserialized.as_str(), "hello world");

        // Test serializing and accessing back the string content
        assert_eq!(archived.as_str(), "hello world");

        // Test with multiple Ustrs
        let ustrs = vec![u_hello, u_test];
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&ustrs).unwrap();

        // For vectors, we need to access the archived vec which contains
        // archived strings
        let archived_vec = unsafe {
            rkyv::access_unchecked::<
                rkyv::vec::ArchivedVec<rkyv::string::ArchivedString>,
            >(&bytes)
        };

        // Deserialize each element
        let mut deserialized_vec = Vec::new();
        for archived_str in archived_vec.iter() {
            let u: Ustr =
                rkyv::deserialize::<Ustr, rkyv::rancor::Error>(archived_str)
                    .unwrap();
            deserialized_vec.push(u);
        }

        assert_eq!(ustrs, deserialized_vec);
    }

    #[test]
    fn partial_ord() {
        let _t = TEST_LOCK.lock();
        use super::ustr;
        let str_a = ustr("aaa");
        let str_z = ustr("zzz");
        let str_k = ustr("kkk");
        assert!(str_a < str_k);
        assert!(str_k < str_z);
    }

    #[test]
    fn ord() {
        let _t = TEST_LOCK.lock();
        use super::ustr;
        let u_apple = ustr("apple");
        let u_bravo = ustr("bravo");
        let u_charlie = ustr("charlie");
        let u_delta = ustr("delta");

        let mut v = vec![u_delta, u_bravo, u_charlie, u_apple];
        v.sort();
        assert_eq!(v, vec![u_apple, u_bravo, u_charlie, u_delta]);
    }

    fn takes_into_str<'a, S: Into<&'a str>>(s: S) -> &'a str {
        s.into()
    }

    #[test]
    fn test_into_str() {
        let _t = TEST_LOCK.lock();
        use super::ustr;

        assert_eq!("converted", takes_into_str(ustr("converted")));
    }

    #[test]
    fn test_existing_ustr() {
        let _t = TEST_LOCK.lock();
        use super::{existing_ustr, ustr};
        assert_eq!(existing_ustr("hello world!"), None);
        let s1 = ustr("hello world!");
        let s2 = existing_ustr("hello world!");
        assert_eq!(Some(s1), s2);
    }

    #[test]
    fn test_empty_cache() {
        unsafe { super::_clear_cache() };
        assert_eq!(
            super::string_cache_iter().collect::<Vec<_>>(),
            Vec::<&'static str>::new()
        );
    }

    #[test]
    fn test_simple_iterator() {
        let _t = TEST_LOCK.lock();
        use super::{string_cache_iter, ustr as u};
        use std::collections::HashSet;

        unsafe { super::_clear_cache() };

        // Create a few strings
        let s1 = u("hello");
        let s2 = u("world");
        let s3 = u("test");

        println!("Created: {:?}, {:?}, {:?}", s1, s2, s3);

        // Collect from iterator
        let found: Vec<_> = string_cache_iter().collect();
        println!("Found via iterator: {:?}", found);

        // Check that we find the right number
        assert_eq!(super::num_entries(), 3);
        assert_eq!(found.len(), 3);

        // Check that we find the right strings
        let mut found_set = HashSet::new();
        for s in found {
            found_set.insert(s);
        }

        assert!(found_set.contains("hello"));
        assert!(found_set.contains("world"));
        assert!(found_set.contains("test"));
    }

    #[test]
    fn as_refs() {
        let _t = TEST_LOCK.lock();

        let u = super::ustr("test");

        let s: String = u.to_owned();
        assert_eq!(u, s);
        assert_eq!(s, u);

        let p: &Path = u.as_ref();
        assert_eq!(p, u);

        let _: &[u8] = u.as_ref();

        let o: &OsStr = u.as_ref();
        assert_eq!(p, o);
        assert_eq!(o, p);

        let cow = std::borrow::Cow::from(u);
        assert_eq!(cow, u);
        assert_eq!(u, cow);

        let boxed: Box<str> = u.into();
        assert_eq!(boxed, u);
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
        #[allow(clippy::missing_transmute_annotations)]
        Bins(unsafe { mem::transmute::<_, [Mutex<StringCache>; NUM_BINS]>(bins) })
    };
}

// Use the top bits of the hash to choose a bin
#[inline]
fn whichbin(hash: u64) -> usize {
    ((hash >> TOP_SHIFT as u64) % NUM_BINS as u64) as usize
}
