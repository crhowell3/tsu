use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

pub fn display_width(s: &str) -> usize {
    s.width()
}

pub fn char_display_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

pub fn char_to_byte(line: &str, char_idx: usize) -> usize {
    line.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(line.len())
}

pub fn byte_to_char(line: &str, byte_offset: usize) -> usize {
    let byte_offset = byte_offset.min(line.len());
    line[..byte_offset].chars().count()
}

pub fn prev_grapheme_boundary(s: &str, byte_offset: usize) -> Option<usize> {
    let graphemes: Vec<(usize, &str)> = s.grapheme_indices(true).collect();

    for i in (0..graphemes.len()).rev() {
        if graphemes[i].0 < byte_offset {
            return Some(graphemes[i].0);
        }
    }

    None
}
