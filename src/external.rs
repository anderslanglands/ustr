//! Sharing one string cache between copies of this crate in a process.
//!
//! Each copy of `ustr` linked into a process has its own global cache, and
//! `Ustr`s compare by address, so equal strings interned by two copies compare
//! unequal. That happens when a program loads a dynamic library built
//! separately with its own copy of the crate, such as a plugin.
//!
//! [`set_external_interner`] makes [`Ustr::from`](crate::Ustr::from) and
//! [`existing_ustr`](crate::existing_ustr) in one copy use the cache of
//! another, whose [`local_interner`] the program passes across. Strings then
//! compare equal across the libraries and can cross between them unchanged.
//! A copy without an external interner uses its own cache, as before.

use std::{
    fmt, ptr,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::Ustr;

/// A function that interns the `len` UTF-8 bytes at `ptr` in a string cache
/// and returns the cached characters, as [`Ustr::as_char_ptr`] gives them.
///
/// With `insert` false it only looks the string up, and returns null when the
/// cache does not hold it.
pub type ExternalInterner =
    unsafe extern "C" fn(ptr: *const u8, len: usize, insert: bool) -> *const u8;

static EXTERNAL_INTERNER: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

// Returns the external interner, if one is installed
#[inline]
pub(crate) fn external_interner() -> Option<ExternalInterner> {
    let interner = EXTERNAL_INTERNER.load(Ordering::Acquire);
    if interner.is_null() {
        return None;
    }
    // SAFETY: `set_external_interner` is the only writer and stores an
    // `ExternalInterner`.
    Some(unsafe { std::mem::transmute::<*mut (), ExternalInterner>(interner) })
}

/// Returns the interner of this copy's own cache, for another copy of the
/// crate to pass to [`set_external_interner`].
#[must_use]
pub fn local_interner() -> ExternalInterner {
    intern_locally
}

unsafe extern "C" fn intern_locally(
    ptr: *const u8,
    len: usize,
    insert: bool,
) -> *const u8 {
    // SAFETY: the caller passes `len` bytes of UTF-8 that stay readable for
    // the call.
    let string = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len))
    };
    if insert {
        Ustr::from_local(string).as_char_ptr().cast()
    } else {
        Ustr::from_existing_local(string)
            .map_or(ptr::null(), |ustr| ustr.as_char_ptr().cast())
    }
}

/// The error [`set_external_interner`] returns for an interner whose strings
/// this copy of the crate reads differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncompatibleInterner;

impl fmt::Display for IncompatibleInterner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "the external interner's strings do not read back in this copy of ustr",
        )
    }
}

impl std::error::Error for IncompatibleInterner {}

/// Makes this copy of the crate intern into the cache behind `interner`.
///
/// Afterwards [`Ustr::from`](crate::Ustr::from) and
/// [`existing_ustr`](crate::existing_ustr) go to that cache, so the strings
/// compare equal to the ones the other copy interns. The cache statistics and
/// [`cache`](crate::cache) still describe this copy's own cache.
///
/// The call interns a probe string and checks that its text and length read
/// back through this copy's entry layout, so a copy with another layout is
/// rejected. Hashes are not compared: a build may seed its hasher differently,
/// and every copy reads the hash the owning cache stored with the entry.
///
/// # Errors
///
/// Returns [`IncompatibleInterner`] if the probe does not read back.
///
/// # Safety
///
/// `interner` must come from [`local_interner`] of another copy of this crate
/// with the same entry layout, which stays loaded for the life of the process.
/// Strings this copy interned before the call stay in its own cache.
pub unsafe fn set_external_interner(
    interner: ExternalInterner,
) -> Result<(), IncompatibleInterner> {
    const PROBE: &str = "ustr-external-interner-probe";
    // SAFETY: `PROBE` is UTF-8 that outlives the call.
    let chars = unsafe { interner(PROBE.as_ptr(), PROBE.len(), true) };
    let char_ptr =
        NonNull::new(chars.cast_mut()).ok_or(IncompatibleInterner)?;
    let probe = Ustr { char_ptr };
    if probe.len() != PROBE.len() || probe.as_str() != PROBE {
        return Err(IncompatibleInterner);
    }
    EXTERNAL_INTERNER.store(interner as *mut (), Ordering::Release);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{existing_ustr, ustr};

    // Returns another string's characters, as a cache read differently would
    unsafe extern "C" fn mismatched(
        _ptr: *const u8,
        _len: usize,
        _insert: bool,
    ) -> *const u8 {
        Ustr::from_local("not-the-probe").as_char_ptr().cast()
    }

    #[test]
    fn rejects_an_interner_that_reads_back_differently() {
        // SAFETY: the interner returns strings of this copy's cache.
        let result = unsafe { set_external_interner(mismatched) };

        assert_eq!(result, Err(IncompatibleInterner));
        assert_eq!(
            result.unwrap_err().to_string(),
            "the external interner's strings do not read back in this copy of ustr"
        );
    }

    #[test]
    fn local_interner_inserts_and_looks_up() {
        let value = "external-interner-local";
        let interner = local_interner();

        // SAFETY: `value` is UTF-8 for the calls.
        let missing = unsafe { interner(value.as_ptr(), value.len(), false) };
        // SAFETY: as above.
        let inserted = unsafe { interner(value.as_ptr(), value.len(), true) };
        // SAFETY: as above.
        let found = unsafe { interner(value.as_ptr(), value.len(), false) };

        assert!(missing.is_null());
        assert_eq!(inserted, ustr(value).as_char_ptr().cast());
        assert_eq!(found, inserted);
    }

    #[test]
    fn an_installed_interner_serves_ustr_and_existing_ustr() {
        // This copy's own interner stands in for another copy's, so the other
        // tests sharing the process see the same cache either way
        // SAFETY: the interner comes from this copy, loaded for the process.
        unsafe { set_external_interner(local_interner()) }.unwrap();
        let value = "external-interner-installed";

        let missing = existing_ustr(value);
        let interned = ustr(value);

        assert_eq!(missing, None);
        assert_eq!(existing_ustr(value), Some(interned));
        assert_eq!(interned, Ustr::from_local(value));
        assert!(external_interner().is_some());
    }
}
