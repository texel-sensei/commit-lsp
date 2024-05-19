use std::fmt::Display;

use tower_lsp::lsp_types::{self, Position, Range};
use tracing::info;

use crate::regex;

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
            .rfind(|c: char| !c.is_alphanumeric() && c != '#')
            .map(|i| i + 1)
            .unwrap_or(0);

        let end = line[cursor..]
            .find(|c: char| !c.is_alphanumeric() && c != '#')
            .map(|i| i + cursor)
            .unwrap_or(line.len());

        if start == end {
            return None;
        }

        let range = self.partial_line(pos.line, start..end);
        let text = self.get_text(range);
        info!(text, "Found word under cursor");

        let ticket_regex = regex!(r"#([0-9]+)");

        let kind = {
            if let Some(caps) = ticket_regex.captures(&text) {
                let Ok(id) = caps
                    .get(1)
                    .expect("There should be one capture")
                    .as_str()
                    .parse()
                else {
                    return None;
                };
                ItemKind::Ref(id)
            } else {
                // TODO(texel, 2024-05-19): determine other types
                return None;
            }
        };

        Some(Item { kind, text, range })
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

    /// Returns the commit text inside the given range.
    fn get_text(&self, range: Range) -> String {
        // range.end.line is inclusive
        let line_range = (range.start.line as usize)..=(range.end.line as usize);

        let lines = &self.lines[line_range];

        // count bytes preceding the last line, taking newlines into account
        let offset: usize = lines
            .iter()
            .take(lines.len() - 1) // do not count line containing "range.end"
            .map(|l| l.len() + 1) // + 1 for the newlines we will add
            .sum();

        let text = lines.join("\n");

        let char_range = (range.start.character as usize)..(range.end.character as usize + offset);

        text[char_range].to_owned()
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

#[cfg(test)]
mod test {
    use super::*;

    /// Parse a text example with embedded range markers.
    ///
    /// `|>` will mark the start of the range and `<|` marks the end.
    ///
    /// Example input string:
    /// ```gitcommit
    /// The range |>is this part<| of the text.
    /// ```
    fn text_with_range(text: &str) -> (String, Range) {
        let mut iter = text
            .lines()
            .enumerate()
            .filter(|(_, l)| l.contains("|>") || l.contains("<|"));

        let (mut idx, mut line) = iter.next().unwrap();
        let begin = {
            let char = line.find("|>").unwrap();
            Position::new(idx as u32, char as u32)
        };

        let end = {
            let single_line = line.contains("<|");
            if !single_line {
                (idx, line) = iter.next().unwrap();
            }

            let mut char = line.find("<|").unwrap();

            if single_line {
                // skip the `|>` that precedes us in single line mode
                char -= 2;
            }
            Position::new(idx as u32, char as u32)
        };

        assert_eq!(iter.next(), None);

        (
            text.replace("|>", "").replace("<|", ""),
            Range::new(begin, end),
        )
    }

    fn example(text: &str) -> (State, Range) {
        let (text, range) = text_with_range(text);
        (State::new(&text), range)
    }

    #[test]
    fn test_get_text_single_line() {
        let (state, range) = example("this |>is a<| test");

        assert_eq!(state.get_text(range), "is a");
    }

    #[test]
    fn test_get_text_full_single_line() {
        let (state, range) = example("|>this is a test<|");

        assert_eq!(state.get_text(range), "this is a test");
    }

    #[test]
    fn test_get_text_full_empty_range() {
        let (state, range) = example("this is|><| a test");

        assert_eq!(state.get_text(range), "");
    }

    #[test]
    fn test_get_text_multi_line() {
        let (state, range) = example("this is a |>test\nover two<| lines");

        assert_eq!(state.get_text(range), "test\nover two");
    }
}
