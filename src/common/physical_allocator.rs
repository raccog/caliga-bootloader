use core::{mem, slice};

#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

const REGION_HEADER_SIZE: usize = mem::size_of::<MemoryRegion>();
// TODO: Change name of cell so as to not conflict with Rust's UnsafeCell?
const CELL_SIZE: usize = mem::size_of::<MemoryBlock>();
const MINIMUM_REGION_SIZE: usize = REGION_HEADER_SIZE + CELL_SIZE * 4;

const BLOCK_STATUS_FREE: u32 = 0x1;

#[derive(Clone, Copy, Debug)]
#[repr(align(32))]
struct MemoryCell([u8; CELL_SIZE]);

#[derive(Debug, PartialEq)]
#[repr(align(32))]
struct MemoryRegion<'a> {
    free_blocks: Option<&'a mut MemoryBlock<'a>>,
    next: Option<&'a mut MemoryRegion<'a>>,
    /// The size of this memory region (including the block headers and region header, but not including
    /// any unaligned bytes).
    size: usize,
    pre_size: u32,
    post_size: u32,
}

#[derive(Debug, PartialEq)]
#[repr(align(32))]
struct MemoryBlock<'a> {
    next: Option<&'a mut MemoryBlock<'a>>,
    /// The number of cells contained in this block (not including the block header).
    cell_count: usize,
    status: u32,
    _padding0: u32,
    _padding1: usize,
}

#[derive(Debug)]
pub struct PhysicalAllocator {
    regions: Option<&'static mut MemoryRegion<'static>>,
}

impl<'a> MemoryRegion<'a> {
    unsafe fn first_block(&mut self) -> &mut MemoryBlock<'a> {
        &mut *((self as *mut MemoryRegion<'a>).add(1) as *mut MemoryBlock<'a>)
    }


    // TODO: Change reference of new_region to a pointer
    fn insert_after(&'a mut self, new_region: &'a mut MemoryRegion<'a>) -> bool {
        assert!(new_region.next.is_none());

        if new_region < self {
            return false;
        }

        new_region.next = self.next.take();
        self.next = Some(new_region);

        true
    }

    // TODO: Change reference of new_region and first_region to a pointer
    unsafe fn insert_before(
        first_region: *mut &'a mut MemoryRegion<'a>,
        new_region: &'a mut MemoryRegion<'a>,
    ) -> bool {
        assert!(new_region.next.is_none());

        if *first_region < new_region {
            return false;
        }

        new_region.next = Some(*first_region);
        *first_region = new_region;

        true
    }

    // TODO: Change reference of other to a pointer
    unsafe fn is_overlapping(&self, other: &MemoryRegion<'a>) -> bool {
        let overlapping_before = ((other > self)
            && ((self as *const MemoryRegion<'a>).add(self.size)
                > (other as *const MemoryRegion<'a>)));
        let overlapping_after = ((self > other)
            && ((other as *const MemoryRegion<'a>).add(other.size)
                > (self as *const MemoryRegion<'a>)));

        overlapping_before || overlapping_after
    }

    // TODO: Change reference of new_region to a pointer
    fn merge(&'a mut self, new_region: &'a mut MemoryRegion<'a>) -> &'a mut MemoryRegion<'a> {
        // TODO: Assert regions are contiguous, non-overlapping, and not initialized
        assert!(new_region.next.is_none());

        let next = self.next.take();

        let (first, second) = match self < new_region {
            true => (self, new_region),
            false => (new_region, self),
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
                (first as *mut MemoryRegion<'a>).add(first.size / CELL_SIZE) as *mut MemoryCell;
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

        first
    }

    fn new(region: &'a mut [u8]) -> Result<&'a mut MemoryRegion<'a>, ()> {
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
        debug!("{:p}", region);
        debug!("{:?} {:?} {:?}", pre_region, region, post_region);

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
        region_header.free_blocks = Some(block_header);

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

impl<'a> PartialOrd for MemoryRegion<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let other_ptr = other as *const MemoryRegion<'a>;
        Some((self as *const MemoryRegion<'a>).cmp(&other_ptr))
    }
}

impl PhysicalAllocator {
    fn insert_region(&'static mut self, new_region: &'static mut MemoryRegion<'static>) -> Result<(), ()> {
        if self.regions.is_none() {
            self.regions = Some(new_region);
            return Ok(());
        }

        let first_region = self.regions.as_mut().unwrap();
        if unsafe { (*first_region).is_overlapping(new_region) } {
            return Err(());
        }

        if unsafe { MemoryRegion::insert_before(first_region as *mut &mut MemoryRegion<'static>, new_region) } {
            return Ok(());
        }

        let mut current_region = &mut self.regions;
        while let Some(ref mut region) = current_region {
            if region.insert_after(new_region) {
                return Ok(());
            }

            current_region = &mut region.next;
        }

        todo!()
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
        let end = REGION_SIZE - 5;
        let region = MemoryRegion::new(&mut backed_region[start..end])
            .expect("Failed to initialize memory region");

        // Ensure the sizes match up correctly
        assert_eq!(region.size, REGION_SIZE - CELL_SIZE * 2);
        assert_eq!(region.pre_size as usize, CELL_SIZE - start);
        assert_eq!(region.post_size as usize, end % CELL_SIZE);
    }
}
