use core::mem;

const REGION_HEADER_SIZE: usize = mem::size_of::<MemoryRegion>();
const CELL_SIZE: usize = mem::size_of::<MemoryBlock>();
const MINIMUM_REGION_SIZE: usize = REGION_HEADER_SIZE + CELL_SIZE * 4;

const BLOCK_STATUS_FREE: u32 = 0x1;

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct MemoryCell([u8; CELL_SIZE]);

#[derive(Debug)]
struct MemoryRegion<'a> {
    free_blocks: Option<&'a MemoryBlock<'a>>,
    next: Option<&'a MemoryRegion<'a>>,
    size: usize,
    pre_size: u32,
    post_size: u32,
}

#[derive(Debug)]
struct MemoryBlock<'a> {
    next: Option<&'a MemoryBlock<'a>>,
    cell_count: usize,
    status: u32,
    _padding0: u32,
    _padding1: usize,
}

impl<'a> MemoryRegion<'a> {
    fn new(region: &'a mut [u8]) -> Result<&'a mut MemoryRegion<'a>, ()> {
        debug_assert!(CELL_SIZE == REGION_HEADER_SIZE);

        // Ensure there will be enough room for a region header, block header, and a single cell;
        // no matter what the alignment of `region` is
        if region.len() < MINIMUM_REGION_SIZE {
            return Err(());
        }

        let (pre_region, region, post_region) = unsafe { region.align_to_mut::<MemoryCell>() };
        debug_assert!(region.len() >= 3);
        let (region, cells) = region.split_at_mut(1);
        debug_assert_eq!(region.len(), 1);
        debug_assert!(cells.len() >= 2);
        debug_assert!(pre_region.len() < CELL_SIZE);
        debug_assert!(post_region.len() < CELL_SIZE);

        let region = unsafe { &mut *(&mut region[0] as *mut MemoryCell as *mut MemoryRegion) };

        let (block_header, cells) = cells.split_at_mut(1);
        debug_assert_eq!(block_header.len(), 1);
        let block_header =
            unsafe { &mut *(&mut block_header[0] as *mut MemoryCell as *mut MemoryBlock) };

        pre_region.fill(0);
        cells.fill(MemoryCell([0; CELL_SIZE]));
        post_region.fill(0);

        block_header.next = None;
        block_header.cell_count = cells.len();
        block_header.status = BLOCK_STATUS_FREE;
        block_header._padding0 = 0;
        block_header._padding1 = 0;

        region.free_blocks = Some(block_header);
        region.next = None;
        // Add 2 here so that the region and block headers are counted in the region size
        region.size = cells.len() + 2 * CELL_SIZE;
        region.pre_size = pre_region.len() as u32;
        region.post_size = post_region.len() as u32;

        Ok(region)
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

        let mut backed_region: Vec<u8> = vec![0; 0x100];
        let region = MemoryRegion::new(backed_region.as_mut_slice())
            .expect("Failed to initialize memory region");
    }
}
