use core::{mem, slice, ptr::NonNull};

#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

const REGION_HEADER_SIZE: usize = mem::size_of::<MemoryRegion>();
// TODO: Change name of cell so as to not conflict with Rust's Cell types?
const CELL_SIZE: usize = mem::size_of::<MemoryBlock>();
const MINIMUM_REGION_SIZE: usize = REGION_HEADER_SIZE + CELL_SIZE * 4;

const BLOCK_STATUS_FREE: u32 = 0x1;

#[derive(Clone, Copy, Debug)]
#[repr(align(32))]
struct MemoryCell([u8; CELL_SIZE]);

#[derive(Debug, PartialEq)]
#[repr(align(32))]
struct MemoryRegion {
    free_blocks: Option<NonNull<MemoryBlock>>,
    next: Option<NonNull<MemoryRegion>>,
    /// The size of this memory region (including the block headers and region header, but not including
    /// any unaligned bytes).
    size: usize,
    pre_size: u32,
    post_size: u32,
}

#[derive(Debug, PartialEq)]
#[repr(align(32))]
struct MemoryBlock {
    next: Option<NonNull<MemoryBlock>>,
    /// The number of cells contained in this block (not including the block header).
    cell_count: usize,
    status: u32,
    _padding0: u32,
    _padding1: usize,
}

#[derive(Debug)]
pub struct PhysicalAllocator {
    regions: Option<NonNull<MemoryRegion>>,
}

impl MemoryRegion {
    /// Returns the first block in this region.
    /// 
    /// # Constraints
    /// 
    /// * This region must contain a valid block directly after the region's header
    unsafe fn first_block(&mut self) -> &mut MemoryBlock {
        &mut *((self as *mut MemoryRegion).add(1) as *mut MemoryBlock)
    }

    /// Attempts to insert a `new_region` after this region. Returns true if successful.
    /// 
    /// # Constraints
    /// 
    /// * Must only be called in `PhysicalAllocator::insert_region`
    /// * Must be no other existing mutable references to `new_region`
    unsafe fn insert_after(&mut self, new_region: *mut MemoryRegion) -> bool {
        // Constraints allow this to be mutated
        let new_region = &mut *new_region;

        // TODO: Assert that regions are not initialized
        assert!((*new_region).next.is_none());

        if new_region < self {
            return false;
        }

        if let Some(mut next) = self.next {
            let next = next.as_mut();
            if &*new_region < next {
                (*new_region).next = self.next.take();
                self.next = Some(NonNull::new_unchecked(new_region));
                true
            } else {
                false
            }
        } else {
            self.next = Some(NonNull::new_unchecked(new_region));
            true
        }
    }

    // TODO: Change reference of new_region and first_region to a pointer
    unsafe fn insert_before(
        first_region: *mut *mut MemoryRegion,
        new_region: *mut MemoryRegion,
    ) -> bool {
        assert!((*new_region).next.is_none());

        if new_region > *first_region {
            return false;
        }

        (*new_region).next = Some(NonNull::new_unchecked(*first_region));
        *first_region = new_region;

        true
    }

    unsafe fn is_contiguous(&self, other: &MemoryRegion) -> bool {
        self.is_directly_after(other) || other.is_directly_after(self)
    }

    unsafe fn is_directly_after(&self, other: &MemoryRegion) -> bool {
        //assert!()

        let self_ptr = self as *const MemoryRegion as *const u8;
        let self_end = self_ptr.add(self.size + self.post_size());
        let other_ptr = other as *const MemoryRegion as *const u8;
        let other_start = other_ptr.sub(other.pre_size());

        self_end == other_start
    }

    unsafe fn is_overlapping(&self, other: &MemoryRegion) -> bool {
        let overlapping_before = (other > self)
            && ((self as *const MemoryRegion).add(self.size)
                > (other as *const MemoryRegion));
        let overlapping_after = (self > other)
            && ((other as *const MemoryRegion).add(other.size)
                > (self as *const MemoryRegion));

        overlapping_before || overlapping_after
    }

    // TODO: Change reference of new_region to a pointer
    unsafe fn merge(&mut self, new_region: *mut MemoryRegion) -> bool {
        // TODO: Assert regions are non-overlapping, and not initialized
        assert!((*new_region).next.is_none());

        if !self.is_contiguous(&*new_region) {
            return false;
        }

        let next = self.next.take();

        let (first, second) = match &*self < &*new_region {
            true => (self, &mut *new_region),
            false => (&mut *new_region, self),
        };

        assert!((first.post_size() + second.pre_size()) % CELL_SIZE == 0);
        assert!(first.post_size() + second.pre_size() <= CELL_SIZE);

        let size = first.size + first.post_size() + second.pre_size() + second.size;
        let pre_size = first.pre_size;
        let post_size = second.post_size;

        let cell_count = unsafe {
            let new_cell_count = second.first_block().cell_count
                + 2
                + ((first.post_size() + second.pre_size()) / CELL_SIZE);
            let cell_count = first.first_block().cell_count + new_cell_count;

            let new_cells_start =
                (first as *mut MemoryRegion).add(first.size / CELL_SIZE) as *mut MemoryCell;
            let new_cells = slice::from_raw_parts_mut(new_cells_start, new_cell_count);

            new_cells.fill(MemoryCell([0; CELL_SIZE]));

            cell_count
        };

        let first_block = unsafe { first.first_block() };
        first_block.cell_count = cell_count;

        first.size = size;
        first.pre_size = pre_size;
        first.post_size = post_size;
        first.next = next;

        true
    }

    fn new(region: &mut [u8]) -> Result<&mut MemoryRegion, ()> {
        // This method (and others) assume that a region header is the same size as a block header
        assert!(CELL_SIZE == REGION_HEADER_SIZE);

        // Ensure there will be enough room for a region header, block header, and a single cell,
        // even if the region is unaligned
        if region.len() < MINIMUM_REGION_SIZE {
            return Err(());
        }

        // Split region in case the start/end are unaligned
        let (pre_region, region, post_region) = unsafe { region.align_to_mut::<MemoryCell>() };
        assert!(region.len() >= 3);
        assert!(pre_region.len() < CELL_SIZE);
        assert!(post_region.len() < CELL_SIZE);
        debug!("{:p} {:p}", pre_region, region);
        debug!("Pre: {:?} Region: {:?} Post: {:?}", pre_region.len(), region.len() * CELL_SIZE, post_region.len());

        // Split off region header from the rest of the cells
        let (region_header, cells) = region.split_at_mut(1);
        assert_eq!(region_header.len(), 1);
        assert!(cells.len() >= 2);
        let region_header =
            unsafe { &mut *(&mut region_header[0] as *mut MemoryCell as *mut MemoryRegion) };

        // Split off block header
        let (block_header, cells) = cells.split_at_mut(1);
        assert_eq!(block_header.len(), 1);
        let block_header =
            unsafe { &mut *(&mut block_header[0] as *mut MemoryCell as *mut MemoryBlock) };

        // Zero out unaligned bytes and memory cells
        pre_region.fill(0);
        cells.fill(MemoryCell([0; CELL_SIZE]));
        post_region.fill(0);

        // Init first block
        block_header.next = None;
        block_header.cell_count = cells.len();
        block_header.status = BLOCK_STATUS_FREE;
        block_header._padding0 = 0;
        block_header._padding1 = 0;

        // Init region
        region_header.next = None;
        // Add 2 here so that the region and block headers are counted in the region size
        region_header.size = (block_header.cell_count + 2) * CELL_SIZE;
        region_header.pre_size = pre_region.len() as u32;
        region_header.post_size = post_region.len() as u32;
        region_header.free_blocks = unsafe { Some(NonNull::new_unchecked(block_header)) };

        debug!("{:?}", region_header);

        Ok(region_header)
    }

    fn post_size(&self) -> usize {
        self.post_size as usize
    }

    fn pre_size(&self) -> usize {
        self.pre_size as usize
    }
}

impl PartialOrd for MemoryRegion {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let other_ptr = other as *const MemoryRegion;
        Some((self as *const MemoryRegion).cmp(&other_ptr))
    }
}

impl PhysicalAllocator {
    fn insert_region<'a>(&'a mut self, new_region: &'a mut MemoryRegion) -> Result<(), ()> {
        if self.regions.is_none() {
            self.regions = unsafe { Some(NonNull::new_unchecked(new_region)) };
            return Ok(());
        }

        let mut first_region = unsafe { self.regions.unwrap().as_mut() };

        if unsafe { first_region.is_overlapping(new_region) } {
            return Err(());
        }

        if unsafe { MemoryRegion::insert_before(&mut (first_region as *mut MemoryRegion), new_region) } {
            self.regions = unsafe { Some(NonNull::new_unchecked(first_region)) };
            return Ok(());
        }

        let mut current_region = unsafe { Some(NonNull::new_unchecked(first_region)) };
        while let Some(mut region) = current_region {
            let region = unsafe { region.as_mut() };
            if unsafe { region.is_overlapping(new_region) } {
                return Err(());
            }

            if unsafe { region.merge(new_region) } {
                return Ok(());
            }

            if unsafe { region.insert_after(new_region) } {
                return Ok(());
            }

            current_region = region.next;
        }

        Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{mem, vec};

    #[test]
    fn init_region() {
        // A region should be the size of 4 pointers
        assert_eq!(REGION_HEADER_SIZE, mem::size_of::<usize>() * 4);
        // A cell/block header should be the size of 4 pointers
        assert_eq!(CELL_SIZE, mem::size_of::<usize>() * 4);

        // Initialize a region backed by allocated memory
        const REGION_SIZE: usize = 0x100;
        let mut backed_region: Vec<u8> = vec![0; REGION_SIZE];
        let start = 4;
        let from_end = 5;
        let end = REGION_SIZE - from_end;
        let start_ptr = (&backed_region[start] as *const u8);
        let end_ptr = (&backed_region[end] as *const u8);
        let region = MemoryRegion::new(&mut backed_region[start..end])
            .expect("Failed to initialize memory region");

        // Ensure the sizes match up correctly
        assert_eq!(region.size, REGION_SIZE - start - from_end - region.pre_size() - region.post_size());
        assert_eq!(region.pre_size(), start_ptr.align_offset(CELL_SIZE));
        assert_eq!(region.post_size(), CELL_SIZE - end_ptr.align_offset(CELL_SIZE));
    }
}
