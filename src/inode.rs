use deku::prelude::*;

const BAD_INODE: u32 = 0;
const ROOT_INODE: u32 = 1;

#[derive(Debug, Copy, Clone, PartialEq, DekuRead, DekuWrite)]
pub struct Inode {
    mode: u16,
    nlinks: u16,
    uid: u16,
    gid: u16,
    size: u32,
    atime: u32,
    mtime: u32,
    ctime: u32,
    blkaddr: [u32; 9],
}

impl Inode {
    pub(crate) fn new(
        mode: u16,
        nlinks: u16,
        uid: u16,
        gid: u16,
        size: u32,
        atime: u32,
        mtime: u32,
        ctime: u32,
        blkaddr: [u32; 9],
    ) -> Self {
        Self {
            mode,
            nlinks,
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
    fn inodes_per_block(&self, block_size: u64) -> u64 {
        block_size / std::mem::size_of::<Self>() as u64
    }
}

type InodeEntry = Option<Inode>;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct InodeList<const N: usize> {
    inodes: [InodeEntry; N],
}

impl<const N: usize> InodeList<N> {
    pub fn new() -> Self {
        let inodes = std::array::from_fn(|i| match i {
            0 | 1 => Some(Inode::new(
                0o040_755, // directory, rwxr-xr-x
                2,
                0,
                0,
                0,
                0,
                0,
                0,
                [1, 0, 0, 0, 0, 0, 0, 0, 0],
            )),
            _ => None,
        });

        Self { inodes }
    }

    pub fn get(&self, index: usize) -> Option<Inode> {
        self.inodes[index]
    }

    pub fn set(&mut self, index: usize, inode: Inode) {
        self.inodes[index] = Some(inode);
    }

    pub fn clear(&mut self, index: usize) {
        self.inodes[index] = None;
    }
}

impl<const N: usize> Default for InodeList<N> {
    fn default() -> Self {
        Self::new()
    }
}
