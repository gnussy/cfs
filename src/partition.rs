use std::{
    io::{Read, Seek, Write},
    os::unix::prelude::{MetadataExt, PermissionsExt},
    time::UNIX_EPOCH,
};

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
        let bits_per_block = bits_per_block(block_size);

        let bam_blocks = (nblocks + bits_per_block - 1) / bits_per_block;

        let inode_size = std::mem::size_of::<inode::Inode>() as u64;
        let inodes_per_block = block_size / inode_size;
        let ninodes = (nblocks / 4096) * inodes_per_block;

        let iam_blocks = (ninodes + bits_per_block - 1) / bits_per_block;

        let inode_list_blocks = (ninodes * inode_size + block_size - 1) / block_size;

        log::debug!("Partition information:");
        log::debug!("inodes_per_block: {inodes_per_block}");
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

        log::debug!(
            "ninodes * std::mem::size_of::<inode::Inode>() = {}",
            ninodes * std::mem::size_of::<inode::Inode>() as u64
        );

        // Inode List - Allocate the first inode for the root directory
        let inode_list = inode::InodeList::new(ninodes as usize);

        // tthe total number of blocks used by the CFS
        let total_blocks = 1 + bam_blocks + iam_blocks + inode_list_blocks;
        log::debug!("total_blocks: {total_blocks}");

        // Create the CFS
        let cfs = Cfs::new(super_block, bam, iam, inode_list);

        log::debug!("bam is located @ {}", cfs.bam_offset());
        log::debug!("iam is located @ {}", cfs.iam_offset());
        log::debug!("inode_list is located @ {}", cfs.inode_list_offset());
        log::debug!("data_blocks_offset: {}\n", cfs.data_blocks_offset());

        Ok(Self { blk_dev, cfs })
    }

    pub fn info(&self) -> (String, String, String, String) {
        (
            self.cfs.bam_offset().to_string(),
            self.cfs.iam_offset().to_string(),
            self.cfs.inode_list_offset().to_string(),
            self.cfs.data_blocks_offset().to_string(),
        )
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
        parent_inode_idx: usize,
        dentry_name: &str,
        inode_idx: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!(
            "add_dentry_to_inode(parent_inode_idx: {}, dentry_name: {}, inode_idx: {})",
            parent_inode_idx,
            dentry_name,
            inode_idx
        );
        // a dentry_name must be at most [u8; 60]
        let dentry_name = utils::str_to_u8_60(dentry_name);
        let dentry = dir_entry::DirEntry::new(dentry_name, inode_idx as u32);
        let mut inode = self.cfs.inode_list.get(parent_inode_idx);
        let nchildren = inode.nchildren;

        // now that we have the inode, we can write the dentries to the inode.blkaddr, we must
        // follow two rules:
        // 1. The first block in inode.blkaddr is reserved for the dentries
        // 2. The rest of the blocks are for the data

        // read inode.blkaddr[0] into a buffer
        let offset = self.cfs.data_blocks_offset()
            + (inode.blkaddr[0] as u64 * self.cfs.super_block.blocksize as u64);
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

        log::debug!("data_blocks_offset * self.cfs.super_block.blocksize: {offset}");
        log::debug!("dentry_offset: {dentry_offset}");
        log::debug!("dentry_data.len(): {}\n", dentry_data.len());

        // update the inode
        inode.nchildren += 1;
        self.cfs.inode_list.set(parent_inode_idx, inode);

        // debug all related values

        self.write_cfs()?;

        Ok(())
    }

    pub fn add_file_to_inode(
        &mut self,
        parent_inode_idx: usize,
        name: &str,
        file: &mut std::fs::File,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("File {name} added in parent inode {parent_inode_idx}");
        let metadata: std::fs::Metadata = file.metadata()?;
        let size = metadata.len();
        let fmode = metadata.permissions().mode();
        let uid = metadata.uid();
        let gid = metadata.gid();
        let atime = metadata.accessed()?;
        let mtime = metadata.modified()?;
        let ctime = metadata.created()?;

        // we need to allocate a new inode for the file
        let inode_idx = match self.cfs.iam.first_free() {
            Some(inode_idx) => {
                self.cfs.iam.set(inode_idx);
                inode_idx
            }
            None => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No free inodes",
                )));
            }
        };

        // we need to allocate the file data blocks
        // NOTE: No time for indirect blocks
        let nblocks = (size as f64 / self.cfs.super_block.blocksize as f64).ceil() as usize;
        let mut blkaddr = [0; 10];

        for (i, addr) in blkaddr.iter_mut().take(nblocks + 1).enumerate() {
            if let Some(block_idx) = self.cfs.bam.first_free() {
                self.cfs.bam.set(block_idx);
                *addr = block_idx as u32;
                log::debug!("blkaddr[{}]: {}", i, *addr);
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No free blocks",
                )));
            }
        }

        // now we need to write the file data to the blocks
        // Remember that the first block in inode.blkaddr is reserved for the dentries
        let mut buffer = vec![0; self.cfs.super_block.blocksize as usize];
        for addr in blkaddr.iter().take(nblocks + 1).skip(1) {
            // read the block into the buffer
            let offset = self.cfs.data_blocks_offset()
                + (*addr as u64 * self.cfs.super_block.blocksize as u64);
            self.blk_dev.seek(std::io::SeekFrom::Start(offset))?;
            self.blk_dev.read_exact(&mut buffer)?;

            log::debug!("Writing {name} @ {offset}");

            // write the file data to the buffer
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }

            // write the buffer back to the file
            self.blk_dev.seek(std::io::SeekFrom::Start(offset))?;
            self.blk_dev.write_all(&buffer)?;
        }

        // now we need to create the inode
        let inode = inode::Inode::new(
            fmode as u16,
            0,
            uid as u16,
            gid as u16,
            size as u32,
            atime.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            mtime.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            ctime.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            blkaddr,
        );
        self.cfs.inode_list.set(inode_idx, inode);

        // add dentry to parent inode
        log::debug!("parent_inode_idx: {}", parent_inode_idx);
        log::debug!("name: {}", name);
        log::debug!("inode_idx: {}", inode_idx);
        self.add_dentry_to_inode(parent_inode_idx, name, inode_idx)?;

        Ok(())
    }

    // This function is used ub the same way as add_file_to_inode
    // but it just add a directory instead of a file
    pub fn add_dir_to_inode(
        &mut self,
        parent_inode_idx: usize,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let size = 0;
        let fmode = 0o755;
        let uid = std::process::id();
        let gid = std::process::id();
        let atime = std::time::SystemTime::now();
        let mtime = atime;
        let ctime = atime;

        // we need to allocate a new inode for the uppcomming directory
        let inode_idx = match self.cfs.iam.first_free() {
            Some(inode_idx) => {
                self.cfs.iam.set(inode_idx);
                inode_idx
            }
            None => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "No free inodes",
                )));
            }
        };

        // The dir does not have any data blocks, so we just set the blkaddr to 0
        let mut blkaddr = [0; 10];
        if let Some(block_idx) = self.cfs.bam.first_free() {
            self.cfs.bam.set(block_idx);
            blkaddr[0] = block_idx as u32;
            log::debug!("blkaddr[0]: {}", blkaddr[0]);
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No free blocks",
            )));
        }

        // now we need to create the inode
        let inode = inode::Inode::new(
            fmode as u16,
            0,
            uid as u16,
            gid as u16,
            size as u32,
            atime.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            mtime.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            ctime.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            blkaddr,
        );
        self.cfs.inode_list.set(inode_idx, inode);

        // add dentry to parent inode
        log::debug!("parent_inode_idx: {}", parent_inode_idx);
        log::debug!("name: {}", name);
        log::debug!("inode_idx: {}", inode_idx);
        self.add_dentry_to_inode(parent_inode_idx, name, inode_idx)?;

        Ok(())
    }

    // This function is used to get the file data from the inode data blocks
    pub fn get_data_from_inode(
        &mut self,
        inode_idx: usize,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // get the inode from the inode list
        let inode = self.cfs.inode_list.get(inode_idx);

        log::debug!("inode: {:?}", inode);

        // the number of blocks that the file is using (ceil(size / blocksize)
        let size = (inode.size as f64 / self.cfs.super_block.blocksize as f64).ceil() as u32;
        log::debug!("size: {}", size);

        // the buffer that we will read the file data into
        let mut buffer = vec![0; inode.size as usize];

        let mut ret = Vec::new();

        // read the data blocks into the buffer
        for i in 1..=size {
            // read the block into the buffer
            let offset = self.cfs.data_blocks_offset()
                + (inode.blkaddr[i as usize] as u64 * self.cfs.super_block.blocksize as u64);
            self.blk_dev.seek(std::io::SeekFrom::Start(offset))?;
            self.blk_dev.read_exact(&mut buffer)?;
            ret.extend_from_slice(&buffer);
        }

        Ok(ret)
    }

    pub fn remove_inode(&mut self, inode_idx: usize) -> Result<(), Box<dyn std::error::Error>> {
        // get the inode from the inode list
        let inode = self.cfs.inode_list.get(inode_idx);

        // free the inode
        self.cfs.iam.clear(inode_idx);
        self.cfs.inode_list.clear(inode_idx);

        // free the data blocks that the inode points to
        let nblocks = inode.size / self.cfs.super_block.blocksize;
        for i in 0..nblocks {
            self.cfs.bam.clear(inode.blkaddr[i as usize] as usize);
        }

        self.write_cfs()?;

        Ok(())
    }

    // This function will delete a dentry from the inode,
    // and also delete the inode
    pub fn remove_dir_from_inode(
        &mut self,
        parent_inode_idx: usize,
        inode_idx: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dentries = self.list_dentries_from_inode(parent_inode_idx)?;

        // find the dentry that we want to remove, filter it, and map it to a new vector, and map it to a new vector of u8s,
        // and then write it to the data block

        let mut buffer: Vec<u8> = dentries
            .into_iter()
            .filter(|x| x.inode != inode_idx)
            .map(|x| x.to_bytes())
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect();

        // pad the buffer with 0s
        buffer.resize(self.cfs.super_block.blocksize as usize, 0);

        // write the dentries to the data block
        let inode = self.cfs.inode_list.get(parent_inode_idx);
        let data_block_idx = inode.blkaddr[0] as usize;
        let index = self.cfs.data_blocks_offset()
            + data_block_idx as u64 * self.cfs.super_block.blocksize as u64;
        std::io::Seek::seek(&mut self.blk_dev, std::io::SeekFrom::Start(index))?;
        std::io::Write::write_all(&mut self.blk_dev, &buffer)?;

        // remove the inode
        self.remove_inode(inode_idx as usize)?;
        Ok(())
    }

    pub fn list_dentries_from_inode(
        &mut self,
        parent_inode_idx: usize,
    ) -> Result<
        Vec<dir_entry::DirEntry>, /* Or perhaps Vec<(String, u32)>?*/
        Box<dyn std::error::Error>,
    > {
        // get the inode from the inode list
        let inode = self.cfs.inode_list.get(parent_inode_idx);

        // the dentrty is stored in the inode data block 0
        let data_block_idx = inode.blkaddr[0] as usize;
        log::debug!("data_block_idx: {}", data_block_idx);
        let index = self.cfs.data_blocks_offset()
            + data_block_idx as u64 * self.cfs.super_block.blocksize as u64;
        let mut buf = vec![0; self.cfs.super_block.blocksize as usize];

        // seek and read the data block
        std::io::Seek::seek(&mut self.blk_dev, std::io::SeekFrom::Start(index))?;
        std::io::Read::read_exact(&mut self.blk_dev, &mut buf)?;

        // the nunmber of dentries in the data block is in inode.nchildren
        let nchildren = inode.nchildren as usize;

        // convert the buffer to a vector of dentries
        let dentries = buf
            .chunks_exact(std::mem::size_of::<dir_entry::DirEntry>())
            .to_owned()
            .take(nchildren)
            .map(|chunk| {
                dir_entry::DirEntry::try_from(chunk)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            })
            .collect::<Result<Vec<dir_entry::DirEntry>, _>>()?;

        Ok(dentries)
    }

    pub fn setup_root_dir(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.add_dentry_to_inode(crate::ROOT_INODE, ".", 1)?;
        self.add_dentry_to_inode(crate::ROOT_INODE, "..", 1)?;
        Ok(())
    }
}

// 💨
impl Drop for CfsPartition {
    fn drop(&mut self) {
        self.blk_dev.sync_all().unwrap();
    }
}

impl TryFrom<std::fs::File> for CfsPartition {
    type Error = Box<dyn std::error::Error>;

    fn try_from(mut blk_dev: std::fs::File) -> Result<Self, Self::Error> {
        let mut buffer = Vec::new();
        blk_dev.read_to_end(&mut buffer)?;

        let (_, cfs) = Cfs::from_bytes((buffer.as_ref(), 0))?;

        Ok(CfsPartition { blk_dev, cfs })
    }
}
