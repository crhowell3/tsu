use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

#[allow(dead_code)]
pub fn display_width(s: &str) -> usize {
    s.width()
}

#[allow(dead_code)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_display_width() {
        let test_str = "test";
        assert_eq!(display_width(test_str), 4);
    }

    #[test]
    fn test_char_display_width() {
        let test_char = 't';
        assert_eq!(char_display_width(test_char), 1);
    }

    #[test]
    fn test_char_display_width_with_emoji() {
        let emoji = '🌊';
        assert_eq!(char_display_width(emoji), 2);
    }

    #[test]
    fn test_char_to_byte() {
        let test_line = "This is a test line";
        let char_idx = 9;
        assert_eq!(char_to_byte(test_line, char_idx), 9);
    }

    #[test]
    fn test_char_to_byte_with_emoji() {
        let test_line = "This is 🌊 a test line";
        let char_idx = 9;
        assert_eq!(char_to_byte(test_line, char_idx), 12);
    }

    #[test]
    fn test_byte_to_char() {
        let test_line = "This is a test line";
        let byte_offset = 9;
        assert_eq!(byte_to_char(test_line, byte_offset), 9);
    }

    #[test]
    fn test_byte_to_char_with_emoji() {
        let test_line = "This is 🌊 a test line";
        let byte_offset = 8;
        assert_eq!(byte_to_char(test_line, byte_offset), 8);
    }

    #[test]
    fn test_prev_grapheme_boundary() {
        let test_line = "Jesus Christ is the son of God";
        let byte_offset = 4;
        assert_eq!(prev_grapheme_boundary(test_line, byte_offset).unwrap(), 3);
    }
}
