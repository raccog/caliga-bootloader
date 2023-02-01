use core::{alloc::{Allocator, AllocError, Layout}, cell::UnsafeCell, fmt::Debug, ptr::{NonNull, self}, slice};
#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

/// Error cases when using a [`SlabAllocator`]
#[derive(Clone, Copy, Debug)]
pub enum SlabAllocatorError {
    /// The backed storage was not aligned properly
    InvalidAlignment,
    /// The backed storage was either too small, or did not have a size divisible by the size
    /// of a single slab
    InvalidSize,
}

/// A slab allocator can allocate evenly distributed memory chunks of the same size; called "slabs"
///
/// Each slab has the same [`Layout`].
///
/// # Constraints
///
/// This allocator can be backed by raw memory, so it can be used even in situations where there
/// is no existing allocator. Thus, this can be used as the first allocator to bootstrap a second
/// allocator.
///
/// See [`SlabAllocator::new`] for an example of initializing this allocator using raw memory.
#[derive(Debug)]
pub struct SlabAllocator {
    // NOTE: This is `NonNull` instead of just a reference so that there are no lifetime
    // annotations in this struct.
    // NOTE: This is `UnsafeCell` so that `self.allocate()` can get a mutable reference to
    // its contents without having a `&mut self`.
    allocated_storage: NonNull<UnsafeCell<[u8]>>,
    slab_layout: Layout,
}

// Since it uses interior mutability without any locking mechanism, this slab allocator should
// not be shared between multiple threads
impl !Send for SlabAllocator {}
impl !Sync for SlabAllocator {}

impl SlabAllocator {
    /// Returns the bitmap used for keeping track of free slabs
    fn bitmap(&self) -> &[u8] {
        unsafe { &self.storage()[self.buffer_size()..] }
    }

    /// Returns the mutable bitmap used for keeping track of free slabs
    fn bitmap_mut(&self) -> &mut [u8] {
        unsafe { &mut self.storage_mut()[self.buffer_size()..] }
     }

    /// Returns the number of usable bits in the bitmap
    ///
    /// Each usable bit corresponds to a single slab in the buffer. Unusable bits do not have any
    /// corresponding slab in the buffer and cannot be used for allocation.
    ///
    /// All bits after the last usable bit are marked with a `1` on initialization; signifying
    /// that they have no corresponding usable slab
    fn bitmap_bits(&self) -> usize {
        self.buffer_size() / self.slab_layout.size()
    }

    /// Returns the size of the bitmap in bytes
    ///
    /// This calculation includes any unusable bits
    fn bitmap_size(&self) -> usize {
        let slab_count = unsafe { self.storage().len() / self.slab_layout.size() };

        const BITS: usize = u8::BITS as usize;
        let bitmap_size = slab_count / BITS;

        // Ensure all bits are counted
        if slab_count % BITS != 0 {
            bitmap_size + 1
        } else {
            bitmap_size
        }
    }

    /// Returns the buffer used for slab allocation
    fn buffer(&self) -> &[u8] {
        unsafe { &self.storage()[..self.buffer_size()] }
    }

    /// Returns the size of the buffer (used for slab allocation) in bytes
    fn buffer_size(&self) -> usize {
        unsafe { self.storage().len() - self.bitmap_size() }
    }

    /// Returns the total number of slabs controlled by this allocator
    pub fn capacity(&self) -> usize {
        self.buffer_size() / self.slab_layout.size()
    }

    /// Initializes a new slab allocator backed by `storage`, with each slab having the same `slab_layout`
    ///
    /// # Errors
    ///
    /// [`SlabAllocatorError::InvalidSize`]:
    ///
    /// * `storage.len()` is not divisible by `slab_layout.size()`; `(storage.len() % slab_layout.size() != 0)`
    /// * `storage.len()` is not large enough to store two slabs of size `slab_layout.size()`;
    ///   `(storage.len() < slab_layout.size() * 2)`
    ///
    /// [`SlabAllocatorError::InvalidAlignment`]:
    ///
    /// * `storage` is not aligned to `slab_layout.align()`
    ///
    /// # Examples
    ///
    /// There are two examples:
    ///
    /// * Initialize this allocator with raw memory
    /// * Initialize this allocator with memory retrieved from another allocator
    ///
    /// ## Raw Memory
    ///
    /// ```
    /// # use std::{alloc::Layout, slice, vec};
    /// # use caliga_bootloader::common::slab_allocator::SlabAllocator;
    /// const MEMORY_SIZE: usize = 0x1000;
    /// # let memory = vec![0; MEMORY_SIZE];
    /// // This raw pointer could come from anywhere
    /// let raw_ptr: *const u8 = memory.as_ptr() as *const u8;
    /// let slab_allocator = unsafe {
    ///     let memory_slice: &mut [u8] = slice::from_raw_parts_mut(raw_ptr as *mut u8, MEMORY_SIZE);
    ///     SlabAllocator::new(memory_slice, Layout::new::<u8>())
    ///         .expect("Failed to initialize slab allocator")
    /// };
    /// ```
    ///
    /// ## Allocator-Backed Memory
    ///
    /// ```
    /// # use std::{alloc::Layout, vec, vec::Vec};
    /// # use caliga_bootloader::common::slab_allocator::SlabAllocator;
    /// const MEMORY_SIZE: usize = 0x1000;
    /// // This memory is allocated using another already-existing allocator
    /// let mut backed_memory: Vec<u8> = vec![0; MEMORY_SIZE];
    /// let slab_allocator = unsafe {
    ///     SlabAllocator::new(&mut backed_memory[..], Layout::new::<u8>())
    ///         .expect("Failed to initialize slab allocator")
    /// };
    /// ```
    pub unsafe fn new(storage: &mut [u8], slab_layout: Layout) -> Result<SlabAllocator, SlabAllocatorError> {
        let layout_size = slab_layout.size();
        let size = storage.len();
        if size % layout_size != 0 || size < layout_size * 2 {
            return Err(SlabAllocatorError::InvalidSize);
        }

        if !storage.as_ptr().is_aligned_to(slab_layout.align()) {
            return Err(SlabAllocatorError::InvalidAlignment);
        }

        // Zero out memory
        storage.fill(0);

        let slab_allocator = SlabAllocator {
            allocated_storage: NonNull::new(storage as *mut [u8] as *mut UnsafeCell<[u8]>).unwrap(), slab_layout
        };

        // Mask bits for memory that is unavailable
        // These masked bits are marked with `1`, showing the allocator that their corresponding
        // slabs are unavailable for allocation.
        let slab_count = slab_allocator.capacity();
        let unmasked_bits_count = slab_allocator.bitmap_bits() % u8::BITS as usize;
        let masked_bytes_start = slab_count / u8::BITS as usize;
        const U8_MAX: u8 = u8::MAX;
        let bitmap = slab_allocator.bitmap_mut();

        if unmasked_bits_count != 0 {
            // Mask the first partially-unusable byte of the bitmap
            // Part of this byte might still have usable bits, so `u8::MAX` needs
            // to be shifted to unset those usable bits.
            *&mut bitmap[masked_bytes_start] = U8_MAX << unmasked_bits_count;
        }

        // Mask any further unusable bits
        if masked_bytes_start < bitmap.len() - 1 {
            for bitmap_part in bitmap[masked_bytes_start + 1..].iter_mut() {
                *bitmap_part = U8_MAX;
            }
        }

        debug!("{:#?} bitmap: {:#?} {:#?} buffer: {:#?} bitmap_bits: {:#?} buffer_size: {:#?}", slab_allocator, slab_allocator.bitmap(), slab_allocator.bitmap(), slab_allocator.buffer(), slab_allocator.bitmap_bits(),
                slab_allocator.buffer_size());

        Ok(slab_allocator)
    }

    unsafe fn storage(&self) -> &[u8] {
        &*self.allocated_storage.as_ref().get()
    }

    unsafe fn storage_mut(&self) -> &mut [u8] {
        &mut *self.allocated_storage.as_ref().get()
    }
}

// Returns the index of the first free bit in a byte (0-7)
//
// Index 0 is the least significant bit, while 7 is the most significant.
//
// # Safety
//
// Panics if `byte == u8::MAX`
fn first_free_bit(mut byte: u8) -> usize {
    for i in 0..u8::BITS as usize {
        // Return index if the associated bit is zero
        if byte & 0x1 == 0 {
            return i;
        }

        // Check the next bit
        byte >>= 1;
    }

    unimplemented!();
}

unsafe impl Allocator for SlabAllocator {
    // Returns [`AllocError`] if:
    //
    // * `layout` does not match this slab allocator's slab layout; `(layout != self.slab_layout)`
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        // Return error if `layout` does not match `self.slab_layout`
        if self.slab_layout != layout {
            return Err(AllocError);
        }

        let bitmap = self.bitmap_mut();
        // Search each byte of the bitmap to find a free slab
        // NOTE: Free slabs are denoted by a `0` in the bitmap.
        for (i, bitmap_part) in bitmap.iter_mut().enumerate() {
            if *bitmap_part < u8::MAX {
                let slab_bit = first_free_bit(*bitmap_part);
                let slab_index = i * u8::BITS as usize + slab_bit;

                // Set bitmap to indicate that the memory location is now used
                *bitmap_part |= 1 << slab_bit;

                unsafe {
                    let ptr = &self.buffer()[slab_index * self.slab_layout.size()];
                    debug!("Alloc {:#?}", ptr);
                    return Ok(NonNull::new(slice::from_raw_parts_mut(ptr as *const u8 as *mut u8, self.slab_layout.size())).unwrap());
                }
            }
        }

        // No memory is available
        return Err(AllocError);
    }

    // # Safety
    //
    // This function has certain constraints around its inputs that need to be followed:
    //
    // * `alloc_ptr` needs to point to a valid slab contained in this allocator's buffer
    // * `layout` needs to match this slab allocator's slab layout
    unsafe fn deallocate(&self, alloc_ptr: NonNull<u8>, layout: Layout) {
        debug!("Dealloc {:#?}", alloc_ptr);

        // Ensure deallocation is valid
        // TODO: Determine if something other than assertions should be used
        let alloc_ptr = alloc_ptr.as_ptr() as *const u8;
        assert!(alloc_ptr >= self.buffer().as_ptr());
        assert!(alloc_ptr < self.bitmap().as_ptr());
        assert_eq!(self.slab_layout, layout);

        // Calculate indices for the bit that corresponds to this memory location
        let offset = alloc_ptr.sub_ptr(self.buffer().as_ptr());
        let slab_index = offset / self.slab_layout.size();
        let byte_idx = slab_index / u8::BITS as usize;
        let bit_idx = slab_index % u8::BITS as usize;

        // Ensure the index is valid
        let bitmap = self.bitmap_mut();
        assert!(byte_idx < bitmap.len());

        // Zero out bit in bitmap
        bitmap[byte_idx] &= !(1 << bit_idx);

        // Zero out freed memory
        ptr::write_bytes(alloc_ptr as *mut u8, 0, self.slab_layout.size());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, mem, vec, vec::Vec};

    /// A `SlabAllocator` that uses a `Vec` to store its allocations
    #[allow(dead_code)]
    struct VecSlabAlloc {
        pub slab_allocator: SlabAllocator,
        pub layout: Layout,
        pub storage: Vec<u8>,
    }

    /// Initialize a slab allocator with memory from a `Vec`
    ///
    /// Used in other tests
    fn init_slab_alloc<T>(size: usize) -> VecSlabAlloc {
        let mut storage: Vec<u8> = vec![0; size];
        let layout = Layout::new::<T>();
        let slab_allocator = unsafe {
            SlabAllocator::new(&mut storage[..], layout)
                .expect("Failed to create allocator")
        };

        VecSlabAlloc {
            slab_allocator,
            layout,
            storage
        }
    }

    /// Ensures that:
    ///
    /// * An allocator of the smallest possible size (1 slab where each slab is 1 byte) can be used
    /// * A single slab can be allocated
    /// * A single slab can be reallocated after being allocated and then freed
    /// * The layout of `u8` can be used
    #[test]
    fn smallest_allocation() {
        type DataType = u8;
        fn smallest_allocation_assert(data: DataType, slab_allocator: &SlabAllocator) {
            let allocated = Box::try_new_in(data, slab_allocator)
                .expect("Failed to allocate");
            assert_eq!(*allocated, data);

            Box::try_new_in(!data, slab_allocator)
                .expect_err("Should have failed to allocate");
        }

        let alloc = init_slab_alloc::<DataType>(2 * mem::size_of::<DataType>());
        let slab_allocator = &alloc.slab_allocator;

        // A single allocation should be available
        const DATA: DataType = 0xda;
        smallest_allocation_assert(DATA, slab_allocator);

        // Since the previous allocation was freed, a new one
        // should be available
        // A single allocation should be available
        smallest_allocation_assert(DATA, slab_allocator);
    }

    // Ensures that:
    //
    // * Slabs can be sequentially allocated and freed using `Box`s
    // * The entire slab capacity can be filled
    // * The entire slab capacity can be reallocated after being allocated and then freed
    // * The layout of `u16` can be used
    #[test]
    fn sequential_allocations() {
        type DataType = u16;
        fn alloc_assert(slab_allocator: &SlabAllocator) {
            // Save allocations in a `Vec` so they are all deallocated at once
            let mut saved_allocations: Vec<Box<DataType, &SlabAllocator>> = vec![];
            let capacity = slab_allocator.capacity();

            // Fill allocator
            for i in 0..capacity {
                let alloc = Box::try_new_in(i as DataType, slab_allocator)
                    .expect("Failed to allocate");
                saved_allocations.push(alloc);
            }

            // Ensure allocations are set correctly
            for i in 0..capacity {
                assert_eq!(i as DataType, *saved_allocations[i]);
            }

            // This allocation is expected to fail because there should be no more room for allocations
            Box::try_new_in(capacity as DataType, slab_allocator)
                .expect_err("Should have failed to allocate");
        }

        const SLAB_COUNT: usize = 100;
        let alloc = init_slab_alloc::<DataType>(SLAB_COUNT * mem::size_of::<DataType>());
        let slab_allocator = &alloc.slab_allocator;

        // This is called twice to see if further allocations are successful after being freed
        alloc_assert(slab_allocator);
        alloc_assert(slab_allocator);
    }

    /// Ensures that a single `u64` can be manually allocated and deallocated
    #[test]
    fn single_manual_allocation() {
        let alloc = init_slab_alloc::<u64>(8 * mem::size_of::<u64>());
        let slab_allocator = alloc.slab_allocator;
        let layout = alloc.layout;

        // Manual allocation
        let allocated = slab_allocator
            .allocate(layout)
            .expect("Failed to allocate");
        let data = unsafe {
            allocated
                .cast::<u64>()
                .as_mut()
        };

        // Ensure it's initialized as 0
        const ZERO: u64 = 0;
        assert_eq!(*data, ZERO);

        // Ensure it gets set correctly
        const DATA: u64 = 0xdeadbeef;
        *data = DATA;
        assert_eq!(*data, DATA);

        // Manual deallocation
        unsafe { slab_allocator.deallocate(allocated.cast::<u8>(), layout) };

        // Ensure it is set to 0 when deallocated
        assert_eq!(*data, 0);
    }

    /// Ensures that a single `f32` can be automatically allocated and deallocated using a `Box`
    #[test]
    fn single_auto_allocation() {
        let alloc = init_slab_alloc::<f32>(8 * mem::size_of::<f32>());
        let slab_allocator = alloc.slab_allocator;

        // Ensure float allocation works
        const DATA: f32 = 3.14159;
        let data = Box::try_new_in(DATA, slab_allocator)
            .expect("Failed to allocate");

        // Ensure it gets set correctly
        assert_eq!(*data, DATA);
    }

    /// Ensures that the entire section of memory owned by the allocator can be used for allocations
    ///
    /// Tests this by giving an allocator enough memory to allocate 7 `u16`s and ensures that they can all be allocated.
    ///
    /// Also tests that an 8th allocation will fail because there is not enough memory.
    #[test]
    fn multiple_auto_allocation() {
        // Init allocator with space for 7 `u16`s and 1 byte for the bitmap
        const NUM_ALLOC: usize = 7;
        let alloc = init_slab_alloc::<u16>((NUM_ALLOC + 1) * mem::size_of::<u16>());
        let slab_allocator = &alloc.slab_allocator;

        // This is inside a block so that the slab_allocator is not deallocated before its own allocations!
        {
            // Save allocations in a `Vec` so they are all deallocated at once
            let mut saved_allocations: Vec<Box<u16, &SlabAllocator>> = vec![];

            // Fill allocator
            for i in 0..NUM_ALLOC {
                let alloc = Box::try_new_in(i as u16, slab_allocator)
                    .expect("Failed to allocate");
                saved_allocations.push(alloc);
            }

            // Ensure allocations are set correctly
            for i in 0..NUM_ALLOC {
                assert_eq!(i as u16, *saved_allocations[i]);
            }

            // This allocation is expected to fail because there should be no more room for allocations
            Box::try_new_in(9 as u16, slab_allocator)
                .expect_err("Should have failed to allocate");
        }
    }
}
