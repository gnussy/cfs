const BAD_INODE: u32 = 0;
const ROOT_INODE: u32 = 1;

#[repr(packed)]
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
}
