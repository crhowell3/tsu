use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_rust::HIGHLIGHTS_QUERY;

use crate::{editor::StyleInfo, theme::Theme};

/// Contains the data and logic for performing syntax highlighting when opening certain file types
pub struct Highlighter {
    /// The syntax parser for tokenizing and highlighting
    parser: Parser,
    /// Result of querying the language of the file
    query: Query,
    /// Color theme to use for applying highlighting
    theme: Theme,
}

impl Highlighter {
    /// Construct a new `Highlighter` with a given theme
    ///
    /// # Arguments
    /// - `theme`: The color theme to use for applying highlighting
    ///
    /// # Returns
    /// - An instance of a `Highlighter` struct
    pub fn new(theme: &Theme) -> anyhow::Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("Error loading Rust grammar");
        let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), HIGHLIGHTS_QUERY)?;
        let theme = theme.clone();

        Ok(Self {
            parser,
            query,
            theme,
        })
    }

    /// Executes the highlighting functionality on some string
    ///
    /// # Arguments
    /// - `code`: The string to be highlighted
    ///
    /// # Returns
    /// - A vector of `StyleInfo` which is used by the terminal to highlight/decorate the text
    pub fn highlight(&mut self, code: &str) -> anyhow::Result<Vec<StyleInfo>> {
        let tree = self.parser.parse(code, None).expect("parse works");

        let mut colors = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.query, tree.root_node(), code.as_bytes());

        while let Some(mat) = {
            matches.advance();
            matches.get()
        } {
            for cap in mat.captures {
                let node = cap.node;
                let start = node.start_byte();
                let end = node.end_byte();
                let scope = self.query.capture_names()[cap.index as usize];
                let style = self.theme.get_style(scope);

                if let Some(style) = style {
                    colors.push(StyleInfo { start, end, style });
                }
            }
        }

        Ok(colors)
    }
}
