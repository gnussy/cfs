#[repr(packed)]
struct SuperBlock {
    magic: u32,
    blocksize: u32,
    bam_blocks: u32,
    iam_blocks: u32,
    inode_blocks: u32,
    nblocks: u32,
    ninodes: u32,
}

impl SuperBlock {
    fn new(
        magic: u32,
        blocksize: u32,
        bam_blocks: u32,
        iam_blocks: u32,
        inode_blocks: u32,
        nblocks: u32,
        ninodes: u32,
    ) -> Self {
        Self {
            magic,
            blocksize,
            bam_blocks,
            iam_blocks,
            inode_blocks,
            nblocks,
            ninodes,
        }
    }
}
