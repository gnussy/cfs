pub mod bitmap;
pub mod dir_entry;
pub mod inode;
pub mod superblock;

use bitmap::Bitmap;
use deku::prelude::*;
use std::io::{Read, Seek, Write};

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
        let bam_blocks = (nblocks + bits_per_block(block_size) - 1) / bits_per_block(block_size);
        let inode_list_blocks = (nblocks) / bits_per_block(block_size);
        let ninodes = inode_list_blocks * bits_per_block(block_size);
        let iam_blocks = (ninodes + bits_per_block(block_size) - 1) / bits_per_block(block_size);

        dbg!(block_size);
        dbg!(size);
        dbg!(nblocks);
        dbg!(bam_blocks);
        dbg!(inode_list_blocks);
        dbg!(ninodes);
        dbg!(iam_blocks);

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
        // Inode 0 is reserved
        // Inode 1 is the root directory
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

    pub fn add_dentry_to_inode(
        &mut self,
        inode_idx: usize,
        dentry_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // denty_name must be [u8; 60]
        let dentry_name = str_to_u8_60(dentry_name);
        let dentry = dir_entry::DirEntry::new(dentry_name, inode_idx as u32);
        let mut inode = self.cfs.inode_list.get(inode_idx);
        let nchildren = inode.nlinks;
        // now we have the inode, we can write the dentries to the inode.blkaddr
        // The first block in inode.blkaddr is reserved for the dentries
        // The rest of the blocks are for the data

        //read inode.blkaddr[0] into a buffer
        let offset = self.cfs.data_blocks_offset()
            + inode.blkaddr[0] as u64 * self.cfs.super_block.blocksize as u64;
        self.blk_dev.seek(std::io::SeekFrom::Start(offset))?;
        let mut buffer = vec![0; self.cfs.super_block.blocksize as usize];
        self.blk_dev.read_exact(&mut buffer)?;

        // inode.nlinks is the number of dentries in the first block, so we need to
        // write to the next available dentry
        let dentry_offset = nchildren as usize * std::mem::size_of::<dir_entry::DirEntry>();
        let dentry_data = dentry.to_bytes()?;

        // write the dentry to the buffer
        buffer[dentry_offset..dentry_offset + dentry_data.len()].copy_from_slice(&dentry_data);

        // write the buffer back to the file
        self.blk_dev.seek(std::io::SeekFrom::Start(
            offset * self.cfs.super_block.blocksize as u64,
        ))?;
        self.blk_dev.write_all(&buffer)?;

        dbg!(offset * self.cfs.super_block.blocksize as u64);
        dbg!(dentry_offset);

        // update the inode
        inode.nlinks += 1;
        self.cfs.inode_list.set(inode_idx, inode);

        // debug all related values

        self.write_cfs()?;

        Ok(())
    }
}

// 💨
impl Drop for CfsPartition {
    fn drop(&mut self) {
        self.blk_dev.sync_all().unwrap();
    }
}

// 💋
#[inline(always)]
pub fn bits_per_block(block_size: u64) -> u64 {
    block_size * 8
}

pub fn str_to_u8_60(s: &str) -> [u8; 60] {
    let mut a = [0; 60];
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        a[i] = bytes[i];
    }
    a
}
