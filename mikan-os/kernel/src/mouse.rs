use crate::graphics::{PixelColor, PixelWriter, Vector2D};

/// マウスカーソルの横幅
const MOUSE_CURSOR_WIDTH: usize = 15;
/// マウスカーソルの高さ
const MOUSE_CURSOR_HEIGHT: usize = 24;
/// マウスカーソルの形
const MOUSE_CURSOR_SHAPE: [&[u8; MOUSE_CURSOR_WIDTH]; MOUSE_CURSOR_HEIGHT] = [
    b"@              ",
    b"@@             ",
    b"@.@            ",
    b"@..@           ",
    b"@...@          ",
    b"@....@         ",
    b"@.....@        ",
    b"@......@       ",
    b"@.......@      ",
    b"@........@     ",
    b"@.........@    ",
    b"@..........@   ",
    b"@...........@  ",
    b"@............@ ",
    b"@......@@@@@@@@",
    b"@......@       ",
    b"@....@@.@      ",
    b"@...@ @.@      ",
    b"@..@   @.@     ",
    b"@.@    @.@     ",
    b"@@      @.@    ",
    b"@       @.@    ",
    b"         @.@   ",
    b"         @@@   ",
];

pub(crate) struct MouseCursor<'a> {
    pixel_writer: &'a dyn PixelWriter,
    erase_color: PixelColor,
    position: Vector2D<u32>,
}

impl<'a> MouseCursor<'a> {
    pub(crate) fn new(
        writer: &'a dyn PixelWriter,
        erase_color: PixelColor,
        initial_position: Vector2D<u32>,
    ) -> Self {
        let mut ret = Self {
            pixel_writer: writer,
            erase_color,
            position: initial_position,
        };
        ret.draw_mouse_cursor();
        ret
    }

    pub(crate) fn move_relative(&mut self, displacement: Vector2D<u32>) {
        self.erase_mouse_cursor();
        self.position += displacement;
        self.draw_mouse_cursor();
    }

    fn draw_mouse_cursor(&mut self) {
        for dy in 0..MOUSE_CURSOR_HEIGHT {
            for dx in 0..MOUSE_CURSOR_WIDTH {
                if MOUSE_CURSOR_SHAPE[dy][dx] == b'@' {
                    self.pixel_writer.write(
                        self.position + Vector2D::new(dx as u32, dy as u32),
                        &PixelColor::new(0, 0, 0),
                    );
                } else if MOUSE_CURSOR_SHAPE[dy][dx] == b'.' {
                    self.pixel_writer.write(
                        self.position + Vector2D::new(dx as u32, dy as u32),
                        &PixelColor::new(255, 255, 255),
                    );
                }
            }
        }
    }

    fn erase_mouse_cursor(&mut self) {
        for dy in 0..MOUSE_CURSOR_HEIGHT {
            for dx in 0..MOUSE_CURSOR_WIDTH {
                if MOUSE_CURSOR_SHAPE[dy][dx] != b' ' {
                    self.pixel_writer.write(
                        self.position + Vector2D::new(dx as u32, dy as u32),
                        &self.erase_color,
                    )
                }
            }
        }
    }
}
