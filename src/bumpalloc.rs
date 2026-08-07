use std::alloc::{GlobalAlloc, Layout, System};

/// Simple, fast bump allocator specialized for the string cache.
/// Bumps a pointer downward and aborts on exhaustion; callers are expected
/// to rotate in a new allocator before that happens.
///
/// See <https://fitzgeraldnick.com/2019/11/01/always-bump-downwards.html>
pub(crate) struct LeakyBumpAlloc {
    layout: Layout,
    start: *mut u8,
    end: *mut u8,
    ptr: *mut u8,
}

impl LeakyBumpAlloc {
    pub fn new(capacity: usize, alignment: usize) -> LeakyBumpAlloc {
        let layout = Layout::from_size_align(capacity, alignment)
            .expect("invalid layout");
        // SAFETY: `layout` is valid (non-zero size, power-of-two alignment)
        // since `from_size_align` succeeded. We check for null below.
        let start = unsafe { System.alloc(layout) };
        if start.is_null() {
            // Abort rather than panic to avoid poisoning the cache mutex.
            std::process::abort();
        }
        // SAFETY: `start` is non-null and points to an allocation of
        // `layout.size()` bytes, so `start + layout.size()` is one past
        // the end of that allocation, which is a valid pointer value.
        let end = unsafe { start.add(layout.size()) };
        LeakyBumpAlloc {
            layout,
            start,
            end,
            ptr: end,
        }
    }

    /// # Safety
    ///
    /// This deallocates the backing memory. Caller must ensure no references
    /// to memory handed out by `allocate` are still in use. Intended only for
    /// benchmark cleanup.
    #[doc(hidden)]
    pub unsafe fn clear(&mut self) {
        // SAFETY: `self.start` was allocated via `System.alloc` with
        // `self.layout`, and the caller guarantees no outstanding references.
        unsafe {
            System.dealloc(self.start, self.layout);
        }
    }

    /// # Safety
    ///
    /// The returned pointer is valid for writes of `num_bytes` bytes and
    /// remains valid for the lifetime of the allocator (i.e., until `clear`
    /// is called). Caller must ensure proper initialization before reading.
    pub unsafe fn allocate(&mut self, num_bytes: usize) -> *mut u8 {
        let ptr = self.ptr as usize;
        let new_ptr = ptr
            .checked_sub(num_bytes)
            .expect("pointer subtraction overflowed");
        // Round down to the allocator's alignment.
        let new_ptr = new_ptr & !(self.layout.align() - 1);
        let start = self.start as usize;
        if new_ptr < start {
            eprintln!(
                "Allocator asked to bump to {} bytes with a capacity of {}",
                self.end as usize - new_ptr,
                self.capacity()
            );
            // Abort instead of panic to avoid poisoning the cache mutex.
            std::process::abort();
        }

        self.ptr = new_ptr as *mut u8;
        self.ptr
    }

    /// Bytes allocated from this bump region.
    pub fn allocated(&self) -> usize {
        self.end as usize - self.ptr as usize
    }

    /// Total capacity of this bump region.
    pub fn capacity(&self) -> usize {
        self.layout.size()
    }
}
