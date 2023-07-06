use deku::prelude::*;

// I've broken my rules of no Clones... 🕺
#[derive(Debug, PartialEq, DekuRead, DekuWrite, Copy, Clone)]
pub struct SuperBlock {
    pub magic: u32,
    pub blocksize: u32,
    pub bam_blocks: u32,
    pub iam_blocks: u32,
    pub inode_blocks: u32,
    pub nblocks: u32,
    pub ninodes: u32,
    pub padding: [u8; 484],
}

impl SuperBlock {
    pub fn new(
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
            padding: [0; 484],
        }
    }
}
