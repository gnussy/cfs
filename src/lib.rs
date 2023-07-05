use deku::prelude::*;

pub const MAGIC: u32 = 0x0CF5B10C;
pub const DEFAULT_BLOCK_SIZE: usize = 4096;
pub const RESERVED_BLOCKS: u64 = 1;

#[inline(always)]
pub fn bits_per_block(block_size: u64) -> u64 {
    block_size * 8
}

// ┌────────────┬─────────────────────────┬─────────────────────────┬────────────┬──────────────┬─────┬──────────────┐
// │Super Block │ Block Allocation Bitmap │ Inode Allocation Bitmap │ Inode List │ Data Block 0 │ ... │ Data Block N │
// └────────────┴─────────────────────────┴─────────────────────────┴────────────┴──────────────┴─────┴──────────────┘

pub mod bitmap;
pub mod dir_entry;
pub mod inode;
pub mod superblock;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct Cfs {
    super_block: superblock::SuperBlock,
    bam: bitmap::Bitmap,
    iam: bitmap::Bitmap,
    #[deku(ctx = "*super_block")]
    inode_list: inode::InodeList,
}
