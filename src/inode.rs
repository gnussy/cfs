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
    blkaddr: [u32; 10],
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
        blkaddr: [u32; 10],
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

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct InodeList {
    size: usize,
    #[deku(count = "size")]
    inodes: Vec<Option<Inode>>,
}

impl InodeList {
pub fn new(size: usize) -> Self {
    let inodes: Vec<Option<Inode>> = std::iter::repeat_with(|| {
        Some(Inode::new(
            0o040_755, // directory, rwxr-xr-x
            2,
            0,
            0,
            0,
            0,
            0,
            0,
            [1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ))
    })
    .take(2)
    .chain(std::iter::repeat(None))
    .take(size)
    .collect();

    Self {
        size: 1024,
        inodes,
    }
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

impl Default for InodeList {
    fn default() -> Self {
        Self::new(0)
    }
}
