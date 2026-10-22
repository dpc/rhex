use std::cell::{Cell, RefCell};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::color;

/// A clipped, buffered region of the existing game layout.
pub struct Window {
    /// Screen coordinates and dimensions of this region.
    area: Rect,
    /// Cells composed into the terminal's next frame.
    buffer: RefCell<Buffer>,
    /// Local cursor used by the text panels.
    cursor: Cell<(i32, i32)>,
    /// The last glyph already advanced the cursor across the right edge.
    wrapped: Cell<bool>,
    /// Style applied to subsequent text.
    style: Cell<Style>,
}

#[cfg(test)]
#[path = "window_tests.rs"]
mod tests;

impl Window {
    /// Create a region with nonnegative dimensions.
    pub fn new(w: i32, h: i32, x: i32, y: i32) -> Self {
        let area = Rect::new(
            x.max(0) as u16,
            y.max(0) as u16,
            w.max(0) as u16,
            h.max(0) as u16,
        );
        Self {
            area,
            buffer: RefCell::new(Buffer::empty(area)),
            cursor: Cell::new((0, 0)),
            wrapped: Cell::new(false),
            style: Cell::new(color::style(color::VISIBLE_FG, color::BACKGROUND_BG)),
        }
    }

    /// Return the height and width in terminal cells.
    pub fn size(&self) -> (i32, i32) {
        (self.area.height.into(), self.area.width.into())
    }

    /// Clear the region and reset its cursor and text style.
    pub fn clear(&self) {
        let style = color::style(color::VISIBLE_FG, color::BACKGROUND_BG);
        let mut buffer = self.buffer.borrow_mut();
        buffer.reset();
        buffer.set_style(self.area, style);
        self.cursor.set((0, 0));
        self.wrapped.set(false);
        self.style.set(style);
    }

    /// Move the text cursor within the region; out-of-range writes are clipped.
    pub fn move_to(&self, y: i32, x: i32) {
        self.cursor.set((x, y));
        self.wrapped.set(false);
    }

    /// Select the style for subsequent text.
    pub fn set_style(&self, style: Style) {
        self.style.set(style);
    }

    /// Return the current local cursor row.
    pub fn cursor_y(&self) -> i32 {
        self.cursor.get().1
    }

    /// Write text with newline handling and wrapping at the region boundary.
    pub fn write(&self, text: &str) {
        for ch in text.chars() {
            self.write_char(ch);
        }
    }

    /// Write a single character using the current text style.
    pub fn write_char(&self, ch: char) {
        let (mut x, mut y) = self.cursor.get();
        let (height, width) = self.size();
        let wrapped = self.wrapped.replace(false);
        if ch == '\n' {
            // An explicit newline terminates the full line, not an extra empty one.
            self.cursor.set((0, y + i32::from(!wrapped)));
            return;
        }
        if x < 0 || y < 0 || x >= width || y >= height {
            return;
        }
        let mut bytes = [0; 4];
        let (next_x, _) = self.buffer.borrow_mut().set_stringn(
            self.area.x + x as u16,
            self.area.y + y as u16,
            ch.encode_utf8(&mut bytes),
            (width - x) as usize,
            self.style.get(),
        );
        x = i32::from(next_x - self.area.x);
        if x >= width {
            x = 0;
            y += 1;
            self.wrapped.set(true);
        }
        self.cursor.set((x, y));
    }

    /// Draw a map glyph without changing the text cursor.
    pub fn put(&self, y: i32, x: i32, ch: char, style: Style) {
        let (height, width) = self.size();
        if x < 0 || y < 0 || x >= width || y >= height {
            return;
        }
        self.buffer.borrow_mut()[(self.area.x + x as u16, self.area.y + y as u16)]
            .set_char(ch)
            .set_style(style);
    }

    /// Copy visible cells into the final frame, clipping against its current size.
    pub fn render(&self, target: &mut Buffer) {
        let area = self.area.intersection(target.area);
        let buffer = self.buffer.borrow();
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                target[(x, y)] = buffer[(x, y)].clone();
            }
        }
    }
}
