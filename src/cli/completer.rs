use rustyline::completion::{Candidate, Completer};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::{MatchingBracketValidator, Validator};
use rustyline::Context;
use rustyline::Helper;
use rustyline::Result;

/// Combined helper for rustyline providing completion, hinting, highlighting, and validation.
pub struct ReplHelper {
    completer: CommandCompleter,
    hinter: HistoryHinter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
}

impl ReplHelper {
    pub fn new() -> Self {
        ReplHelper {
            completer: CommandCompleter::new(),
            hinter: HistoryHinter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            validator: MatchingBracketValidator::new(),
        }
    }
}

impl Completer for ReplHelper {
    type Candidate = KeywordCandidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<KeywordCandidate>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for ReplHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for ReplHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        self.highlighter.highlight_prompt(prompt, default)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        self.highlighter.highlight_hint(hint)
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(
        &self,
        line: &str,
        pos: usize,
        forced: rustyline::highlight::CmdKind,
    ) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for ReplHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext<'_>,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

impl Helper for ReplHelper {}

/// Simple completer that suggests SQL keywords and dot commands.
pub struct CommandCompleter {
    keywords: Vec<String>,
}

impl CommandCompleter {
    pub fn new() -> Self {
        let keywords = vec![
            "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "DELETE", "CREATE", "TABLE",
            "DROP", "SHOW", "TABLES", "DESCRIBE", "DESC", "AND", "OR", "ORDER", "BY", "ASC",
            "DESC", "LIMIT", "OFFSET", "BETWEEN", "COUNT", "NOT", "NULL", "SET", "KEY", "BEGIN",
            "COMMIT", "ROLLBACK", "INT", "INTEGER", "I64", "U64", "UINT", "FLOAT", "F64", "REAL",
            "STRING", "TEXT", "STR", "BOOL", "BOOLEAN", "BYTES", "BLOB", "HELP", "CLEAR", "EXIT",
            "QUIT", ".open", ".info", ".compact", ".help",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        CommandCompleter { keywords }
    }
}

impl Completer for CommandCompleter {
    type Candidate = KeywordCandidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<KeywordCandidate>)> {
        let line_bytes = line.as_bytes();
        if pos > line_bytes.len() {
            return Ok((0, vec![]));
        }

        // Find start of current word
        let mut start = pos;
        while start > 0 {
            let prev = line_bytes[start - 1];
            if !prev.is_ascii_alphanumeric() && prev != b'_' && prev != b'.' {
                break;
            }
            start -= 1;
            if start == 0 {
                break;
            }
        }

        let current_word = &line[start..pos];
        if current_word.is_empty() {
            return Ok((pos, vec![]));
        }

        let current_upper = current_word.to_uppercase();

        let matches: Vec<KeywordCandidate> = self
            .keywords
            .iter()
            .filter(|kw| {
                let kw_upper = kw.to_uppercase();
                kw_upper.starts_with(&current_upper) && kw_upper != current_upper
            })
            .map(|kw| KeywordCandidate(kw.clone()))
            .collect();

        Ok((start, matches))
    }
}

#[derive(Debug)]
pub struct KeywordCandidate(String);

impl Candidate for KeywordCandidate {
    fn display(&self) -> &str {
        &self.0
    }

    fn replacement(&self) -> &str {
        &self.0
    }
}
