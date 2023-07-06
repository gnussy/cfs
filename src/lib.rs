pub mod bitmap;
pub mod dir_entry;
pub mod inode;
pub mod superblock;

use bitmap::Bitmap;
use deku::prelude::*;
use std::io::Write;

pub const MAGIC: u32 = 0x0CF5B10C;
pub const DEFAULT_BLOCK_SIZE: usize = 4096;
pub const RESERVED_BLOCKS: u64 = 1;

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
}

pub struct CfsPartition {
    pub blk_dev: std::fs::File,
    pub cfs: Cfs,
}

impl CfsPartition {
    pub fn new(
        blk_dev: std::fs::File,
        block_size: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let blk_dev_metadata = blk_dev.metadata()?;
        let size = blk_dev_metadata.len();
        let nblocks = size / block_size;
        let bam_blocks = nblocks + bits_per_block(block_size) - 1 / bits_per_block(block_size);
        let inode_list_blocks = (nblocks / 4) / bits_per_block(block_size);
        let ninodes = inode_list_blocks * bits_per_block(block_size);
        let iam_blocks = (ninodes + bits_per_block(block_size) - 1) / bits_per_block(block_size);
        //let data_start = RESERVED_BLOCKS + bam_blocks + iam_blocks + inode_list_blocks;

        // Super block
        let super_block = superblock::SuperBlock::new(
            MAGIC,
            block_size as u32,
            bam_blocks as u32,
            iam_blocks as u32,
            inode_list_blocks as u32,
            nblocks as u32,
            ninodes as u32,
        );

        // BAM - Allocate a bitmap with the first block occupied by the root directory
        // and all other blocks free
        let mut bam = bitmap::Bam::new(bam_blocks as usize * block_size as usize);
        bam.set(0);

        // IAM - Allocate a bitmap with the first inode occupied by the root directory
        // and all other inodes free
        let mut iam = bitmap::Iam::new(iam_blocks as usize * block_size as usize);
        iam.set(0);
        iam.set(1);

        // Inode List - Allocate the first inode for the root directory
        let inode_list = inode::InodeList::new();

        // Create the CFS
        let cfs = Cfs::new(super_block, bam, iam, inode_list);

        Ok(Self { blk_dev, cfs })
    }

    // serialize the CFS to the block device
    pub fn write_cfs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let buffer = self.cfs.to_bytes()?;
        self.blk_dev.write_all(&buffer)?;
        Ok(())
    }
}

// 💋
#[inline(always)]
pub fn bits_per_block(block_size: u64) -> u64 {
    block_size * 8
}
