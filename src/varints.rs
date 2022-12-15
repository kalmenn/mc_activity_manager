use bitvec::prelude::*;

pub fn into_varint(number: u64) -> Vec<u8> {
    let bits = number.view_bits::<Lsb0>();

    let mut bytes: Vec<u8> = vec![0; (bits.len() + 6) / 7];

    // Fill bytes with groups of 7 bits form the bits BitSlice
    for bit in bits.iter().by_vals().enumerate() {
        bytes[(bit.0) / 7] += 2_u8.pow((bit.0 % 7) as u32) * match bit.1 {
            true => 1,
            false => 0
        };
    }

    // Remove trailing null bytes
    bytes = bytes.into_iter()
    .enumerate()
    .take_while(|byte| byte.1 > 0 || byte.0 == 0)
    .map(|byte| byte.1)
    .collect();

    bytes.reverse();

    // Add continuation bits
    for i in 0..bytes.len()-1 {
        bytes[i] += 128;
    }

    return bytes;
}