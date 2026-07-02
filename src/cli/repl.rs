use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;

use crate::cli::completer::ReplHelper;
use crate::engine::executor::Executor;
use crate::engine::session::Session;
use crate::parser::ast::Statement;
use crate::parser::sql::parse_sql;

pub struct Repl {
    session: Session,
    editor: Editor<ReplHelper, DefaultHistory>,
    running: bool,
}

impl Repl {
    pub fn new() -> Self {
        let mut editor = Editor::new().expect("无法创建行编辑器");
        editor.set_helper(Some(ReplHelper::new()));
        Repl {
            session: Session::new(),
            editor,
            running: true,
        }
    }

    pub fn open_on_start(&mut self, path: &str) -> bool {
        match self.session.open(path) {
            Ok(()) => true,
            Err(_) => match self.session.create(path) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("打开文件错误 '{}': {}", path, e);
                    false
                }
            },
        }
    }

    pub fn run(&mut self) {
        println!("redb-cli v0.1.0 (redb 4.x)");
        println!("输入 .help 或 HELP; 查看帮助。");
        println!("输入 EXIT; 或 QUIT; 退出。\n");

        while self.running {
            let base = self.session.prompt_name();
            let current_prompt = if self.session.has_active_write() {
                format!("{}(txn)> ", base)
            } else {
                format!("{}> ", base)
            };

            match self.editor.readline(&current_prompt) {
                Ok(line) => {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    self.editor.add_history_entry(&line).ok();
                    self.process_line(&line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("\n再见！");
                    break;
                }
                Err(err) => {
                    eprintln!("Readline 错误: {:?}", err);
                    break;
                }
            }
        }
        let _ = self.session.close();
    }

    fn process_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        if line.starts_with('.') {
            self.process_dot_command(line);
            return;
        }

        let upper = line.to_uppercase();
        if upper == "EXIT" || upper == "QUIT" || upper == "EXIT;" || upper == "QUIT;" {
            println!("再见！");
            self.running = false;
            return;
        }
        if upper == "CLEAR" || upper == "CLEAR;" {
            print!("\x1b[2J\x1b[1;1H");
            return;
        }
        if upper == "HELP" || upper == "HELP;" {
            Executor::print_help_static();
            return;
        }

        let statements = match parse_sql(line) {
            Ok(stmts) => stmts,
            Err(e) => {
                eprintln!("解析错误: {}", e);
                return;
            }
        };

        for stmt in &statements {
            if let Statement::Exit = stmt {
                println!("再见！");
                self.running = false;
                return;
            }
            match Executor::execute(&mut self.session, stmt) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("错误: {}", e);
                }
            }
        }
    }

    fn process_dot_command(&mut self, line: &str) {
        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            ".open" => {
                if arg.is_empty() {
                    eprintln!("用法: .open <文件路径>");
                } else {
                    let _ = self.session.open(arg);
                }
            }
            ".info" | ".tables" => {
                if !self.session.is_open() {
                    eprintln!("未打开数据库。");
                    return;
                }
                if let Ok(db) = self.session.db() {
                    let r = if cmd == ".info" {
                        crate::engine::meta::execute_info(db)
                    } else {
                        crate::engine::meta::execute_show_tables(db)
                    };
                    if let Ok(r) = r {
                        println!("{}", crate::engine::display::render_table(&r));
                    }
                }
            }
            ".compact" => {
                if !self.session.is_open() {
                    eprintln!("未打开数据库。");
                    return;
                }
                match self.session.db_mut() {
                    Ok(db) => match crate::engine::meta::execute_compact(db) {
                        Ok(r) => println!("{}", crate::engine::display::render_table(&r)),
                        Err(e) => eprintln!("压缩错误: {}", e),
                    },
                    Err(e) => eprintln!("错误: {}", e),
                }
            }
            ".help" => {
                Executor::print_help_static();
            }
            _ => {
                eprintln!("未知命令: {}。输入 .help 查看可用命令。", cmd);
            }
        }
    }
}
