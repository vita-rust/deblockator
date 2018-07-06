#![allow(dead_code)]

/// Align downwards.
///
/// Returns the greatest x with alignment `align` so that x <= addr.
/// The alignment must be a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    if align.is_power_of_two() {
        addr & !(align - 1)
    } else if align == 0 {
        addr
    } else {
        panic!("`align` must be a power of 2");
    }
}

/// Align upwards.
///
/// Returns the smallest x with alignment `align` so that x >= addr.
/// The alignment must be a power of 2.
pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr + align - 1, align)
}

/// Write a number as an hexadecimal formatter bytestring
///
/// Panics if the buffer is shorter than `size_of::<usize>()`.
pub fn write_hex(number: usize, buf: &mut [u8]) {
    let length = ::core::mem::size_of::<usize>() / 4;
    for i in 0..length {
        buf[buf.len() - (i + 2)] = match (number & 0xF) as u8 {
            x @ 0x0u8...0x9u8 => x as u8 + b'0',
            y @ 0xAu8...0xFu8 => y as u8 + b'A',
            _ => unreachable!(),
        };
    }
}
