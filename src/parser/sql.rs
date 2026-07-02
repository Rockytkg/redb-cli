use super::ast::*;
use super::lexer::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(input: &str) -> Result<Self, String> {
        let tokens: Vec<Token> = super::lexer::Lexer::new(input).collect::<Result<Vec<_>, _>>()?;
        Ok(Parser { tokens, pos: 0 })
    }

    pub fn parse_statement(&mut self) -> Result<Statement, String> {
        match self.current() {
            Some(Token::Select) => self.parse_select(),
            Some(Token::Insert) => self.parse_insert(),
            Some(Token::Delete) => self.parse_delete(),
            Some(Token::Create) => self.parse_create(),
            Some(Token::Drop) => self.parse_drop(),
            Some(Token::Show) => self.parse_show(),
            Some(Token::Describe) | Some(Token::Desc) => self.parse_describe(),
            Some(Token::Help) => {
                self.advance();
                self.skip_statement_terminator();
                Ok(Statement::Help)
            }
            Some(Token::Clear) => {
                self.advance();
                self.skip_statement_terminator();
                Ok(Statement::Clear)
            }
            Some(Token::Exit) | Some(Token::Quit) => {
                self.advance();
                self.skip_statement_terminator();
                Ok(Statement::Exit)
            }
            Some(Token::Begin) => {
                self.advance();
                self.skip_statement_terminator();
                Ok(Statement::Begin)
            }
            Some(Token::Commit) => {
                self.advance();
                self.skip_statement_terminator();
                Ok(Statement::Commit)
            }
            Some(Token::Rollback) => {
                self.advance();
                self.skip_statement_terminator();
                Ok(Statement::Rollback)
            }
            Some(Token::DotOpen) => self.parse_dot_open(),
            Some(Token::DotInfo) => {
                self.advance();
                let output_mode = self.consume_output_terminator();
                Ok(Statement::DotInfo { output_mode })
            }
            Some(Token::DotCompact) => {
                self.advance();
                let output_mode = self.consume_output_terminator();
                Ok(Statement::DotCompact { output_mode })
            }
            Some(Token::DotHelp) => {
                self.advance();
                Ok(Statement::DotHelp)
            }
            Some(Token::Compact) => {
                self.advance();
                let output_mode = self.consume_output_terminator();
                Ok(Statement::DotCompact { output_mode })
            }
            Some(tok) => Err(format!("Unexpected token: {:?}", tok)),
            None => Err("Empty input".to_string()),
        }
    }

    pub fn parse_statements(&mut self) -> Result<Vec<Statement>, String> {
        let mut stmts = Vec::new();
        while self.current().is_some() {
            stmts.push(self.parse_statement()?);
            if self.current() == Some(&Token::Semicolon) {
                self.advance();
            }
        }
        Ok(stmts)
    }

    // --- helpers ---
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }
    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }
    fn skip_statement_terminator(&mut self) {
        if self.current() == Some(&Token::Semicolon) {
            self.advance();
        }
    }

    fn consume_output_terminator(&mut self) -> OutputMode {
        match self.current() {
            Some(Token::VerticalTerminator) => {
                self.advance();
                if self.current() == Some(&Token::Semicolon) {
                    self.advance();
                }
                OutputMode::Vertical
            }
            Some(Token::Semicolon) => {
                self.advance();
                OutputMode::Table
            }
            _ => OutputMode::Table,
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s),
            Some(t) => Err(format!("Expected identifier, got {:?}", t)),
            None => Err("Unexpected end of input".into()),
        }
    }

    fn eat(&mut self, tok: &Token) -> Result<(), String> {
        match self.current() {
            Some(t) if std::mem::discriminant(t) == std::mem::discriminant(tok) => {
                self.advance();
                Ok(())
            }
            Some(t) => Err(format!("Expected {:?}, got {:?}", tok, t)),
            None => Err("Unexpected end of input".into()),
        }
    }

    // --- dot command ---
    fn parse_dot_open(&mut self) -> Result<Statement, String> {
        self.advance(); // DotOpen
        let mut path = self.expect_ident()?;
        while self.current() == Some(&Token::Dot) {
            self.advance();
            if let Some(Token::Ident(p)) = self.advance() {
                path = format!("{}.{}", path, p);
            } else {
                break;
            }
        }
        self.skip_statement_terminator();
        Ok(Statement::DotOpen(path))
    }

    // --- SELECT ---
    fn parse_select(&mut self) -> Result<Statement, String> {
        self.advance(); // SELECT
        let count_only = if self.current() == Some(&Token::Count) {
            self.advance();
            self.eat(&Token::LParen)?;
            self.eat(&Token::Star)?;
            self.eat(&Token::RParen)?;
            true
        } else if self.current() == Some(&Token::Star) {
            self.advance();
            false
        } else {
            let _ = self.expect_ident()?;
            false
        };

        self.eat(&Token::From)?;
        let table = self.expect_ident()?;

        let condition = if self.current() == Some(&Token::Where) {
            self.advance();
            Some(self.parse_condition()?)
        } else {
            None
        };

        let order_by = if self.current() == Some(&Token::Order) {
            self.advance();
            self.eat(&Token::By)?;
            let col = self.expect_ident()?;
            let dir = match self.current() {
                Some(Token::Desc) | Some(Token::Desc_) => {
                    self.advance();
                    OrderDirection::Desc
                }
                Some(Token::Asc) => {
                    self.advance();
                    OrderDirection::Asc
                }
                _ => OrderDirection::Asc,
            };
            Some(OrderBy {
                column: col,
                direction: dir,
            })
        } else {
            None
        };

        let limit = if self.current() == Some(&Token::Limit) {
            self.advance();
            match self.advance() {
                Some(Token::IntLit(n)) if n >= 0 => Some(n as u64),
                _ => return Err("Expected integer for LIMIT".into()),
            }
        } else {
            None
        };

        let offset = if self.current() == Some(&Token::Offset) {
            self.advance();
            match self.advance() {
                Some(Token::IntLit(n)) if n >= 0 => Some(n as u64),
                _ => return Err("Expected integer for OFFSET".into()),
            }
        } else {
            None
        };

        let output_mode = self.consume_output_terminator();
        Ok(Statement::Select {
            table,
            condition,
            order_by,
            limit,
            offset,
            count_only,
            output_mode,
        })
    }

    fn parse_condition(&mut self) -> Result<Condition, String> {
        let col = self.expect_ident()?;
        match self.current() {
            Some(Token::Between) => {
                self.advance();
                let v1 = self.parse_literal()?;
                self.eat(&Token::And)?;
                let v2 = self.parse_literal()?;
                Ok(Condition::Between(col, v1, v2))
            }
            Some(Token::Eq) => {
                self.advance();
                Ok(Condition::Equals(col, self.parse_literal()?))
            }
            Some(Token::Neq) => {
                self.advance();
                Ok(Condition::NotEquals(col, self.parse_literal()?))
            }
            Some(Token::Gt) => {
                self.advance();
                Ok(Condition::GreaterThan(col, self.parse_literal()?))
            }
            Some(Token::Ge) => {
                self.advance();
                Ok(Condition::GreaterEquals(col, self.parse_literal()?))
            }
            Some(Token::Lt) => {
                self.advance();
                Ok(Condition::LessThan(col, self.parse_literal()?))
            }
            Some(Token::Le) => {
                self.advance();
                Ok(Condition::LessEquals(col, self.parse_literal()?))
            }
            Some(t) => Err(format!("Expected comparison operator, got {:?}", t)),
            None => Err("Unexpected end of input in WHERE".into()),
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, String> {
        match self.advance() {
            Some(Token::IntLit(n)) => Ok(Literal::Int(n)),
            Some(Token::FloatLit(f)) => Ok(Literal::Float(f)),
            Some(Token::StringLit(s)) => Ok(Literal::String(s)),
            Some(Token::Null) => Ok(Literal::Null),
            Some(Token::Ident(s)) => Ok(Literal::String(s)),
            Some(t) => Err(format!("Expected literal, got {:?}", t)),
            None => Err("Unexpected end of input".into()),
        }
    }

    // --- INSERT ---
    fn parse_insert(&mut self) -> Result<Statement, String> {
        self.advance();
        self.eat(&Token::Into)?;
        let table = self.expect_ident()?;
        self.eat(&Token::Values)?;
        self.eat(&Token::LParen)?;
        let key = self.parse_literal()?;
        self.eat(&Token::Comma)?;
        let value = self.parse_literal()?;
        self.eat(&Token::RParen)?;
        let output_mode = self.consume_output_terminator();
        Ok(Statement::Insert {
            table,
            key,
            value,
            output_mode,
        })
    }

    // --- DELETE ---
    fn parse_delete(&mut self) -> Result<Statement, String> {
        self.advance();
        self.eat(&Token::From)?;
        let table = self.expect_ident()?;
        self.eat(&Token::Where)?;
        let condition = self.parse_condition()?;
        let output_mode = self.consume_output_terminator();
        Ok(Statement::Delete {
            table,
            condition,
            output_mode,
        })
    }

    // --- CREATE ---
    fn parse_create(&mut self) -> Result<Statement, String> {
        self.advance();
        self.eat(&Token::Table)?;
        let name = self.expect_ident()?;
        self.eat(&Token::LParen)?;
        let kt = self.expect_ident()?;
        self.eat(&Token::Comma)?;
        let vt = self.expect_ident()?;
        self.eat(&Token::RParen)?;
        let output_mode = self.consume_output_terminator();
        let key_type =
            DataType::from_str(&kt).ok_or_else(|| format!("Unknown key type: {}", kt))?;
        let value_type =
            DataType::from_str(&vt).ok_or_else(|| format!("Unknown value type: {}", vt))?;
        Ok(Statement::CreateTable {
            name,
            key_type,
            value_type,
            output_mode,
        })
    }

    // --- DROP ---
    fn parse_drop(&mut self) -> Result<Statement, String> {
        self.advance();
        self.eat(&Token::Table)?;
        let name = self.expect_ident()?;
        let output_mode = self.consume_output_terminator();
        Ok(Statement::DropTable { name, output_mode })
    }

    // --- SHOW ---
    fn parse_show(&mut self) -> Result<Statement, String> {
        self.advance();
        self.eat(&Token::Tables)?;
        let output_mode = self.consume_output_terminator();
        Ok(Statement::ShowTables { output_mode })
    }

    // --- DESCRIBE ---
    fn parse_describe(&mut self) -> Result<Statement, String> {
        self.advance();
        let table = self.expect_ident()?;
        let output_mode = self.consume_output_terminator();
        Ok(Statement::Describe { table, output_mode })
    }
}

pub fn parse_sql(input: &str) -> Result<Vec<Statement>, String> {
    Parser::new(input)?.parse_statements()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_show_tables() {
        let stmts = parse_sql("SHOW TABLES").unwrap();
        assert_eq!(
            stmts[0],
            Statement::ShowTables {
                output_mode: OutputMode::Table
            }
        );
    }

    #[test]
    fn test_select() {
        let stmts = parse_sql("SELECT * FROM t").unwrap();
        assert!(matches!(&stmts[0], Statement::Select { table, .. } if table == "t"));
    }

    #[test]
    fn test_select_where() {
        let stmts = parse_sql("SELECT * FROM t WHERE key = 42").unwrap();
        assert!(matches!(
            &stmts[0],
            Statement::Select {
                condition: Some(Condition::Equals(_, Literal::Int(42))),
                ..
            }
        ));
    }

    #[test]
    fn test_insert() {
        let stmts = parse_sql("INSERT INTO t VALUES ('k', 123)").unwrap();
        assert!(matches!(&stmts[0], Statement::Insert { table, .. } if table == "t"));
    }

    #[test]
    fn test_create() {
        let stmts = parse_sql("CREATE TABLE t (I64, STRING)").unwrap();
        assert!(matches!(&stmts[0], Statement::CreateTable { name, .. } if name == "t"));
    }

    #[test]
    fn test_select_vertical_terminator() {
        let stmts = parse_sql("SELECT * FROM t\\G").unwrap();
        assert!(matches!(
            &stmts[0],
            Statement::Select {
                table,
                output_mode: OutputMode::Vertical,
                ..
            } if table == "t"
        ));
    }

    #[test]
    fn test_show_tables_vertical_terminator() {
        let stmts = parse_sql("SHOW TABLES \\G").unwrap();
        assert_eq!(
            stmts[0],
            Statement::ShowTables {
                output_mode: OutputMode::Vertical
            }
        );
    }
}
