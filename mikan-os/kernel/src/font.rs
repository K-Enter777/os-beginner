use crate::graphics::{PixelColor, PixelWriter};

const K_FONT_A: [u8; 16] = [
    0b00000000, //
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00011000, //    **
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b00100100, //   *  *
    0b01111110, //  ******
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b01000010, //  *    *
    0b11100111, // ***  ***
    0b00000000, //
    0b00000000, //
];

pub(crate) fn write_ascii(writer: &dyn PixelWriter, x: usize, y: usize, c: u8, color: &PixelColor) {
    if c != b'A' {
        return;
    }
    for dy in 0..16 {
        for dx in 0..8 {
            if ((K_FONT_A[dy] << dx) & 0x80) != 0 {
                writer.write(x + dx, y + dy, color);
            }
        }
    }
}
