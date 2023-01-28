# Physical Memory Allocation

NOTE: None of this is final. Just planning how I want my physical memory allocator to work.

I am calling this a "physical memory allocator" because the boot loader (or firmware) will identity map the available RAM before booting the kernel. Thus, although it may technically be working with virtual addresses, I will be referring to them as physical because the virtual and physical addresses will be identical for the entire boot process.

## Free Blocks List

[1] TODO: The first block of memory might not be large enough. Plan for what happens with many small blocks of memory.

At the beginning of the first free block of memory [1], an intrusive linked list is stored with entries for each free block of memory.

### Struct

Each entry for a free block of memory only needs two things; the starting address of the free block and the address of the next entry in the free block list.

```rust
# use std::ptr::NonNull;
# struct BlockHeader;
struct FreeBlockEntry {
    block: NonNull<BlockHeader>,
    next_entry: Option<NonNull<FreeBlockEntry>>
}
```

The `next_entry` is an `Option` because it will be `None` if the entry is the last one in the list.

## Block Structure

The block will store a header at the start:

```rust
struct BlockHeader {
    size: usize,
    is_free: bool
}
```

The first usable memory address after the block header can be retrieved with a function:

```rust
# struct BlockHeader;
impl BlockHeader {
    pub unsafe fn block_ptr(&self) -> *const u8 {
        // Alignment for start of each free block
        const ALIGN: usize = 8;
        
        // Get size of header
        let mut offset = std::mem::size_of::<BlockHeader>();
        
        // Ensure block is aligned properly
        let align_remainder = offset % ALIGN;
        if align_remainder != 0 {
            offset += ALIGN - align_remainder;
        }
        
        // Return address of block start
        (self as *const BlockHeader as *const u8)
            .add(offset)
    }
}
```
