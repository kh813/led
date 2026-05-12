use crossterm::{
    cursor,
    style::{self, Color, ContentStyle},
    QueueableCommand,
};
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub underline: bool,
    pub width: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            bold: false,
            underline: false,
            width: 1,
        }
    }
}

pub struct Renderer {
    width: u16,
    height: u16,
    prev_buffer: Vec<Cell>,
    curr_buffer: Vec<Cell>,
}

impl Renderer {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        Self {
            width,
            height,
            prev_buffer: vec![Cell::default(); size],
            curr_buffer: vec![Cell::default(); size],
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let size = (width as usize) * (height as usize);
        self.prev_buffer = vec![Cell::default(); size];
        self.curr_buffer = vec![Cell::default(); size];
    }

    pub fn set_cell(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.curr_buffer[idx] = cell;
        }
    }

    pub fn clear(&mut self) {
        for cell in self.curr_buffer.iter_mut() {
            *cell = Cell::default();
        }
    }

    pub fn present<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        let mut last_style = ContentStyle::default();
        let mut cursor_x = 0u16;
        let mut cursor_y = 0u16;
        let mut force_move = true;

        for y in 0..self.height {
            let mut x = 0;
            while x < self.width {
                let idx = (y as usize) * (self.width as usize) + (x as usize);
                let curr = self.curr_buffer[idx];
                let prev = self.prev_buffer[idx];

                if curr != prev || force_move {
                    if force_move || cursor_x != x || cursor_y != y {
                        writer.queue(cursor::MoveTo(x, y))?;
                        cursor_x = x;
                        cursor_y = y;
                    }

                    let mut style = ContentStyle::default();
                    style.foreground_color = Some(curr.fg);
                    style.background_color = Some(curr.bg);
                    
                    if curr.bold {
                        style.attributes.set(style::Attribute::Bold);
                    }
                    if curr.underline {
                        style.attributes.set(style::Attribute::Underlined);
                    }

                    if style != last_style {
                        writer.queue(style::SetStyle(style))?;
                        last_style = style;
                    }

                    writer.queue(style::Print(curr.ch))?;
                    cursor_x += curr.width as u16;
                    force_move = false;
                }
                x += curr.width as u16;
            }
            force_move = true; // Force move at start of next line
        }

        self.prev_buffer.copy_from_slice(&self.curr_buffer);
        writer.flush()?;
        Ok(())
    }
}
