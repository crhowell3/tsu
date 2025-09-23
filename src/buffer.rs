use ropey::Rope;

#[derive(Debug)]
pub struct Buffer {
    pub file: Option<String>,
    content: Rope,
}

impl Buffer {
    pub fn new(file: Option<String>, contents: String) -> Self {
        let contents = if contents.is_empty() {
            "\n".to_string()
        } else {
            contents
        };

        Self {
            file,
            content: Rope::from_str(&contents),
        }
    }

    pub fn from_file(file: Option<String>) -> Self {
        match &file {
            Some(file) => {
                let contents = std::fs::read_to_string(file).unwrap();
                Self::new(Some(file.to_string()), contents.to_string())
            }
            None => Self::new(file, String::new()),
        }
    }

    /// Returns the entirety of the buffer's content as a String
    pub fn contents(&self) -> String {
        self.content.to_string()
    }

    pub fn save(&self) -> anyhow::Result<String> {
        if let Some(file) = &self.file {
            let contents = self.contents();
            std::fs::write(file, &contents)?;
            let message = format!("{:?} {}L, {}B written", file, self.len(), contents.len());
            Ok(message)
        } else {
            Err(anyhow::anyhow!("No file name"))
        }
    }

    pub fn get(&self, line: usize) -> Option<String> {
        if line > self.len() {
            return None;
        }

        Some(self.content.line(line).to_string())
    }

    pub fn len(&self) -> usize {
        self.content.len_lines() - 1
    }

    pub fn insert(&mut self, x: usize, y: usize, c: char) {
        let char_idx = self.position_to_char_idx(x, y);
        let total_chars = self.content.len_chars();

        if char_idx > total_chars {
            self.content.insert_char(total_chars, c);
        } else {
            self.content.insert_char(char_idx, c);
        }
    }

    pub fn insert_line(&mut self, line: usize, content: String) {
        let char_idx = if line >= self.content.len_lines() {
            self.content.len_chars()
        } else {
            self.content.line_to_char(line)
        };
        self.content.insert(char_idx, &format!("{}\n", content));
    }

    pub fn remove(&mut self, x: usize, y: usize) {
        let char_idx = self.position_to_char_idx(x, y);
        if char_idx < self.content.len_chars() {
            self.content.remove(char_idx..char_idx + 1);
        }
    }

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
    }

    pub fn view(&self, vtop: usize, vheight: usize) -> String {
        let height = std::cmp::min(vtop + vheight, self.content.len_lines());
        let mut result = String::new();
        for i in vtop..height {
            result.push_str(&self.content.line(i).to_string());
        }
        result
    }

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

        assert_eq!(buffer.view(0, 2), "this\nis");
    }

    #[test]
    fn test_view_with_small_buffer() {
        let buffer = Buffer::new(Some("example".to_string()), "a\ntest".to_string());
        assert_eq!(buffer.view(0, 5), "a\ntest");
    }
}
