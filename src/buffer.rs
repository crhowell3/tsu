#[derive(Debug)]
pub struct Buffer {
    pub file: Option<String>,
    pub lines: Vec<String>,
}

impl Buffer {
    pub fn new(file: Option<String>, contents: String) -> Self {
        let lines = contents.lines().map(|s| s.to_string()).collect();
        Self { file, lines }
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

    pub fn save(&self) -> anyhow::Result<String> {
        if let Some(file) = &self.file {
            let contents = self.lines.join("\n");
            std::fs::write(file, &contents)?;
            let message = format!(
                "{:?} {}L, {}B written",
                file,
                self.lines.len(),
                contents.len()
            );
            Ok(message)
        } else {
            Err(anyhow::anyhow!("No file name"))
        }
    }

    pub fn get(&self, line: usize) -> Option<String> {
        if self.lines.len() > line {
            return Some(self.lines[line].clone());
        }

        None
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn insert(&mut self, x: usize, y: usize, c: char) {
        if let Some(line) = self.lines.get_mut(y) {
            (*line).insert(x, c);
        }
    }

    pub fn insert_line(&mut self, line: usize, content: String) {
        self.lines.insert(line, content);
    }

    pub fn remove(&mut self, x: usize, y: usize) {
        if let Some(line) = self.lines.get_mut(y) {
            (*line).remove(x);
        }
    }

    pub fn remove_line(&mut self, line: usize) {
        if self.len() > line {
            self.lines.remove(line);
        }
    }

    pub(crate) fn view(&self, vtop: usize, vheight: usize) -> String {
        let height = std::cmp::min(vtop + vheight, self.lines.len());
        self.lines[vtop..height].join("\n")
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
