use deku::prelude::*;

const BAD_INODE: u32 = 0;
const ROOT_INODE: u32 = 1;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct Inode {
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
    fn new(
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
