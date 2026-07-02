use redb::{Database, WriteTransaction};
use std::path::PathBuf;

use crate::error::{CliError, CliResult};

/// 数据库会话管理
pub struct Session {
    db: Option<Database>,
    path: Option<PathBuf>,
    write_txn: Option<WriteTransaction>,
}

impl Session {
    pub fn new() -> Self {
        Session {
            db: None,
            path: None,
            write_txn: None,
        }
    }

    /// 打开已有 redb 数据库文件
    pub fn open(&mut self, path: &str) -> CliResult<()> {
        self.close()?;
        let db = Database::open(path)?;
        self.path = Some(PathBuf::from(path));
        self.db = Some(db);
        println!("已连接到: {}", self.filename());
        Ok(())
    }

    /// 创建新的 redb 数据库文件
    pub fn create(&mut self, path: &str) -> CliResult<()> {
        self.close()?;
        let db = Database::create(path)?;
        self.path = Some(PathBuf::from(path));
        self.db = Some(db);
        println!("已创建并连接: {}", self.filename());
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.db.is_some()
    }

    pub fn db(&self) -> CliResult<&Database> {
        self.db.as_ref().ok_or(CliError::NoDatabase)
    }

    /// 返回仅文件名的提示符字符串
    pub fn prompt_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| format!("redb({})", n))
            .unwrap_or_else(|| "redb(no db)".to_string())
    }

    fn filename(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
    }

    /// 获取文件路径字符串
    #[allow(dead_code)]
    pub fn path_str(&self) -> Option<&str> {
        self.path.as_ref().and_then(|p| p.to_str())
    }

    /// 开始写事务 (由 BEGIN 触发)
    pub fn begin_write(&mut self) -> CliResult<()> {
        if self.write_txn.is_some() {
            return Err(CliError::Engine(
                "已存在活动事务，请先 COMMIT 或 ROLLBACK。".to_string(),
            ));
        }
        let db = self.db()?;
        let txn = db.begin_write()?;
        self.write_txn = Some(txn);
        println!("写事务已开始。");
        Ok(())
    }

    pub fn commit(&mut self) -> CliResult<()> {
        match self.write_txn.take() {
            Some(txn) => {
                txn.commit()?;
                println!("事务已提交。");
                Ok(())
            }
            None => Err(CliError::Engine("没有活动的事务可以提交。".to_string())),
        }
    }

    pub fn rollback(&mut self) -> CliResult<()> {
        match self.write_txn.take() {
            Some(txn) => {
                txn.abort()?;
                println!("事务已回滚。");
                Ok(())
            }
            None => Err(CliError::Engine("没有活动的事务可以回滚。".to_string())),
        }
    }

    pub fn has_active_write(&self) -> bool {
        self.write_txn.is_some()
    }

    pub fn active_write(&self) -> Option<&WriteTransaction> {
        self.write_txn.as_ref()
    }

    pub fn close(&mut self) -> CliResult<()> {
        if let Some(txn) = self.write_txn.take() {
            let _ = txn.abort();
        }
        self.db = None;
        self.path = None;
        Ok(())
    }
}
