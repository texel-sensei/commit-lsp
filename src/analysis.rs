use std::fmt::Display;

use tower_lsp::lsp_types::{self, Position, Range};
use tracing::info;

use crate::{
    config::{self, CommitElementDefinition},
    regex,
};

pub struct State {
    config: config::Repository,

    lines: Vec<String>,

    ty: Option<Range>,
    scope: Option<Range>,
}

impl State {
    pub fn new(config: config::Repository) -> Self {
        Self {
            config,
            lines: Vec::new(),
            ty: None,
            scope: None,
        }
    }

    pub fn update_text(&mut self, new_text: &str) {
        self.lines = new_text.lines().map(ToOwned::to_owned).collect();

        if let Some(header) = self.lines.first() {
            if let Some((ty, scope, _breaking)) = parse_header(header) {
                self.ty = Some(self.partial_line(0, substr_offset(header, ty)));

                self.scope = scope.map(|txt| self.partial_line(0, substr_offset(header, txt)));
            }
        }
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

        let kind = {
            let ticket_regex = regex!(r"#([0-9]+)");
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
            } else if Some(range) == self.ty {
                ItemKind::Ty
            } else if Some(range) == self.scope {
                ItemKind::Scope
            } else {
                // TODO(texel, 2024-05-19): determine other types
                return None;
            }
        };

        Some(Item { kind, text, range })
    }

    pub fn commit_type_info(&self) -> Option<CommitElementDefinition> {
        let ty = self.get_text(self.ty?);
        self.config.types.iter().find(|t| t.name == ty).cloned()
    }

    pub fn commit_scope_info(&self) -> Option<CommitElementDefinition> {
        let ty = self.get_text(self.scope?);
        self.config.scopes.iter().find(|t| t.name == ty).cloned()
    }

    pub fn get_commit_types(&self) -> &[CommitElementDefinition] {
        &self.config.types
    }

    pub fn get_commit_scopes(&self) -> &[CommitElementDefinition] {
        &self.config.scopes
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

fn parse_header(first_line: &str) -> Option<(&str, Option<&str>, bool)> {
    let header_format =
        regex!(r#"(?P<ty>[a-z]+)(?:\((?P<scope>[^)]+)\))?(?P<breaking>!)?: (?P<subject>.*)$"#);

    let captures = header_format.captures(first_line)?;

    let ty = captures.name("ty")?.as_str();
    let scope = captures.name("scope").map(|m| m.as_str());
    let breaking = captures.name("breaking").is_some();

    Some((ty, scope, breaking))
}

/// Returns the offset of a string slice in another string slice.
/// The second slice **MUST** point into part of the first.
fn substr_offset<'needle, 'haystack: 'needle>(
    container: &'haystack str,
    contained: &'needle str,
) -> std::ops::Range<usize> {
    let delta = unsafe { contained.as_ptr().offset_from(container.as_ptr()) };
    assert!(delta >= 0);

    let delta = delta as usize;
    assert!(delta + contained.len() <= container.len());

    delta..delta + contained.len()
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
        let mut state = State::new(Default::default());
        state.update_text(&text);
        (state, range)
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

    #[test]
    fn test_parse_header_with_scope() {
        let example = "feat(lsp): implement the thing";

        let (ty, scope, breaking) = parse_header(example).unwrap();

        assert_eq!(ty, "feat");
        assert_eq!(scope, Some("lsp"));
        assert!(!breaking);
    }

    #[test]
    fn test_parse_header_without_scope() {
        let example = "feat: implement the thing";

        let (ty, scope, breaking) = parse_header(example).unwrap();

        assert_eq!(ty, "feat");
        assert_eq!(scope, None);
        assert!(!breaking);
    }

    #[test]
    fn test_parse_header_with_scope_and_breaking_change() {
        let example = "feat(lsp)!: implement the thing";

        let (ty, scope, breaking) = parse_header(example).unwrap();

        assert_eq!(ty, "feat");
        assert_eq!(scope, Some("lsp"));
        assert!(breaking);
    }

    #[test]
    fn test_substring_offset_works() {
        let outer = "Hello World!";
        let inner = &outer[6..];

        assert_eq!(substr_offset(outer, inner), 6..12);
    }
}
