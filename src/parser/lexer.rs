/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum Token {
    // Keywords
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Delete,
    Create,
    Drop,
    Table,
    Show,
    Tables,
    Describe,
    Desc,
    And,
    Or,
    Order,
    By,
    Asc,
    Desc_,
    Limit,
    Offset,
    Between,
    Count,
    Not,
    Null,
    Set,
    Help,
    Clear,
    Exit,
    Quit,
    Begin,
    Commit,
    Rollback,
    Compact,

    // Dot commands
    DotOpen,
    DotInfo,
    DotCompact,
    DotHelp,

    // Symbols
    Semicolon,
    Comma,
    LParen,
    RParen,
    Star,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    Dot,
    VerticalTerminator,

    // Literals
    Ident(String),
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    Eof,
}

impl Token {
    #[allow(dead_code)]
    pub fn is_keyword(&self, s: &str) -> bool {
        matches!(
            (self, s.to_uppercase().as_str()),
            (Token::Select, "SELECT")
                | (Token::From, "FROM")
                | (Token::Where, "WHERE")
                | (Token::Insert, "INSERT")
                | (Token::Into, "INTO")
                | (Token::Values, "VALUES")
                | (Token::Delete, "DELETE")
                | (Token::Create, "CREATE")
                | (Token::Drop, "DROP")
                | (Token::Table, "TABLE")
                | (Token::Show, "SHOW")
                | (Token::Tables, "TABLES")
                | (Token::Describe, "DESCRIBE")
                | (Token::Desc, "DESC")
                | (Token::And, "AND")
                | (Token::Order, "ORDER")
                | (Token::By, "BY")
                | (Token::Asc, "ASC")
                | (Token::Desc_, "DESC")
                | (Token::Limit, "LIMIT")
                | (Token::Offset, "OFFSET")
                | (Token::Between, "BETWEEN")
                | (Token::Count, "COUNT")
                | (Token::Not, "NOT")
                | (Token::Null, "NULL")
                | (Token::Help, "HELP")
                | (Token::Clear, "CLEAR")
                | (Token::Exit, "EXIT")
                | (Token::Quit, "QUIT")
                | (Token::Begin, "BEGIN")
                | (Token::Commit, "COMMIT")
                | (Token::Rollback, "ROLLBACK")
                | (Token::Compact, "COMPACT")
        )
    }
}

/// Lexer: converts a SQL string into a stream of tokens.
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }
    fn advance(&mut self) {
        self.pos += 1;
    }
    fn peek(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '-' && self.peek() == Some('-') {
                self.advance();
                self.advance();
                while let Some(c) = self.current() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let word: String = self.input[start..self.pos].iter().collect();
        Self::match_keyword(&word)
    }

    fn match_keyword(word: &str) -> Token {
        match word.to_uppercase().as_str() {
            "SELECT" => Token::Select,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "INSERT" => Token::Insert,
            "INTO" => Token::Into,
            "VALUES" => Token::Values,
            "DELETE" => Token::Delete,
            "CREATE" => Token::Create,
            "DROP" => Token::Drop,
            "TABLE" => Token::Table,
            "SHOW" => Token::Show,
            "TABLES" => Token::Tables,
            "DESCRIBE" => Token::Describe,
            "DESC" => Token::Desc,
            "AND" => Token::And,
            "OR" => Token::Or,
            "ORDER" => Token::Order,
            "BY" => Token::By,
            "ASC" => Token::Asc,
            "LIMIT" => Token::Limit,
            "OFFSET" => Token::Offset,
            "BETWEEN" => Token::Between,
            "COUNT" => Token::Count,
            "NOT" => Token::Not,
            "NULL" => Token::Null,
            "SET" => Token::Set,
            "HELP" => Token::Help,
            "CLEAR" => Token::Clear,
            "EXIT" => Token::Exit,
            "QUIT" => Token::Quit,
            "BEGIN" => Token::Begin,
            "COMMIT" => Token::Commit,
            "ROLLBACK" => Token::Rollback,
            "COMPACT" => Token::Compact,
            "TRUE" => Token::IntLit(1),
            "FALSE" => Token::IntLit(0),
            _ => Token::Ident(word.to_string()),
        }
    }

    fn read_string(&mut self) -> Result<Token, String> {
        let quote = self.current().unwrap();
        self.advance();
        let mut s = String::new();
        while let Some(ch) = self.current() {
            if ch == quote {
                self.advance();
                return Ok(Token::StringLit(s));
            }
            if ch == '\\' {
                self.advance();
                if let Some(esc) = self.current() {
                    let ch = match esc {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '\'' => '\'',
                        '"' => '"',
                        c => {
                            s.push('\\');
                            c
                        }
                    };
                    s.push(ch);
                    self.advance();
                    continue;
                }
            }
            s.push(ch);
            self.advance();
        }
        Err("Unterminated string literal".to_string())
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let mut is_float = false;
        if self.current() == Some('-') {
            self.advance();
        }
        while let Some(ch) = self.current() {
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !is_float {
                is_float = true;
                self.advance();
            } else {
                break;
            }
        }
        let num: String = self.input[start..self.pos].iter().collect();
        if is_float {
            Token::FloatLit(num.parse().unwrap_or(0.0))
        } else {
            Token::IntLit(num.parse().unwrap_or(0))
        }
    }

    fn read_dot_command(&mut self) -> Token {
        self.advance();
        let start = self.pos;
        while let Some(ch) = self.current() {
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let cmd: String = self.input[start..self.pos].iter().collect();
        match cmd.to_lowercase().as_str() {
            "open" => Token::DotOpen,
            "info" => Token::DotInfo,
            "compact" => Token::DotCompact,
            "help" => Token::DotHelp,
            _ => Token::Ident(format!(".{}", cmd)),
        }
    }

    fn next_token_internal(&mut self) -> Result<Token, String> {
        self.skip_whitespace();
        let ch = match self.current() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };

        if ch == '.' && self.peek().is_some_and(|c| c.is_alphabetic()) {
            return Ok(self.read_dot_command());
        }

        match ch {
            ';' => {
                self.advance();
                Ok(Token::Semicolon)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            '*' => {
                self.advance();
                Ok(Token::Star)
            }
            '=' => {
                self.advance();
                Ok(Token::Eq)
            }
            '!' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::Neq)
                } else {
                    Err("Unexpected character '!'".to_string())
                }
            }
            '<' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::Le)
                } else if self.current() == Some('>') {
                    self.advance();
                    Ok(Token::Neq)
                } else {
                    Ok(Token::Lt)
                }
            }
            '>' => {
                self.advance();
                if self.current() == Some('=') {
                    self.advance();
                    Ok(Token::Ge)
                } else {
                    Ok(Token::Gt)
                }
            }
            '.' => {
                self.advance();
                Ok(Token::Dot)
            }
            '\\' => {
                self.advance();
                if self.current() == Some('G') || self.current() == Some('g') {
                    self.advance();
                    Ok(Token::VerticalTerminator)
                } else {
                    Err("Unexpected backslash command. Did you mean \\G?".to_string())
                }
            }
            '\'' | '"' => self.read_string(),
            c if c.is_alphabetic() || c == '_' => Ok(self.read_ident_or_keyword()),
            c if c.is_ascii_digit() || c == '-' => Ok(self.read_number()),
            _ => Err(format!("Unexpected character: '{}'", ch)),
        }
    }
}

impl Iterator for Lexer {
    type Item = Result<Token, String>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token_internal() {
            Ok(Token::Eof) => None,
            Ok(tok) => Some(Ok(tok)),
            Err(e) => Some(Err(e)),
        }
    }
}

#[allow(dead_code)]
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    Lexer::new(input).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let tokens = tokenize("SELECT * FROM my_table").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], Token::Select);
        assert_eq!(tokens[1], Token::Star);
        assert_eq!(tokens[2], Token::From);
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize("'hello world'").unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_keywords_case_insensitive() {
        let tokens = tokenize("select * from tbl").unwrap();
        assert_eq!(tokens.len(), 4);
    }
}
