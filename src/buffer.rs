use std::path::Path;

use ropey::Rope;

use crate::unicode::column_to_char;

#[derive(Debug)]
/// The internal representation of an opened file
///
/// This entity contains the contents of a file, whether that file exists or is to be written, and
/// maintains state information about the file, such as whether it is "dirty" (changes have been
/// made that have not been saved yet), etc.
pub struct Buffer {
    /// The path to the file; optional because we can open empty buffers that can be written to files
    /// using `Action::SaveAs`
    pub file: Option<String>,
    /// The contents of the file stored as a Rope which is a fast (worst case O(log N)) data structure
    /// for editing utf8 text
    content: Rope,
    /// State flag for keeping track of unsaved (dirty) changes
    pub dirty: bool,
    pub position: (usize, usize),
    pub vtop: usize,
}

impl Buffer {
    #[must_use]
    /// Create a new buffer using a file path and some file contents
    ///
    /// # Arguments
    /// - `file`: Optional file path
    /// - `contents`: String representation of the contents of the associated file
    ///
    /// # Examples
    ///
    /// ```
    /// use tsu::buffer::Buffer;
    ///
    /// let file = Some("some_file.rs".to_string());
    /// let contents = "";
    ///
    /// let buffer = Buffer::new(file.clone(), contents.to_string());
    ///
    /// assert_eq!(buffer.file, file);
    /// ```
    pub fn new(file: Option<String>, contents: String) -> Self {
        let contents = if contents.is_empty() {
            "\n".to_string()
        } else {
            contents
        };

        Self {
            file,
            content: Rope::from_str(&contents),
            dirty: false,
            position: (0, 0),
            vtop: 0,
        }
    }

    #[allow(clippy::unused_async)]
    /// Create a new buffer by opening a file and retrieving its contents
    ///
    /// # Arguments
    /// - `file`: An optional file path
    ///
    /// # Examples
    ///
    /// ```
    /// use tsu::buffer::Buffer;
    ///
    /// # use std::fs::{self, File};
    /// # use std::io::Write;
    /// # use std::env;
    ///
    /// # tokio_test::block_on(async {
    /// # let dir = env::temp_dir();
    /// # let file_path = dir.join("some_file.rs");
    /// # let mut file = File::create(&file_path).unwrap();
    /// let file_path_str = Some(file_path.clone().into_os_string().into_string().unwrap());
    /// let buffer = Buffer::from_file(file_path_str.clone()).await.unwrap();
    /// assert_eq!(buffer.file, file_path_str);
    ///
    /// # std::fs::remove_file(file_path).unwrap();
    /// # })
    /// ```
    ///
    /// # Errors
    /// Can return an `IoError` if the file cannot be read from
    pub async fn from_file(file: Option<String>) -> anyhow::Result<Self> {
        match &file {
            Some(file) => {
                let path = Path::new(file);
                if !path.exists() {
                    return Err(anyhow::anyhow!("file {file} not found"));
                }

                let contents = std::fs::read_to_string(file)?;

                if contents
                    .chars()
                    .any(|c| c as u32 >= 0x1F300 && c as u32 <= 0x1F9FF)
                {
                    // NOOP
                }

                Ok(Self::new(Some(file.to_string()), contents))
            }
            None => Ok(Self::new(file, "\n".to_string())),
        }
    }

    #[must_use]
    /// Returns the entirety of the content as a String
    ///
    /// # Returns
    /// - The contents of self as a String
    pub fn contents(&self) -> String {
        self.content.to_string()
    }

    /// Writes the contents of self to `file`
    ///
    /// # Returns
    /// - A message to print to the command line of the editor
    ///
    /// # Errors
    /// Can return an `IoError` if it fails to write the contents to the provided file
    pub fn save(&mut self) -> anyhow::Result<String> {
        if let Some(file) = &self.file {
            let contents = self.contents();
            std::fs::write(file, &contents)?;
            self.dirty = false;
            let message = format!("{:?} {}L, {}B written", file, self.len(), contents.len());
            Ok(message)
        } else {
            Err(anyhow::anyhow!("No file name"))
        }
    }

    /// Writes the contents of self to a new file
    ///
    /// # Arguments
    /// - `new_file_name`: The path to the file to be written
    ///
    /// # Returns
    /// - A message to print to the command line of the editor
    ///
    /// # Errors
    /// Can return an `IoError` if it fails to write the contents to the provided file
    pub fn save_as(&mut self, new_file_name: &str) -> anyhow::Result<String> {
        let contents = self.contents();
        std::fs::write(new_file_name, &contents)?;
        self.dirty = false;
        self.file = Some(new_file_name.to_string());
        let message = format!(
            "{:?} {}L, {}B written",
            new_file_name,
            self.len(),
            contents.len()
        );
        Ok(message)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.file.as_deref().unwrap_or("[No name]")
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[must_use]
    /// Retrieves the contents at a specific line in the buffer
    ///
    /// # Arguments
    /// - `line`: The line number
    ///
    /// # Returns
    /// - The contents of the line if the line exists, i.e., is within the bounds of the buffer
    pub fn get(&self, line: usize) -> Option<String> {
        if line > self.len() {
            return None;
        }

        Some(self.content.line(line).to_string())
    }

    #[must_use]
    /// Get the length of the buffer, i.e., the number of lines
    ///
    /// # Returns
    /// - The size of the buffer expressed as the number of lines
    pub fn len(&self) -> usize {
        self.content.len_lines() - 1
    }

    #[must_use]
    /// Check if the buffer is empty, i.e., the number of lines is 0
    ///
    /// # Returns
    /// - True if the buffer is empty, false otherwise
    pub fn is_empty(&self) -> bool {
        self.content.len_bytes() == 0
    }

    /// Put a character at a coordinate position within the buffer
    ///
    /// # Arguments
    /// - `x`: The coordinate along the x axis (column)
    /// - `y`: The coordinate along the y axis (row)
    /// - `c`: Character to insert at the specified coordinates
    pub fn insert(&mut self, x: usize, y: usize, c: char) {
        let char_idx = self.position_to_char_idx(x, y);
        let total_chars = self.content.len_chars();

        crate::debug!(
            "Buffer::insert - x: {x}, y: {y}, char: '{c}', char_idx: {char_idx}, total_chars: {total_chars}"
        );

        if char_idx > total_chars {
            crate::error!(
                "char_idx {char_idx} exceeds total_chars {total_chars}! Clamping to end."
            );
            self.content.insert_char(total_chars, c);
        } else {
            self.content.insert_char(char_idx, c);
        }
        self.dirty = true;
    }

    /// Insert a line at the specified line index (row)
    ///
    /// # Arguments
    /// - `line`: The line index where the contents will be inserted
    /// - `content`: The string to insert at the specified row
    pub fn insert_line(&mut self, line: usize, content: String) {
        let char_idx = if line >= self.content.len_lines() {
            self.content.len_chars()
        } else {
            self.content.line_to_char(line)
        };
        self.content.insert(char_idx, &format!("{content}\n"));
        self.dirty = true;
    }

    /// Remove a character at the specified coordinates
    ///
    /// # Arguments
    /// - `x`: The coordinate along the x axis (column)
    /// - `y`: The coordinate along the y axis (row)
    pub fn remove(&mut self, x: usize, y: usize) {
        let char_idx = self.position_to_char_idx(x, y);
        if char_idx < self.content.len_chars() {
            self.content.remove(char_idx..=char_idx);
            self.dirty = true;
        }
    }

    /// Replaces the line at a given line index with a new String
    ///
    /// # Arguments
    /// - `line`: The index of the line being replaced
    /// - `new_line`: The String to be written to the line index
    pub fn replace_line(&mut self, line: usize, new_line: String) {
        if line >= self.len() {
            return;
        }

        let start_char = self.content.line_to_char(line);
        let end_char = if line + 1 < self.len() {
            self.content.line_to_char(line + 1)
        } else {
            self.content.len_chars()
        };

        self.content.remove(start_char..end_char);
        self.content.insert(start_char, &format!("{new_line}\n"));
        self.dirty = true;
    }

    /// Removes a line at the given line index
    ///
    /// # Arguments
    /// - `line`: The index of the line being removed
    pub fn remove_line(&mut self, line: usize) {
        if line >= self.content.len_lines() {
            return;
        }

        let start_char = self.content.line_to_char(line);
        let end_char = if line + 1 < self.content.len_lines() {
            self.content.line_to_char(line + 1)
        } else {
            self.content.len_chars()
        };

        self.content.remove(start_char..end_char);
        self.dirty = true;
    }

    #[must_use]
    /// Calculates the view inside the buffer using a specified view top and view height
    ///
    /// # Arguments
    /// - `vtop`: The top of the view corresponding to a line index
    /// - `vheight`: The height of the view corresponding to the number of lines to display
    ///
    /// # Returns
    /// - The contents within the calculated view as a String
    pub fn view(&self, vtop: usize, vheight: usize) -> String {
        let height = std::cmp::min(vtop + vheight, self.content.len_lines());
        let mut result = String::new();
        for i in vtop..height {
            result.push_str(&self.content.line(i).to_string());
        }
        result
    }

    /// Converts a coordinate pair to a character index within the `contents` Rope
    ///
    /// # Arguments
    /// - `x`: The coordinate along the x axis (column)
    /// - `y`: The coordinate along the y axis (row)
    ///
    /// # Returns
    /// - The calculated index of the character within the Rope
    fn position_to_char_idx(&self, x: usize, y: usize) -> usize {
        if y >= self.content.len_lines() {
            return self.content.len_chars();
        }

        let line_start_char = self.content.line_to_char(y);

        let line = self.content.line(y);
        let line_chars = line.len_chars();

        let line_chars_no_newline = if line_chars > 0 && line.char(line_chars - 1) == '\n' {
            line_chars - 1
        } else {
            line_chars
        };

        let x = x.min(line_chars_no_newline);

        line_start_char + x
    }

    pub fn column_to_char_index(&self, column: usize, y: usize) -> usize {
        if let Some(line) = self.get(y) {
            let line = line.trim_end_matches('\n');
            column_to_char(line, column)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_view() {
        let buffer = Buffer::new(
            Some("example".to_string()),
            "this\nis\na\ntest\nusing\nmultiple\nlines".to_string(),
        );

        assert_eq!(buffer.view(0, 2), "this\nis\n");
    }

    #[test]
    fn test_view_with_small_buffer() {
        let buffer = Buffer::new(Some("example".to_string()), "a\ntest".to_string());
        assert_eq!(buffer.view(0, 5), "a\ntest");
    }
}
