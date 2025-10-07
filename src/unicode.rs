use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

#[allow(dead_code)]
#[must_use]
/// Calculates the width of the string when displayed in the terminal
///
/// # Arguments
/// - `s`: Some string of which the display width will be calculated
///
/// # Returns
/// - The width of the input string when it is displayed in the terminal
pub fn display_width(s: &str) -> usize {
    s.width()
}

#[allow(dead_code)]
#[must_use]
/// Calculates the width of the char when displayed in the terminal
///
/// # Arguments
/// - `c`: Some character of which the diplay width will be calculated
///
/// # Returns
/// - The width of the input character when it is displayed in the terminal
pub fn char_display_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

#[must_use]
/// Convert a character to a byte representation
///
/// # Arguments
/// -`line`: A line from which the character will be retrieved
/// - `char_idx`: The index of the character relative to its position within the provided line
///
/// # Returns
/// - The byte representation of the retrieved character if the given index is within the bounds of
///   the provided line (i.e., less than the line length)
pub fn char_to_byte(line: &str, char_idx: usize) -> usize {
    line.char_indices()
        .nth(char_idx)
        .map_or(line.len(), |(idx, _)| idx)
}

#[must_use]
/// Convert a byte to a character representation
///
/// # Arguments
/// - `line`: A line from with the byte will be retrieved
/// - `byte_offset`: The byte offset of the byte relative to its position within the provided line
///
/// # Returns
/// - The character representation of the retrieved byte if the given byte offset is within the
///   bounds of the provided line (i.e., less than the line length)
pub fn byte_to_char(line: &str, byte_offset: usize) -> usize {
    let byte_offset = byte_offset.min(line.len());
    line[..byte_offset].chars().count()
}

#[must_use]
/// Query the byte boundary of the previous grapheme in a given string
///
/// # Arguments
/// - `s`: A string containing the grapheme in question
/// - `byte_offset`: The byte offset of the grapheme succeeding the grapheme of interest
///
/// # Returns
/// - The byte boundary of the grapheme preceding the grapheme corresponding to the byte offset
///   provided if the byte offset is within the bounds of the string, i.e., less than the length of
///   the string
pub fn prev_grapheme_boundary(s: &str, byte_offset: usize) -> Option<usize> {
    let graphemes: Vec<(usize, &str)> = s.grapheme_indices(true).collect();

    for i in (0..graphemes.len()).rev() {
        if graphemes[i].0 < byte_offset {
            return Some(graphemes[i].0);
        }
    }

    None
}

#[must_use]
/// Convert a character index to a column index within the view
///
/// # Arguments
/// - `line`: The string of the line containing the character in question
/// - `char_idx`: The index of the character in the provided line
///
/// # Returns
/// - The column index of the character based on its index within the line
pub fn char_to_column(line: &str, char_idx: usize) -> usize {
    line.chars().take(char_idx).map(char_display_width).sum()
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
