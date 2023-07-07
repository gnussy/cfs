// 💋
#[inline(always)]
pub fn bits_per_block(block_size: u64) -> u64 {
    block_size * 8
}

pub fn str_to_u8_60(s: &str) -> [u8; 60] {
    let mut res = [0; 60];
    res.iter_mut()
        .zip(s.bytes())
        .for_each(|(a_byte, s_byte)| *a_byte = s_byte);
    res
}
