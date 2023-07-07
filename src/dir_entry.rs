use deku::prelude::*;

const MAX_NAME_LEN: usize = 60;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct DirEntry {
    pub name: [u8; MAX_NAME_LEN],
    pub inode: u32,
}

impl DirEntry {
    pub fn new(name: [u8; MAX_NAME_LEN], inode: u32) -> Self {
        Self { name, inode }
    }
}
