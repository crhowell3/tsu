use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use tree_sitter_rust::HIGHLIGHTS_QUERY;

use crate::{editor::StyleInfo, theme::Theme};

pub struct Highlighter {
    parser: Parser,
    query: Query,
    theme: Theme,
}

impl Highlighter {
    #[cfg(miri)]
    pub fn new(theme: &Theme) -> anyhow::Result<Self> {
        Ok(())
    }

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
