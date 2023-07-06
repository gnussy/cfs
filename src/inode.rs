use crate::superblock;
use deku::prelude::*;

pub const BAD_INODE: u32 = 0;
pub const ROOT_INODE: u32 = 1;

#[derive(Debug, Copy, Clone, PartialEq, DekuRead, DekuWrite)]
pub struct Inode {
    pub mode: u16,
    pub nchildren: u16,
    pub uid: u16,
    pub gid: u16,
    pub size: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub blkaddr: [u32; 10],
}

impl Inode {
    pub(crate) fn new(
        mode: u16,
        nchildren: u16,
        uid: u16,
        gid: u16,
        size: u32,
        atime: u32,
        mtime: u32,
        ctime: u32,
        blkaddr: [u32; 10],
    ) -> Self {
        Self {
            mode,
            nchildren,
            uid,
            gid,
            size,
            atime,
            mtime,
            ctime,
            blkaddr,
        }
    }

    #[inline(always)]
    pub fn inodes_per_block(&self, block_size: u64) -> u64 {
        block_size / std::mem::size_of::<Self>() as u64
    }
}

impl Default for Inode {
    fn default() -> Self {
        Self::new(0, 0, 0, 0, 0, 0, 0, 0, [0; 10])
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[deku(ctx = "super_block: superblock::SuperBlock")]
pub struct InodeList {
    #[deku(count = "super_block.ninodes")]
    inodes: Vec<Inode>,
}

impl InodeList {
    pub fn new() -> Self {
        let root_inode = Inode::new(
            0o040_755, // directory, rwxr-xr-x
            0,         // We'll add root dentries later
            0,
            0,
            0,
            0,
            0,
            0,
            [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );

        // This is a hack to get the root inode in the right place, so we don't
        // have to do arithmetic when indexing the inode list.
        let inodes = vec![Inode::default(), root_inode];

        Self { inodes }
    }

    pub fn get(&self, index: usize) -> Inode {
        self.inodes[index]
    }

    pub fn set(&mut self, index: usize, inode: Inode) {
        self.inodes[index] = inode;
    }

    pub fn clear(&mut self, index: usize) {
        self.inodes[index] = Inode::default();
    }
}

impl Default for InodeList {
    fn default() -> Self {
        Self::new()
    }
}
