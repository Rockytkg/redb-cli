use thiserror::Error;

/// 统一错误类型
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CliError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("redb 错误: {0}")]
    Redb(#[from] redb::Error),

    #[error("redb 事务错误: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("redb 表错误: {0}")]
    Table(#[from] redb::TableError),

    #[error("redb 存储错误: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("redb 数据库错误: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("redb 提交错误: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("redb 压缩错误: {0}")]
    Compaction(#[from] redb::CompactionError),

    #[error("引擎错误: {0}")]
    Engine(String),

    #[error("未打开数据库")]
    NoDatabase,

    #[error("表不存在: {0}")]
    TableNotFound(String),

    #[error("不支持的类型: {0}")]
    UnsupportedType(String),

    #[error("类型不匹配: {0}")]
    TypeMismatch(String),

    #[error("无效的值: {0}")]
    InvalidValue(String),
}

pub type CliResult<T> = Result<T, CliError>;
