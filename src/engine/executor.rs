use crate::engine::ddl;
use crate::engine::display::render_result;
use crate::engine::meta;
use crate::engine::mutate;
use crate::engine::query;
use crate::engine::session::Session;
use crate::error::CliResult;
use crate::parser::ast::Statement;

pub struct Executor;

impl Executor {
    pub fn execute(session: &mut Session, stmt: &Statement) -> CliResult<()> {
        match stmt {
            Statement::DotOpen(path) => {
                session.open(path)?;
                Ok(())
            }
            Statement::DotInfo { output_mode } => {
                let db = session.db()?;
                let r = meta::execute_info(db)?;
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::DotCompact { output_mode } => {
                let db = session.db_mut()?;
                let r = meta::execute_compact(db)?;
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::DotHelp => {
                Self::print_help_static();
                Ok(())
            }
            Statement::ShowTables { output_mode } => {
                let db = session.db()?;
                let r = meta::execute_show_tables(db)?;
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::Describe { table, output_mode } => {
                let db = session.db()?;
                let r = query::execute_describe(db, table)?;
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::CreateTable {
                name,
                key_type,
                value_type,
                output_mode,
            } => {
                let r = if let Some(txn) = session.active_write() {
                    ddl::execute_create_table_in_txn(txn, name, key_type, value_type)?
                } else {
                    let db = session.db()?;
                    ddl::execute_create_table(db, name, key_type, value_type)?
                };
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::DropTable { name, output_mode } => {
                let r = if let Some(txn) = session.active_write() {
                    ddl::execute_drop_table_in_txn(txn, name)?
                } else {
                    let db = session.db()?;
                    ddl::execute_drop_table(db, name)?
                };
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::Select {
                table,
                condition,
                order_by,
                limit,
                offset,
                count_only,
                output_mode,
            } => {
                let db = session.db()?;
                let r = query::execute_select(
                    db,
                    table,
                    condition.as_ref(),
                    order_by.as_ref(),
                    *limit,
                    *offset,
                    *count_only,
                )?;
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::Insert {
                table,
                key,
                value,
                output_mode,
            } => {
                let r = if let Some(txn) = session.active_write() {
                    mutate::execute_insert_in_txn(txn, table, key, value)?
                } else {
                    let db = session.db()?;
                    mutate::execute_insert(db, table, key, value)?
                };
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::Delete {
                table,
                condition,
                output_mode,
            } => {
                let r = if let Some(txn) = session.active_write() {
                    mutate::execute_delete_in_txn(txn, table, condition)?
                } else {
                    let db = session.db()?;
                    mutate::execute_delete(db, table, condition)?
                };
                println!("{}", render_result(&r, *output_mode));
                Ok(())
            }
            Statement::Begin => session.begin_write(),
            Statement::Commit => session.commit(),
            Statement::Rollback => session.rollback(),
            Statement::Help => {
                Self::print_help_static();
                Ok(())
            }
            Statement::Clear => {
                print!("\x1b[2J\x1b[1;1H");
                Ok(())
            }
            Statement::Exit => Ok(()),
        }
    }

    pub fn print_help_static() {
        println!(
            r#"
redb-cli — 类 SQL 语法的 redb 嵌入式数据库命令行工具 (redb 4.x)

数据库命令:
  .open <路径>            打开（自动创建）redb 数据库文件
  .info                   显示数据库统计信息
  .tables                 列出所有表（等同 SHOW TABLES）
  .help                   显示本帮助

表管理 (DDL):
  SHOW TABLES;                    列出所有表
  DESCRIBE <表名>;                 查看表结构（键/值类型、行数）
  DESC <表名>;                     同上（简写）
  CREATE TABLE <表名> (<键类型>, <值类型>);
  DROP TABLE <表名>;               删除表

数据查询 (DML):
  SELECT * FROM <表名> [LIMIT <n>] [OFFSET <n>];
  SELECT COUNT(*) FROM <表名>;
  SELECT * FROM <表名>\G                  使用纵向输出（MariaDB/MySQL 风格）
  SELECT * FROM <表名> WHERE key = <值>;
  SELECT * FROM <表名> WHERE key BETWEEN <v1> AND <v2>;
  SELECT * FROM <表名> ORDER BY key [ASC|DESC];

数据写入 (DML):
  INSERT INTO <表名> VALUES (<键>, <值>);
  DELETE FROM <表名> WHERE key = <值>;

支持的数据类型 (redb 4.x):
  ┌─ 键类型（均可作为 Key）─────────────────┐
  │ I32, I64, U32, U64, I16, U16, I8, U8  │
  │ I128, U128, BOOL, STRING              │
  │ STR_KEY(&str), BYTES_KEY(&[u8])       │
  ├─ 仅值类型（不能作为键）─────────────────┤
  │ F32, F64(FLOAT), BYTES / BLOB         │
  └────────────────────────────────────────┘

事务命令:
  BEGIN;           开始写事务
  COMMIT;          提交事务
  ROLLBACK;        回滚事务

工具命令:
  HELP;            显示本帮助
  CLEAR;           清屏
  EXIT; / QUIT;    退出程序
"#
        );
    }
}
