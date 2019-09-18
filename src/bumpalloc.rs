use std::alloc::{GlobalAlloc, Layout, System};

// The world's dumbest allocator. Just keep bumping a pointer until we run out
// of memory, in which case we abort. StringCache is responsible for creating
// a new allocator when that's about to happen.
pub(crate) struct LeakyBumpAlloc {
    layout: Layout,
    // pointer to start of free, aligned memory
    data: *mut u8,
    // number of bytes given out
    allocated: usize,
    // total capacity
    capacity: usize,
    // alignment of all allocations
    alignment: usize,
    // pointer to the start of the whole arena
    start: *mut u8,
}

pub fn round_up_to(n: usize, align: usize) -> usize {
    debug_assert!(align.is_power_of_two());
    (n + align - 1) & !(align - 1)
}

impl LeakyBumpAlloc {
    pub fn new(capacity: usize, alignment: usize) -> LeakyBumpAlloc {
        let layout = Layout::from_size_align(capacity, alignment).unwrap();
        let data = unsafe { System.alloc(layout) };
        LeakyBumpAlloc {
            layout,
            data,
            allocated: 0,
            capacity,
            alignment,
            start: data,
        }
    }

    #[doc(hidden)]
    // used for resetting the cache between benchmark runs. Do not call this.
    pub unsafe fn clear(&mut self) {
        System.dealloc(self.start, self.layout);
    }

    // Allocates a new chunk. Panics if out of memory.
    pub unsafe fn allocate(&mut self, num_bytes: usize) -> *mut u8 {
        let aligned_size = round_up_to(num_bytes, self.alignment);

        if self.allocated + aligned_size > self.capacity {
            eprintln!(
                "Allocator asked to bump to {} bytes with a capacity of {}",
                self.allocated + aligned_size,
                self.capacity
            );
            // we have to abort here or the mutex may deadlock
            std::process::abort();
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

    pub(crate) fn start(&self) -> *const u8 {
        self.start
    }

    pub(crate) fn end(&self) -> *const u8 {
        self.data
    }
}
