#![allow(dead_code)]
pub const MAGIC: u32 = 0x0CF5B10C;
pub const DEFAULT_BLOCK_SIZE: usize = 4096;
pub const RESERVED_BLOCKS: u64 = 1;

#[inline(always)]
pub fn bits_per_block(block_size: u64) -> u64 {
    block_size * 8
}

pub mod dir_entry;
pub mod inode;
pub mod superblock;
