use crate::{
    font_data::get_font,
    graphics::{PixelColor, PixelWriter},
};

pub(crate) fn write_ascii(writer: &dyn PixelWriter, x: usize, y: usize, c: u8, color: &PixelColor) {
    let font = get_font(c);
    for dy in 0..16 {
        for dx in 0..8 {
            if ((font[dy] << dx) & 0x80) != 0 {
                writer.write(x + dx, y + dy, color);
            }
        }
    }
}

pub(crate) fn write_string(
    writer: &dyn PixelWriter,
    x: usize,
    y: usize,
    s: &[u8],
    color: &PixelColor,
) {
    for i in 0..s.len() {
        write_ascii(writer, x + 8 * i, y, s[i], color);
    }
}
