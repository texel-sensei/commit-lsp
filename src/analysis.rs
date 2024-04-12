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

    fn full_line(&self, idx: u32) -> Range {
        Range::new(
            Position::new(idx, 0),
            Position::new(idx, self.lines[idx as usize].len() as u32),
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
