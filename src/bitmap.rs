use deku::prelude::*;

// TODO: make this compliant with deku (proly doing the same const slice as in inode.rs) 💃
#[derive(Debug, PartialEq, DekuRead, DekuWrite, Clone)]
pub struct Bitmap {
    pub size: usize,
    #[deku(count = "size")]
    pub data: Vec<u8>,
}

impl Bitmap {
    pub fn new(size: usize) -> Self {
        let mut data = Vec::new();
        data.resize(size, 0);
        Self { data, size }
    }

    pub fn set(&mut self, index: usize) {
        let byte = index / 8;
        let bit = index % 8;
        self.data[byte] |= 1 << bit;
    }

    pub fn clear(&mut self, index: usize) {
        let byte = index / 8;
        let bit = index % 8;
        self.data[byte] &= !(1 << bit);
    }

    pub fn get(&self, index: usize) -> bool {
        let byte = index / 8;
        let bit = index % 8;
        self.data[byte] & (1 << bit) != 0
    }
}
