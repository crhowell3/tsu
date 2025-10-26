use std::num::NonZeroU32;

use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_rust::HIGHLIGHTS_QUERY;

use crate::{editor::StyleInfo, theme::Theme};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Highlight(NonZeroU32);

impl Highlight {
    pub const MAX: u32 = u32::MAX - 1;

    pub const fn new(inner: u32) -> Self {
        assert!(inner != u32::MAX);
        Self(NonZeroU32::new(inner ^ u32::MAX).unwrap())
    }

    pub const fn get(&self) -> u32 {
        self.0.get() ^ u32::MAX
    }

    pub const fn idx(&self) -> usize {
        self.get() as usize
    }
}

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
    ///
    /// # Errors
    /// This function will return a `LanguageError` if the tree sitter's parser is unable to set a
    /// language
    ///
    /// # Panics
    /// This function might panic if the value returned by `set_langauge` is an `Err`, with a panic
    /// message including the passed message, and the content of the `Err`
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
    ///
    /// # Errors
    /// There does not appear to be any function call within this code that returns an error, so
    /// this function should not return an `Err`
    ///
    /// # Panics
    /// This function might panic if the `colors` vector size exceeds `isize::MAX`
    pub fn highlight(&mut self, code: &str) -> anyhow::Result<Vec<StyleInfo>> {
        let tree = self.parser.parse(code, None).expect("parse works");

        let mut colors = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.query, tree.root_node(), code.as_bytes());

        while let Some(mat) = matches.next() {
            for cap in mat.captures {
                let node = cap.node;
                let start = node.start_byte();
                let end = node.end_byte();
                let scope = self.query.capture_names()[cap.index as usize];
                let style = self.theme.get(scope);

                colors.push(StyleInfo { start, end, style });
            }
        }

        Ok(colors)
    }
}
