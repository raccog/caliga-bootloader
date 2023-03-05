//! Slab allocator implementation.

use core::{
    alloc::{AllocError, Allocator, Layout},
    cell::UnsafeCell,
    fmt::Debug,
    ptr::{self, NonNull},
};
#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

/// The error type returned when initializing a [`SlabAllocator`].
///
/// See [`SlabAllocator::new`] for more details on how to initialize a [`SlabAllocator`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlabAllocatorError {
    /// The storage has a different alignment than the allocator.
    InvalidAlignment,
    /// The allocator's storage was too small to contain a bitmap and a single slab.
    StorageTooSmall,
    /// The storage could not be divided into slabs; the storage's size should be divisible by
    /// the size of a single slab without any remainder.
    NonDivisibleSize,
}

// TODO: See if `slab_layout` can be implemented as a constant generic argument?
// TODO: See what can be done to ensure that the allocator is not freed before its slabs are freed.
/// A slab allocator can allocate evenly distributed memory chunks of the same size; called "slabs".
///
/// Each slab has the same [`Layout`] (meaning alignment and size).
///
/// # Constraints
///
/// This allocator can be backed by raw memory, so it can be used even in situations where there
/// is no existing allocator. Thus, it can be used as the first allocator to bootstrap a second
/// allocator.
///
/// See [`SlabAllocator::new`] for an example of initializing this allocator using raw memory.
#[derive(Debug)]
pub struct SlabAllocator {
    // `UnsafeCell<[u8]>` is used instead of `[u8]` so that `SlabAllocator::allocate()` can get
    // a mutable reference to the bitmap and allocation slabs. Without `UnsafeCell`,
    // `SlabAllocator::allocate()` would not be able to mutate its own storage because it does
    // not have a mutable reference to itself (`&mut self`).
    //
    // `NonNull<UnsafeCell>` is used instead of `&UnsafeCell` so that this allocator does not
    // need any lifetime annotations.
    allocated_storage: NonNull<UnsafeCell<[u8]>>,
    slab_layout: Layout,
}

// Since it uses interior mutability without any locking mechanism, this slab allocator should
// not be shared between multiple threads.
impl !Send for SlabAllocator {}
impl !Sync for SlabAllocator {}

impl SlabAllocator {
    /// Returns the bitmap used for keeping track of free slabs.
    fn bitmap(&self) -> &[u8] {
        unsafe { &self.storage()[self.buffer_size()..] }
    }

    /// Returns the mutable bitmap used for keeping track of free slabs.
    fn bitmap_mut(&self) -> &mut [u8] {
        unsafe { &mut self.storage_mut()[self.buffer_size()..] }
    }

    /// Returns the number of usable bits in the bitmap.
    ///
    /// Each usable bit corresponds to a single slab in the allocator's buffer. Unusable bits
    /// do not have any corresponding slab in the buffer and cannot be used for allocation.
    ///
    /// All bits after the last usable bit are marked with a `1` on initialization; signifying
    /// that they have no corresponding usable slab.
    fn bitmap_bits(&self) -> usize {
        self.buffer_size() / self.slab_layout.size()
    }

    /// Returns the size of the bitmap in bytes.
    ///
    /// This calculation includes any unusable bits.
    fn bitmap_size(&self) -> usize {
        let slab_count = unsafe { self.storage().len() / self.slab_layout.size() };

        const BITS: usize = u8::BITS as usize;
        let bitmap_size = slab_count / BITS;
        // If the slab count is not divisible by `8` without a remainder, then an extra byte is added
        // to the bitmap's size to account for the remaining bits.
        if slab_count % BITS != 0 {
            bitmap_size + 1
        } else {
            bitmap_size
        }
    }

    /// Returns the buffer used for slab allocation.
    fn buffer(&self) -> &[u8] {
        unsafe { &self.storage()[..self.buffer_size()] }
    }

    /// Returns the mutable buffer used for slab allocation.
    fn buffer_mut(&self) -> &mut [u8] {
        unsafe { &mut self.storage_mut()[..self.buffer_size()] }
    }

    /// Returns the size of the allocator's slab buffer in bytes.
    fn buffer_size(&self) -> usize {
        unsafe { self.storage().len() - self.bitmap_size() }
    }

    /// Returns the total number of slabs controlled by this allocator.
    pub fn capacity(&self) -> usize {
        self.buffer_size() / self.slab_layout.size()
    }

    /// Initializes a new slab allocator backed by `storage`, with each slab having the same `slab_layout`.
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
    /// Raw memory can be used when there is no currently avaible allocator.
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
    pub unsafe fn new(
        storage: &mut [u8],
        slab_layout: Layout,
    ) -> Result<SlabAllocator, SlabAllocatorError> {
        let layout_size = slab_layout.size();
        let storage_size = storage.len();
        if storage_size < layout_size * 2 {
            return Err(SlabAllocatorError::StorageTooSmall);
        }
        if storage_size % layout_size != 0 {
            return Err(SlabAllocatorError::NonDivisibleSize);
        }
        if !storage.as_ptr().is_aligned_to(slab_layout.align()) {
            return Err(SlabAllocatorError::InvalidAlignment);
        }

        storage.fill(0);

        let slab_allocator = SlabAllocator {
            allocated_storage: NonNull::new(storage as *mut [u8] as *mut UnsafeCell<[u8]>).unwrap(),
            slab_layout,
        };

        const U8_MAX: u8 = u8::MAX;
        let slab_count = slab_allocator.capacity();
        let unmasked_bits_count = slab_allocator.bitmap_bits() % u8::BITS as usize;
        let masked_bytes_start = slab_count / u8::BITS as usize;
        let bitmap = slab_allocator.bitmap_mut();

        // Mask the first partially-unusable byte of the bitmap
        if unmasked_bits_count != 0 {
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

        debug!(
            "{:#?}, storage_size: {:?}, slab_count: {:#?}, buffer_size: {:#?}, bitmap_size: {:#?}",
            slab_allocator,
            slab_allocator.storage().len(),
            slab_allocator.bitmap_bits(),
            slab_allocator.buffer_size(),
            slab_allocator.bitmap_size()
        );

        Ok(slab_allocator)
    }

    /// Returns the allocator's storage. It contains the allocator's slabs and bitmap.
    unsafe fn storage(&self) -> &[u8] {
        &*self.allocated_storage.as_ref().get()
    }

    /// Returns the allocator's mutable storage. It contains the allocator's slabs and bitmap.
    unsafe fn storage_mut(&self) -> &mut [u8] {
        &mut *self.allocated_storage.as_ref().get()
    }
}

unsafe impl Allocator for SlabAllocator {
    // Returns [`AllocError`] if:
    //
    // * `layout` does not match this slab allocator's slab layout; `(layout != self.slab_layout)`
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if self.slab_layout != layout {
            return Err(AllocError);
        }

        let bitmap = self.bitmap_mut();
        for (i, bitmap_part) in bitmap.iter_mut().enumerate() {
            if *bitmap_part < u8::MAX {
                let slab_bit = (*bitmap_part).trailing_ones() as usize;
                assert!(slab_bit < u8::BITS as usize);
                let slab_index = i * u8::BITS as usize + slab_bit;

                // Set bitmap to indicate that the memory location is now used
                *bitmap_part |= 1 << slab_bit;

                let slab_size = self.slab_layout.size();
                let slab_start = slab_index * slab_size;
                let slab_end = slab_start + slab_size;
                let slab = &mut self.buffer_mut()[slab_start..slab_end];
                debug!("Alloc {:#?}", slab.as_ptr());
                return Ok(NonNull::new(slab).unwrap());
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
    // * `layout` needs to match this allocator's slab layout
    unsafe fn deallocate(&self, alloc_ptr: NonNull<u8>, layout: Layout) {
        debug!("Dealloc {:#?}", alloc_ptr);

        // Ensure deallocation is valid
        // TODO: Remove assertions; they could be used to accidentally or maliciously
        //       crash the entire bootloader/kernel by using invalid deallocations
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

        // Zero out part of bitmap to indicate that the slab is free
        bitmap[byte_idx] &= !(1 << bit_idx);

        // Zero out freed memory so it cannot be leaked
        ptr::write_bytes(alloc_ptr as *mut u8, 0, self.slab_layout.size());
    }
}

// TODO: Add test for an invalid `storage` slice (such as null address or an invalid address range)
// TODO: Add test for a `Layout` that has a size different from its alignment
// TODO: Add test for a `Layout` that is larger than `u64`
#[cfg(test)]
mod tests {
    use super::*;
    use std::{boxed::Box, collections::VecDeque, mem, vec, vec::Vec};

    /// A `SlabAllocator` that uses a `Vec` to store its allocations
    #[allow(dead_code)]
    struct VecSlabAlloc {
        pub slab_allocator: SlabAllocator,
        pub layout: Layout,
        pub storage: Vec<u8>,
    }

    /// Initialize a slab allocator using memory from a `Vec`
    ///
    /// Only used in other tests
    fn init_slab_alloc<T>(size: usize) -> VecSlabAlloc {
        let mut storage: Vec<u8> = vec![0; size];
        let layout = Layout::new::<T>();
        let slab_allocator = unsafe {
            SlabAllocator::new(&mut storage[..], layout).expect("Failed to create allocator")
        };

        VecSlabAlloc {
            slab_allocator,
            layout,
            storage,
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
            let allocated = Box::try_new_in(data, slab_allocator).expect("Failed to allocate");
            assert_eq!(*allocated, data);

            Box::try_new_in(!data, slab_allocator).expect_err("Should have failed to allocate");
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

    /// Ensures that:
    ///
    /// * Slabs can be sequentially allocated and freed using `Box`s
    /// * The entire slab capacity can be filled
    /// * The entire slab capacity can be reallocated after being allocated and then freed
    /// * The layout of `u16` can be used
    #[test]
    fn sequential_allocations() {
        type DataType = u16;
        fn alloc_assert(slab_allocator: &SlabAllocator) {
            // Save allocations in a `Vec` so they are all deallocated at once
            let mut saved_allocations: Vec<Box<DataType, &SlabAllocator>> = vec![];
            let capacity = slab_allocator.capacity();

            // Fill allocator
            for i in 0..capacity {
                let alloc =
                    Box::try_new_in(i as DataType, slab_allocator).expect("Failed to allocate");
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

    /// Ensures that
    ///
    /// * Slabs can be non-sequentially allocated and freed using `Box`s
    /// * The entire slab capacity can be filled
    /// * The entire slab capacity can be reallocated after being allocated and then freed
    /// * The layout of `u32` can be used
    #[test]
    fn complex_allocations() {
        type DataType = u32;
        const SLAB_COUNT: usize = 7;
        fn alloc_assert(slab_allocator: &SlabAllocator) {
            // Save allocations in a `Vec` so they are all deallocated at once
            let mut saved_allocations: VecDeque<Box<DataType, &SlabAllocator>> = VecDeque::new();

            // Make all allocations
            for i in 0..SLAB_COUNT {
                let alloc =
                    Box::try_new_in(i as DataType, slab_allocator).expect("Failed to allocate");
                saved_allocations.push_back(alloc);
            }

            // Ensure allocations are set correctly
            for i in 0..SLAB_COUNT {
                assert_eq!(i as DataType, *saved_allocations[i]);
            }

            // Free even-indexed slabs
            for i in (0..SLAB_COUNT).step_by(2).rev() {
                saved_allocations.remove(i);
            }

            // Re-allocate
            for i in 0..SLAB_COUNT / 2 + 1 {
                let alloc =
                    Box::try_new_in(i as DataType, slab_allocator).expect("Failed to allocate");
                saved_allocations.push_back(alloc);
            }

            // Allocator should be full
            Box::try_new_in(0xff, slab_allocator).expect_err("Should have failed to allocate");

            // Free odd-indexed slabs
            for i in (1..SLAB_COUNT).step_by(2).rev() {
                saved_allocations.remove(i);
            }

            // Re-allocate
            for i in 0..SLAB_COUNT / 2 {
                let alloc =
                    Box::try_new_in(i as DataType, slab_allocator).expect("Failed to allocate");
                saved_allocations.push_back(alloc);
            }

            // Allocator should be full
            Box::try_new_in(0xff, slab_allocator).expect_err("Should have failed to allocate");

            // Free first half
            for _ in 0..SLAB_COUNT / 2 {
                saved_allocations.pop_front().unwrap();
            }

            // Free other half
            for _ in 0..saved_allocations.len() {
                saved_allocations.pop_front().unwrap();
            }
        }
        let alloc = init_slab_alloc::<DataType>((SLAB_COUNT + 1) * mem::size_of::<DataType>());
        let slab_allocator = &alloc.slab_allocator;

        // This is called twice to see if further allocations are successful after being freed
        alloc_assert(slab_allocator);
        alloc_assert(slab_allocator);
    }

    /// Ensures that:
    ///
    /// * A manual allocation returns a working pointer
    /// * After being freed, the deallocated memory is zeroed out
    /// * The layout of `u64` can be used
    #[test]
    fn manual_allocation() {
        type DataType = u64;
        let alloc = init_slab_alloc::<DataType>(8 * mem::size_of::<DataType>());
        let slab_allocator = alloc.slab_allocator;
        let layout = alloc.layout;

        // Manual allocation
        let allocated = slab_allocator.allocate(layout).expect("Failed to allocate");
        let data = unsafe { allocated.cast::<DataType>().as_mut() };

        // Ensure it's initialized as 0
        const ZERO: DataType = 0;
        assert_eq!(*data, ZERO);

        // Ensure it gets set correctly
        const DATA: DataType = 0xdeadbeef;
        *data = DATA;
        assert_eq!(*data, DATA);

        // Manual deallocation
        unsafe { slab_allocator.deallocate(allocated.cast::<u8>(), layout) };

        // Ensure it is set to 0 when deallocated
        assert_eq!(*data, 0);
    }

    /// Ensures that proper errors are returned for:
    ///
    /// * An invalid size
    /// * Incorrectly aligned memory
    #[test]
    fn invalid_layouts() {
        type DataType = u64;
        const NUM_SLABS: usize = 8;
        const LAYOUT: Layout = Layout::new::<DataType>();

        // Using a size that is not divisible by the slab size should cause an error
        let size = NUM_SLABS * mem::size_of::<DataType>() + 1;
        let mut storage: Vec<u8> = vec![0; size];
        let alloc_err = unsafe {
            SlabAllocator::new(&mut storage[..], LAYOUT)
                .expect_err("Should have failed to create allocator")
        };
        assert_eq!(alloc_err, SlabAllocatorError::NonDivisibleSize);

        // Using a size that is too small should cause an error
        let size = 1;
        let mut storage: Vec<u8> = vec![0; size];
        let alloc_err = unsafe {
            SlabAllocator::new(&mut storage[..], LAYOUT)
                .expect_err("Should have failed to create allocator")
        };
        assert_eq!(alloc_err, SlabAllocatorError::StorageTooSmall);

        // Using an invalid alignment should cause an error
        let size = NUM_SLABS * mem::size_of::<DataType>() + 1;
        let mut storage: Vec<u8> = vec![0; size];
        let alloc_err = unsafe {
            SlabAllocator::new(&mut storage[1..], LAYOUT)
                .expect_err("Should have failed to create allocator")
        };
        assert_eq!(alloc_err, SlabAllocatorError::InvalidAlignment);
    }

    /// Ensures that proper errors are returned for:
    ///
    /// * Using an invalid `Layout` for an allocation
    #[test]
    fn invalid_allocation() {
        type DataType = u8;
        const SLAB_COUNT: usize = 8;

        let alloc = init_slab_alloc::<DataType>(SLAB_COUNT * mem::size_of::<DataType>());
        let slab_allocator = &alloc.slab_allocator;

        // Using a layout that doesn't match the slab allocator should cause an error,
        // such as allocating a float (align 4) with a u8 allocator (align 1)
        Box::try_new_in(3.14159, slab_allocator).expect_err("Should have failed to allocate");
    }
}
