pub mod bitmap;
pub mod dir_entry;
pub mod inode;
pub mod partition;
pub mod superblock;
pub mod utils;

use deku::prelude::*;

pub const MAGIC: u32 = 0x0CF5B10C;
pub const DEFAULT_BLOCK_SIZE: usize = 4096;
pub const RESERVED_BLOCKS: u64 = 1;
pub const ROOT_INODE: usize = 1;

pub fn init_library_logger() {
    env_logger::builder().format_timestamp(None).init();
}

// ┌────────────┬─────────────────────────┬─────────────────────────┬────────────┬──────────────┬─────┬──────────────┐
// │Super Block │ Block Allocation Bitmap │ Inode Allocation Bitmap │ Inode List │ Data Block 0 │ ... │ Data Block N │
// └────────────┴─────────────────────────┴─────────────────────────┴────────────┴──────────────┴─────┴──────────────┘
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct Cfs {
    super_block: superblock::SuperBlock,
    #[deku(ctx = "*super_block")]
    bam: bitmap::Bam,
    #[deku(ctx = "*super_block")]
    iam: bitmap::Iam,
    #[deku(ctx = "*super_block")]
    inode_list: inode::InodeList,
}

impl Cfs {
    pub fn new(
        super_block: superblock::SuperBlock,
        bam: bitmap::Bam,
        iam: bitmap::Iam,
        inode_list: inode::InodeList,
    ) -> Self {
        Self {
            super_block,
            bam,
            iam,
            inode_list,
        }
    }

    pub fn super_block_offset(&self) -> u64 {
        0
    }

    pub fn bam_offset(&self) -> u64 {
        self.super_block.blocksize as u64 * RESERVED_BLOCKS
    }

    pub fn iam_offset(&self) -> u64 {
        self.super_block.blocksize as u64 * RESERVED_BLOCKS + self.super_block.bam_blocks as u64
    }

    pub fn inode_list_offset(&self) -> u64 {
        self.super_block.blocksize as u64 * RESERVED_BLOCKS
            + self.super_block.bam_blocks as u64
            + self.super_block.iam_blocks as u64
    }

    pub fn data_blocks_offset(&self) -> u64 {
        self.super_block.blocksize as u64 * RESERVED_BLOCKS
            + self.super_block.bam_blocks as u64
            + self.super_block.iam_blocks as u64
            + self.super_block.inode_blocks as u64
    }
}
