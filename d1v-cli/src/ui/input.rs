use crossterm::event::{KeyCode, KeyEvent};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Grapheme-based single-line text input state.
pub struct InputState {
    content: String,
    offset: usize,
    cursor: Option<GraphemeCursor>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            offset: 0,
            cursor: None,
        }
    }

    /// Returns the number of grapheme clusters.
    pub fn grapheme_count(&self) -> usize {
        self.content.graphemes(true).count()
    }

    /// Returns the display width (in terminal columns) of text before the cursor.
    pub fn display_width(&self) -> usize {
        self.content[..self.offset].width()
    }

    /// Returns a masked representation where each grapheme is replaced by `mask`.
    pub fn masked(&self, mask: char) -> String {
        self.content.graphemes(true).map(|_| mask).collect()
    }

    /// Returns the display width of the masked text before the cursor.
    pub fn masked_display_width(&self, mask: char) -> usize {
        let count = self.content[..self.offset].graphemes(true).count();
        count * mask.width().unwrap_or(1)
    }

    fn prev_boundary(&mut self) -> usize {
        let cursor = self
            .cursor
            .get_or_insert_with(|| GraphemeCursor::new(self.offset, self.content.len(), true));
        cursor.set_cursor(self.offset);

        if let Ok(Some(pos)) = cursor.prev_boundary(&self.content, 0) {
            pos
        } else {
            0
        }
    }

    fn next_boundary(&mut self) -> usize {
        let cursor = self
            .cursor
            .get_or_insert_with(|| GraphemeCursor::new(self.offset, self.content.len(), true));
        cursor.set_cursor(self.offset);

        if let Ok(Some(pos)) = cursor.next_boundary(&self.content, 0) {
            pos
        } else {
            self.content.len()
        }
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Clears all content and resets the cursor.
    pub fn clear(&mut self) {
        self.content.clear();
        self.offset = 0;
        self.cursor = None;
    }

    /// Inserts a character at the cursor position.
    pub fn insert(&mut self, ch: char) {
        self.content.insert(self.offset, ch);
        self.offset += ch.len_utf8();
        self.cursor = None;
    }

    /// Deletes the grapheme cluster before the cursor (Backspace).
    pub fn delete_prev(&mut self) -> bool {
        if self.offset == 0 {
            return false;
        }

        let prev = self.prev_boundary();
        self.content.drain(prev..self.offset);
        self.offset = prev;
        self.cursor = None;

        true
    }

    /// Deletes the grapheme cluster at the cursor (Delete).
    pub fn delete_next(&mut self) -> bool {
        if self.offset >= self.content.len() {
            return false;
        }

        let next = self.next_boundary();
        self.content.drain(self.offset..next);
        self.cursor = None;

        true
    }

    /// Moves the cursor left by one grapheme cluster.
    pub fn move_left(&mut self) {
        if self.offset == 0 {
            return;
        }

        self.offset = self.prev_boundary();
    }

    /// Moves the cursor right by one grapheme cluster.
    pub fn move_right(&mut self) {
        if self.offset >= self.content.len() {
            return;
        }

        self.offset = self.next_boundary();
    }

    /// Moves the cursor to the beginning.
    pub fn move_start(&mut self) {
        self.offset = 0;
    }

    /// Moves the cursor to the end.
    pub fn move_end(&mut self) {
        self.offset = self.content.len();
    }

    /// Handles editing key events (characters, backspace, delete, navigation).
    pub fn handle_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Char(c) => self.insert(c),
            KeyCode::Backspace => {
                self.delete_prev();
            }
            KeyCode::Delete => {
                self.delete_next();
            }
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Home => self.move_start(),
            KeyCode::End => self.move_end(),
            _ => {}
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl From<InputState> for String {
    fn from(state: InputState) -> Self {
        state.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_delete() {
        let mut s = InputState::new();
        s.insert('h');
        s.insert('i');

        assert_eq!(s.text(), "hi");
        assert_eq!(s.display_width(), 2);

        s.delete_prev();
        assert_eq!(s.text(), "h");

        s.delete_next();
        assert_eq!(s.text(), "h");
    }

    #[test]
    fn insert_middle() {
        let mut s = InputState::new();
        s.insert('a');
        s.insert('c');
        s.move_left();
        s.insert('b');

        assert_eq!(s.text(), "abc");
        assert_eq!(s.display_width(), 2);
    }

    #[test]
    fn cjk_width() {
        let mut s = InputState::new();
        s.insert('你');
        s.insert('好');

        assert_eq!(s.display_width(), 4);
        assert_eq!(s.grapheme_count(), 2);
    }

    #[test]
    fn cjk_navigation() {
        let mut s = InputState::new();
        for ch in ['你', '好', '世', '界'] {
            s.insert(ch);
        }

        s.move_left();
        s.move_left();
        assert_eq!(s.display_width(), 4);

        s.delete_next();
        assert_eq!(s.text(), "你好界");

        s.delete_prev();
        assert_eq!(s.text(), "你界");
    }

    #[test]
    fn move_boundaries() {
        let mut s = InputState::new();
        for ch in ['a', 'b', 'c'] {
            s.insert(ch);
        }

        s.move_start();
        assert_eq!(s.display_width(), 0);

        s.move_end();
        assert_eq!(s.display_width(), 3);
    }

    #[test]
    fn masked() {
        let mut s = InputState::new();
        s.insert('密');
        s.insert('码');

        assert_eq!(s.masked('*'), "**");
        assert_eq!(s.masked_display_width('*'), 2);

        assert_eq!(s.masked('●'), "●●");
        assert_eq!(s.masked_display_width('●'), 2);
    }

    #[test]
    fn empty() {
        let mut s = InputState::new();

        assert!(s.is_empty());
        assert!(!s.delete_prev());
        assert!(!s.delete_next());

        s.move_left();
        s.move_right();
        s.move_start();
        s.move_end();

        assert!(s.is_empty());
    }

    #[test]
    fn clear() {
        let mut s = InputState::new();
        s.insert('x');
        s.insert('y');
        s.clear();

        assert!(s.is_empty());
        assert_eq!(s.display_width(), 0);
    }

    #[test]
    fn into_string() {
        let mut s = InputState::new();
        s.insert('o');
        s.insert('k');

        let result: String = s.into();
        assert_eq!(result, "ok");
    }

    #[test]
    fn combining_mark() {
        let mut s = InputState::new();
        // é
        s.insert('e');
        s.insert('\u{0301}');

        assert_eq!(s.text(), "e\u{0301}");
        assert_eq!(s.grapheme_count(), 1);
        assert_eq!(s.display_width(), 1);
    }
}
