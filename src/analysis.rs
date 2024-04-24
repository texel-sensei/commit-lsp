use std::fmt::Display;

use tower_lsp::lsp_types::{self, Position, Range};

pub struct State {
    lines: Vec<String>,
}

impl State {
    pub fn new(text: &str) -> Self {
        Self {
            lines: text.lines().map(ToOwned::to_owned).collect(),
        }
    }

    pub fn update_text(&mut self, new_text: &str) {
        self.lines = new_text.lines().map(ToOwned::to_owned).collect();
    }

    pub fn all_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if self.lines.len() > 1 && !self.lines[1].is_empty() {
            diagnostics.push(Diagnostic::new(
                self.full_line(1),
                "The second line should be empty!",
            ));
        }

        diagnostics
    }

    /// Look at the given position in the text and return the element there.
    /// Returns `None` for out of bounds accesses and if there is nothing special there.
    pub fn lookup(&self, pos: Position) -> Option<Item> {
        let cursor = pos.character as usize;
        let line = self.lines.get(pos.line as usize)?;

        // find word under cursor
        let start = line[..cursor]
            .rfind(|c: char| !c.is_alphabetic())
            .map(|i| i+1)
            .unwrap_or(0);

        let end = line[cursor..]
            .find(|c: char| !c.is_alphabetic())
            .map(|i| i + cursor)
            .unwrap_or(line.len());

        if start == end {
            return None;
        }

        let text = line[start..end].to_owned();

        Some(Item{
            kind: ItemKind::Ty, // TODO
            text,
            range: self.partial_line(pos.line, start..end)
        })
    }

    fn full_line(&self, idx: u32) -> Range {
        Range::new(
            Position::new(idx, 0),
            Position::new(idx, self.lines[idx as usize].len() as u32),
        )
    }

    fn partial_line(&self, line: u32, range: std::ops::Range<usize>) -> Range {
        Range::new(
            Position::new(line, range.start as u32),
            Position::new(line, range.end as u32),
        )
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new("")
    }
}

pub struct Diagnostic {
    inner: lsp_types::Diagnostic,
}

impl Diagnostic {
    pub fn new(range: Range, message: impl ToString) -> Self {
        Self {
            inner: lsp_types::Diagnostic {
                range,
                severity: None,
                code: None,
                code_description: None,
                source: None,
                message: message.to_string(),
                related_information: None,
                tags: None,
                data: None,
            },
        }
    }
}

impl From<Diagnostic> for lsp_types::Diagnostic {
    fn from(value: Diagnostic) -> Self {
        value.inner
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line = self.inner.range.start.line + 1;
        let col = self.inner.range.start.character + 1;
        let msg = &self.inner.message;
        write!(f, "[{line}:{col}] {msg}")
    }
}

pub struct Item {
    pub kind: ItemKind,
    pub text: String,
    pub range: Range,
}

/// An item of interest in the commit text.
pub enum ItemKind {
    /// The commit type (e.g. `feat`, `fix`)
    Ty,
    /// The commit scope
    Scope,
    /// A reference to a ticket/issue/etc
    Ref(u64),
}
