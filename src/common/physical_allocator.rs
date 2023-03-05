//! This physical allocator is implemented as a singly linked list of continuous memory regions.
//!
//! The implementation is inspired by GRUB's memory allocator.
//!
//! Each memory region is non-contiguous with other memory regions, meaning unusable memory is contained
//! in beteween each memory region. Any regions that would potentially be contiguous are merged into a
//! single region.
//!
//! A `MemoryRegion` contains a circular, singly linked list of free memory blocks. Each free block has a
//! specified size of continuous memory. This size is represented as a count of 'cells', where
//! each cell is the size of 4 pointers (16 bytes on 32-bit and 32 bytes on 64-bit). Each cell is the same
//! size as a `MemoryBlock`; the header for all blocks of memory, used or free. The first block's address
//! is directly after the `MemoryRegion` struct.
//!
//!
//! A `MemoryBlock` contains a pointer to the next free block in its `MemoryRegion` and also a count of the
//! number of cells contained in the block. The first cell's address is directly after the `MemoryBlock` struct.
//!
//! ## Initialization Steps
//!
//! 1. Figure out which memory regions are usable
//! 2. For each memory region, initialize its header, initialize the first free block header, and insert it
//!    into the allocator's linked list of memory regions

use core::{
    mem,
    ptr::{self, NonNull},
};

/// The size of a single 'cell' contained in a memory block.
const CELL_SIZE: usize = mem::size_of::<MemoryBlock>();
/// The size of a `MemoryRegion` header.
const REGION_HEADER_SIZE: usize = mem::size_of::<MemoryRegion>();
/// The smallest possible region
const SMALLEST_REGION_SIZE: usize = REGION_HEADER_SIZE + CELL_SIZE * 2;

/// These status numbers are used to tell whether a memory block is free or used.
/// They can also be used for error detection; if a block's status does not match
/// the used or free statuses, another part of the program may have overwritten part
/// of the block header.
const BLOCK_STATUS_FREE: u32 = 0xdea1be7f;
const BLOCK_STATUS_USED: u32 = 0x6c2ef40d;

/// A region of continuous memory used for allocations.
#[derive(Debug, PartialEq, Eq)]
struct MemoryRegion {
    first_free_block: Option<NonNull<MemoryBlock>>,
    next: Option<NonNull<MemoryRegion>>,
    size: usize,
    pre_size: u32,
    post_size: u32,
    _padding: [usize; 4],
}

impl MemoryRegion {
    unsafe fn addr(&self) -> usize {
        self as *const MemoryRegion as usize
    }

    unsafe fn as_ptr(&mut self) -> *mut MemoryRegion {
        self as *mut MemoryRegion
    }

    /// Returns the first block of memory in this region.
    unsafe fn first_block(&mut self) -> *mut MemoryBlock {
        self.as_ptr().add(1) as *mut MemoryBlock
    }

    unsafe fn insert_block(&mut self, block: &mut MemoryBlock) {
        // TODO: Insert block to linked list in order
    }

    /// Returns true if this memory region is directly after `other`.
    unsafe fn is_directly_after(&self, other: &MemoryRegion) -> bool {
        self.region_end() == other.region_start()
    }

    /// Returns true if `other` is directly after this memory region or if this region is directly after `other`.
    unsafe fn is_contiguous(&self, other: &MemoryRegion) -> bool {
        self.is_directly_after(other) || (*other).is_directly_after(self)
    }

    /// Returns true if this region has a `next` region or if the region has allocated any blocks already.
    unsafe fn is_initialized(&self) -> bool {
        if self.next.is_some() {
            return true;
        }

        if let Some(block) = self.first_free_block {
            let block = block.as_ptr();
            (*block).cell_count * CELL_SIZE + CELL_SIZE + REGION_HEADER_SIZE != self.size
        } else {
            false
        }
    }

    unsafe fn is_overlapping(&self, other: &MemoryRegion) -> bool {
        (other.region_start() < self.region_end() && other.region_start() >= self.region_start())
            || (other.region_end() > self.region_start() && other.region_end() <= self.region_end())
    }

    /// Returns an iterator over the linked list of regions, starting with this region.
    unsafe fn iter(&mut self) -> MemoryRegionIter {
        MemoryRegionIter {
            current_region: Some(NonNull::new_unchecked(self)),
        }
    }

    /// Merges `other` with this region.
    ///
    /// # Safety
    ///
    /// This does not check if the two regions are contiguous before merging them.
    unsafe fn merge_unchecked<'a>(&'a mut self, other: &'a mut MemoryRegion) -> &mut MemoryRegion {
        // TODO: Complete merge
        //debug_assert!()

        let (first, second) = if self.as_ptr() < other.as_ptr() {
            (self, other)
        } else {
            (other, self)
        };

        //first.siz

        first
    }

    // TODO: Maybe check for existing memory regions in case this is called twice
    /// Initializes a new memory region at `addr`.
    ///
    /// This initialization includes:
    ///
    /// * Zeroing out the memory region
    /// * Creating the linked list with a single free block
    /// * Set up the block's metadata
    unsafe fn new(addr: usize, size: usize) -> Result<&'static mut Self, PhysicalAllocatorError> {
        // TODO: Ensure that address is not passed the end of valid address range
        //       idk how to do this on aarch64 or riscv64 yet.
        if usize::MAX - size < addr {
            return Err(PhysicalAllocatorError::InvalidRegionAddress { addr });
        }

        let addr = addr as *mut u8;
        if addr.is_null() {
            return Err(PhysicalAllocatorError::RegionIsNull);
        }

        let (pre_size, addr) = {
            let maybe_aligned_addr = addr as *mut MemoryRegion;
            if maybe_aligned_addr.is_aligned() {
                (0, maybe_aligned_addr)
            } else {
                let byte_offset = maybe_aligned_addr.align_offset(mem::align_of::<MemoryRegion>());

                (byte_offset, maybe_aligned_addr.byte_add(byte_offset))
            }
        };

        let post_size = (size - REGION_HEADER_SIZE - pre_size) % CELL_SIZE;

        debug_assert!(size >= pre_size + post_size);
        let size = size - pre_size - post_size;

        // This region needs to be large enough to contain a region header and at least two cells;
        // one cell for the block header, and at least one more for the allocation space. Note that
        // each cell is the size of a `MemoryBlock`.
        if size < SMALLEST_REGION_SIZE {
            return Err(PhysicalAllocatorError::RegionTooSmall {
                addr: addr as usize,
                size,
            });
        }

        debug_assert!(pre_size <= (u32::MAX as usize));
        debug_assert!(post_size <= (u32::MAX as usize));
        Ok(Self::new_unchecked(
            addr,
            size,
            pre_size as u32,
            post_size as u32,
        ))
    }

    unsafe fn new_unchecked(
        addr: *mut MemoryRegion,
        size: usize,
        pre_size: u32,
        post_size: u32,
    ) -> &'static mut Self {
        // A mutable reference is more convenient
        let region = &mut *(addr);

        let first_free_block = &mut *region.first_block();
        let cell_count = (size - REGION_HEADER_SIZE) / CELL_SIZE - CELL_SIZE;
        // Each cell should be zeroed out before being allocated to prevent data leaks
        ptr::write_bytes(
            (first_free_block as *mut MemoryBlock).add(1),
            0,
            cell_count * CELL_SIZE,
        );

        // Initialize a circular linked list of free blocks.
        // At first this is only a single block
        let first_free_block_ptr = NonNull::new_unchecked(first_free_block);
        first_free_block.next = Some(first_free_block_ptr);
        first_free_block.cell_count = cell_count;
        first_free_block.status = BLOCK_STATUS_FREE;
        first_free_block._padding0 = 0;
        first_free_block._padding1 = 0;

        // Initialize region
        region.first_free_block = Some(first_free_block_ptr);
        region.next = None;
        region.size = size;
        region.pre_size = pre_size;
        region.post_size = post_size;
        region._padding = [0; 4];

        region
    }

    fn next(&mut self) -> Option<NonNull<MemoryRegion>> {
        self.next
    }

    fn post_size(&self) -> usize {
        self.post_size as usize
    }

    fn pre_size(&self) -> usize {
        self.pre_size as usize
    }

    unsafe fn region_end(&self) -> usize {
        let region_start = self.addr();
        debug_assert!(usize::MAX - self.size - self.post_size() >= region_start);
        region_start + self.size + self.post_size()
    }

    unsafe fn region_start(&self) -> usize {
        let region_start = self.addr();
        debug_assert!(region_start >= self.pre_size());
        region_start - self.pre_size()
    }
}

// This allows memory regions to be ordered by their addresses.
// It does not account for the `pre_size`, `size`, or `post_size`. Only the aligned addresses are compared.
impl PartialOrd<MemoryRegion> for MemoryRegion {
    fn partial_cmp(&self, other: &MemoryRegion) -> Option<core::cmp::Ordering> {
        unsafe {
            let other = other.addr();
            Some(self.addr().cmp(&other))
        }
    }
}

/// An iterator over a linked list of memory regions.
struct MemoryRegionIter {
    current_region: Option<NonNull<MemoryRegion>>,
}

impl Iterator for MemoryRegionIter {
    type Item = *mut MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(region) = self.current_region {
            let region = unsafe { &mut *region.as_ptr() };

            self.current_region = region.next();

            Some(region)
        } else {
            None
        }
    }
}

/// A block of memory used for allocations.
struct MemoryBlock {
    next: Option<NonNull<MemoryBlock>>,
    cell_count: usize,
    status: u32,
    _padding0: u32,
    _padding1: usize,
}

pub enum PhysicalAllocatorError {
    InvalidRegionAddress { addr: usize },
    NoRegions,
    OverlappingRegion,
    RegionIsNull,
    RegionTooSmall { addr: usize, size: usize },
}

pub struct PhysicalAllocator {
    first_region: Option<NonNull<MemoryRegion>>,
}

impl PhysicalAllocator {
    /// Initialize a new physical allocator that uses a `memory_map` to provide dynamic allocation.
    pub unsafe fn new(memory_map: &[(usize, usize)]) -> Result<Self, PhysicalAllocatorError> {
        if memory_map.len() == 0 {
            return Err(PhysicalAllocatorError::NoRegions);
        }

        let mut allocator = PhysicalAllocator { first_region: None };

        for (addr, size) in memory_map {
            let region = MemoryRegion::new(*addr, *size)?;
            allocator.insert_region(region)?;
        }

        // TODO: insert regions

        Ok(allocator)
    }

    unsafe fn insert_region(
        &mut self,
        new_region: &'static mut MemoryRegion,
    ) -> Result<(), PhysicalAllocatorError> {
        // Inserted regions should not have allocated any memory yet
        debug_assert!(!new_region.is_initialized());

        let inserted_region = Some(NonNull::new_unchecked(new_region));
        if let Some(first_region) = self.first_region {
            let first_region = &mut *first_region.as_ptr();
            if first_region.is_overlapping(new_region) {
                return Err(PhysicalAllocatorError::OverlappingRegion);
            }

            // TODO: Try to merge with first region

            // Insert if the new region is before the first region
            if new_region < first_region {
                (*new_region).next = Some(NonNull::new_unchecked(first_region));
                self.first_region = inserted_region;
                return Ok(());
            }

            // Otherwise, each already existing region should be checked to see if it is contigious
            // with the new region. If so, the two regions should be merged.
            for region in (*self.first_region.unwrap().as_ptr()).iter() {
                if let Some(next_region) = (*region).next {
                    let next_region = &mut *next_region.as_ptr();
                    if new_region.is_overlapping(next_region) {
                        return Err(PhysicalAllocatorError::OverlappingRegion);
                    }

                    // TODO: Try to merge regions

                    // Insert if the new region is after this one, but before the next one
                    if new_region < next_region {
                        (*new_region).next = Some(NonNull::new_unchecked(next_region));
                        (*region).next = inserted_region;
                        return Ok(());
                    }
                } else {
                    // Region is inserted at the end
                    (*region).next = inserted_region;
                    return Ok(());
                }
            }
        } else {
            // This is easy if there is no first region yet
            self.first_region = inserted_region;
            return Ok(());
        }

        unimplemented!();
    }
}
