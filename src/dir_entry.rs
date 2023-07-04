use deku::prelude::*;

const MAX_NAME_LEN: usize = 60;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct DirEntry {
    name: [u8; MAX_NAME_LEN],
    inode: u32,
}

impl DirEntry {
    fn new(name: [u8; MAX_NAME_LEN], inode: u32) -> Self {
        Self { name, inode }
    }
}
