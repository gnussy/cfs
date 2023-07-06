use std::io::{Read, Seek, Write};

use deku::prelude::*;

use crate::{
    bitmap::{self, Bitmap},
    dir_entry, inode, superblock,
    utils::{self, bits_per_block},
    Cfs, MAGIC,
};

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

        log::debug!("block_size: {block_size}");
        log::debug!("size: {size}");
        log::debug!("nblocks: {nblocks}");
        log::debug!("bam_blocks: {bam_blocks}");
        log::debug!("inode_list_blocks: {inode_list_blocks}");
        log::debug!("ninodes: {ninodes}");
        log::debug!("iam_blocks: {iam_blocks}");

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
        self.blk_dev.seek(std::io::SeekFrom::Start(0))?;
        self.blk_dev.write_all(&buffer)?;
        Ok(())
    }

    pub fn add_dentry_to_inode(
        &mut self,
        inode_idx: usize,
        dentry_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // a dentry_name must be at most [u8; 60]
        let dentry_name = utils::str_to_u8_60(dentry_name);
        let dentry = dir_entry::DirEntry::new(dentry_name, inode_idx as u32);
        let mut inode = self.cfs.inode_list.get(inode_idx);
        let nchildren = inode.nchildren;

        // now that we have the inode, we can write the dentries to the inode.blkaddr, we must
        // follow two rules:
        // 1. The first block in inode.blkaddr is reserved for the dentries
        // 2. The rest of the blocks are for the data

        // read inode.blkaddr[0] into a buffer
        let offset = (self.cfs.data_blocks_offset() + inode.blkaddr[0] as u64)
            * self.cfs.super_block.blocksize as u64;
        self.blk_dev.seek(std::io::SeekFrom::Start(offset))?;
        let mut buffer = vec![0; self.cfs.super_block.blocksize as usize];
        self.blk_dev.read_exact(&mut buffer)?;

        // inode.nchildren is the number of dentries in the first block, so we need to
        // write to the next available dentry
        let dentry_offset = nchildren as usize * std::mem::size_of::<dir_entry::DirEntry>();
        let dentry_data = dentry.to_bytes()?;

        // write the dentry to the buffer
        buffer[dentry_offset..dentry_offset + dentry_data.len()].copy_from_slice(&dentry_data);

        // write the buffer back to the file
        self.blk_dev.seek(std::io::SeekFrom::Start(offset))?;
        self.blk_dev.write_all(&buffer)?;

        log::debug!("offset * self.cfs.super_block.blocksize: {}", offset);
        log::debug!("dentry_offset: {dentry_offset}");
        log::debug!("dentry_data.len(): {}", dentry_data.len());

        // update the inode
        inode.nchildren += 1;
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
