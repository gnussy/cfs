#![allow(dead_code)]
const MAGIC: u32 = 0x0CF5B10C;
const DEFAULT_BLOCK_SIZE: usize = 4096;

mod dir_entry;
mod inode;
mod superblock;
