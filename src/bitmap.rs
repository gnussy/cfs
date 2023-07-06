use crate::superblock;
use deku::prelude::*;

pub trait Bitmap {
    fn get_data(&mut self) -> &mut [u8];

    fn set(&mut self, index: usize) {
        let byte = index / 8;
        let bit = index % 8;
        let data = self.get_data();
        data[byte] |= 1 << bit;
    }

    fn clear(&mut self, index: usize) {
        let byte = index / 8;
        let bit = index % 8;
        let data = self.get_data();
        data[byte] &= !(1 << bit);
    }

    fn get(&mut self, index: usize) -> bool {
        let byte = index / 8;
        let bit = index % 8;
        let data = self.get_data();
        data[byte] & (1 << bit) != 0
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[deku(ctx = "super_block: superblock::SuperBlock")]
pub struct Bam {
    #[deku(count = "super_block.bam_blocks * super_block.blocksize")]
    pub data: Vec<u8>,
}

impl Bam {
    pub fn new(size: usize) -> Self {
        let mut data = Vec::new();
        data.resize(size, 0);
        Self { data }
    }
}

impl Bitmap for Bam {
    fn get_data(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
#[deku(ctx = "super_block: superblock::SuperBlock")]
pub struct Iam {
    #[deku(count = "super_block.bam_blocks * super_block.blocksize")]
    pub data: Vec<u8>,
}

impl Iam {
    pub fn new(size: usize) -> Self {
        let mut data = Vec::new();
        data.resize(size, 0);
        Self { data }
    }
}

impl Bitmap for Iam {
    fn get_data(&mut self) -> &mut [u8] {
        &mut self.data
    }
}
