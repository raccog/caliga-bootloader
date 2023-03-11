use core::{mem, ops::Deref};

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

#[derive(Debug)]
#[repr(align(32))]
struct MemoryRegion<'a> {
    free_blocks: Option<&'a MemoryBlock<'a>>,
    next: Option<&'a MemoryRegion<'a>>,
    /// The size of this memory region (including the block headers and region header, but not including
    /// any unaligned bytes).
    size: usize,
    pre_size: u32,
    post_size: u32,
}

#[derive(Debug)]
#[repr(align(32))]
struct MemoryBlock<'a> {
    next: Option<&'a MemoryBlock<'a>>,
    /// The number of cells contained in this block (not including the block header).
    cell_count: usize,
    status: u32,
    _padding0: u32,
    _padding1: usize,
}

impl<'a> MemoryRegion<'a> {
    fn new(region: &'a mut [u8]) -> Result<&'a mut MemoryRegion<'a>, ()> {
        // This method (and others) assume that a region header is the same size as a block header
        debug_assert!(CELL_SIZE == REGION_HEADER_SIZE);

        // Ensure there will be enough room for a region header, block header, and a single cell,
        // even if the region is unaligned
        if region.len() < MINIMUM_REGION_SIZE {
            return Err(());
        }

        // Split region in case the start/end are unaligned
        let (pre_region, region, post_region) = unsafe { region.align_to_mut::<MemoryCell>() };
        debug_assert!(region.len() >= 3);
        debug_assert!(pre_region.len() < CELL_SIZE);
        debug_assert!(post_region.len() < CELL_SIZE);
        debug!("{:p}", region);
        debug!("{:?} {:?} {:?}", pre_region, region, post_region);

        // Split off region header from the rest of the cells
        let (region_header, cells) = region.split_at_mut(1);
        debug_assert_eq!(region_header.len(), 1);
        debug_assert!(cells.len() >= 2);
        let region_header = unsafe { &mut *(&mut region_header[0] as *mut MemoryCell as *mut MemoryRegion) };

        // Split off block header
        let (block_header, cells) = cells.split_at_mut(1);
        debug_assert_eq!(block_header.len(), 1);
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
        region_header.free_blocks = Some(block_header);
        region_header.next = None;
        // Add 2 here so that the region and block headers are counted in the region size
        region_header.size = (block_header.cell_count + 2) * CELL_SIZE;
        region_header.pre_size = pre_region.len() as u32;
        region_header.post_size = post_region.len() as u32;

        debug!("{:?}", region_header);

        Ok(region_header)
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
