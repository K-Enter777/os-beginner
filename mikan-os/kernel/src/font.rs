use crate::{
    font_data::get_font,
    graphics::{PixelColor, PixelWriter, Vector2D},
};

pub(crate) fn write_ascii(
    writer: &mut dyn PixelWriter,
    pos: Vector2D<u32>,
    c: u8,
    color: &PixelColor,
) {
    let font = get_font(c);
    for dy in 0..16 {
        for dx in 0..8 {
            if ((font[dy] << dx) & 0x80) != 0 {
                writer.write(pos + Vector2D::new(dx, dy as u32), color);
            }
        }
    }
}

pub(crate) fn write_string(
    writer: &mut dyn PixelWriter,
    pos: Vector2D<u32>,
    s: &[u8],
    color: &PixelColor,
) {
    for i in 0..s.len() {
        write_ascii(
            writer,
            Vector2D::new(pos.x() + 8 * i as u32, pos.y()),
            s[i],
            color,
        );
    }
}
