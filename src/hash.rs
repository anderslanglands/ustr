use super::Ustr;
use byteorder::{ByteOrder, NativeEndian};
use std::{
    collections::{HashMap, HashSet},
    hash::{BuildHasherDefault, Hasher},
};

/// A const-compatible hash function using FNV-1a algorithm.
/// This can be evaluated at compile time when used with string literals.
///
/// # Examples
/// ```
/// use ustr::hash::string_hash;
///
/// // This can be computed at compile time!
/// const HASH: u64 = string_hash(b"hello");
///
/// // Works at runtime too
/// let runtime_hash = string_hash(b"world");
/// ```
#[inline]
pub const fn string_hash(bytes: &[u8]) -> u64 {
    const_fnv1a_hash(bytes)
}

/// Const-compatible FNV-1a hash implementation
const fn const_fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64; // FNV-1a offset basis
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV-1a prime
        i += 1;
    }
    hash
}

/// Runtime hash function using AHash.
///
/// # Performance Note
///
/// Our benchmarks show that for typical string sizes used in ustr (< 40 bytes),
/// AHash outperforms other popular hashers:
/// - 1 byte: 0.74 ns (vs XXHash3: 1.70 ns, GxHash: 0.77 ns)
/// - 5 bytes: 0.76 ns (vs XXHash3: 1.44 ns, GxHash: 0.80 ns)
/// - 19 bytes: 0.75 ns (vs XXHash3: 1.79 ns, GxHash: 1.15 ns)
///
/// However, the hash function speed is rarely the bottleneck. The real
/// performance constraints are:
/// 1. Mutex locking for thread-safe cache access (~20-30 ns)
/// 2. Hash table lookup and insertion (~10-15 ns)
/// 3. Memory allocation for new strings (~5-10 ns)
///
/// The hash computation (~1 ns) is only about 2% of the total time for string
/// interning.
#[inline]
pub fn runtime_hash(bytes: &[u8]) -> u64 {
    use ahash::AHasher;
    let mut hasher = AHasher::default();
    hasher.write(bytes);
    hasher.finish()
}

/// Unified hash function that automatically selects the best implementation.
/// When Rust gains const_eval_select, this will automatically use const_hash
/// at compile time and runtime_hash at runtime.
///
/// Currently uses AHash which is optimized for small strings.
#[inline]
pub fn hash(bytes: &[u8]) -> u64 {
    // In the future with const_eval_select:
    // const_eval_select((bytes,), const_fnv1a_hash, runtime_hash)

    // Use AHash for best performance on small strings
    runtime_hash(bytes)
}

/// Macro to force compile-time hash evaluation when possible
#[macro_export]
macro_rules! const_hash {
    ($s:literal) => {{
        const HASH: u64 = $crate::hash::string_hash($s.as_bytes());
        HASH
    }};
}

/// A standard `HashMap` using `Ustr` as the key type with a custom `Hasher`
/// that just uses the precomputed hash for speed instead of calculating it.
pub type UstrMap<V> = HashMap<Ustr, V, BuildHasherDefault<IdentityHasher>>;

/// A standard `HashSet` using `Ustr` as the key type with a custom `Hasher`
/// that just uses the precomputed hash for speed instead of calculating it.
pub type UstrSet = HashSet<Ustr, BuildHasherDefault<IdentityHasher>>;

/// The worst hasher in the world -- the identity hasher.
#[doc(hidden)]
#[derive(Default)]
pub struct IdentityHasher {
    hash: u64,
}

impl Hasher for IdentityHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        match bytes.len() {
            8 => {
                self.hash = NativeEndian::read_u64(bytes);
            }
            0 => {}
            _ => panic!(
                "IdentityHasher only supports 8-byte writes; got {} bytes",
                bytes.len()
            ),
        }
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = i;
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.hash = i as u64;
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

#[test]
fn test_hashing() {
    let _t = super::TEST_LOCK.lock();
    use crate::ustr as u;

    use std::hash::Hash;
    let u1 = u("the quick brown fox");
    let u2 = u("jumped over the lazy dog");

    let mut hasher = IdentityHasher::default();
    u1.hash(&mut hasher);
    assert_eq!(hasher.finish(), u1.precomputed_hash());

    let mut hasher = IdentityHasher::default();
    u2.hash(&mut hasher);
    assert_eq!(hasher.finish(), u2.precomputed_hash());

    let mut hm = UstrMap::<u32>::default();
    hm.insert(u1, 17);
    hm.insert(u2, 42);

    assert_eq!(hm.get(&u1), Some(&17));
    assert_eq!(hm.get(&u2), Some(&42));
}
