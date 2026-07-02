use redb::{Database, MultimapTableDefinition, TableDefinition, WriteTransaction};

use crate::engine::query::QueryResult;
use crate::error::{CliError, CliResult};
use crate::parser::ast::DataType;

/// 执行 CREATE TABLE。
/// redb 4.x 键类型: 整数, bool, String, &str, &[u8], char, (), 及元组/数组
/// redb 4.x 值类型: 所有键类型 + f32, f64, &str, &[u8]
pub fn execute_create_table(
    db: &Database,
    name: &str,
    key_type: &DataType,
    value_type: &DataType,
) -> CliResult<QueryResult> {
    let txn = db.begin_write()?;
    match open_table_for_types(&txn, name, key_type, value_type) {
        Ok(()) => {
            txn.commit()?;
            Ok(create_table_message(name, key_type, value_type))
        }
        Err(err) => {
            txn.abort()?;
            Err(err)
        }
    }
}

pub fn execute_create_table_in_txn(
    txn: &WriteTransaction,
    name: &str,
    key_type: &DataType,
    value_type: &DataType,
) -> CliResult<QueryResult> {
    open_table_for_types(txn, name, key_type, value_type)?;
    Ok(create_table_message(name, key_type, value_type))
}

fn create_table_message(name: &str, key_type: &DataType, value_type: &DataType) -> QueryResult {
    QueryResult::with_message(format!(
        "表 '{}' 已创建 (键: {}, 值: {})。",
        name,
        key_type.to_type_str(),
        value_type.to_type_str()
    ))
}

fn open_table_for_types(
    txn: &WriteTransaction,
    name: &str,
    key_type: &DataType,
    value_type: &DataType,
) -> CliResult<()> {
    match (key_type, value_type) {
        // ── &str 键 (redb 4.x 新增) ──
        (DataType::Str, DataType::Str) => {
            txn.open_table(TableDefinition::<&str, &str>::new(name))?;
        }
        (DataType::Str, DataType::I32) => {
            txn.open_table(TableDefinition::<&str, i32>::new(name))?;
        }
        (DataType::Str, DataType::I64) => {
            txn.open_table(TableDefinition::<&str, i64>::new(name))?;
        }
        (DataType::Str, DataType::U32) => {
            txn.open_table(TableDefinition::<&str, u32>::new(name))?;
        }
        (DataType::Str, DataType::U64) => {
            txn.open_table(TableDefinition::<&str, u64>::new(name))?;
        }
        (DataType::Str, DataType::F64) => {
            txn.open_table(TableDefinition::<&str, f64>::new(name))?;
        }
        (DataType::Str, DataType::Bool) => {
            txn.open_table(TableDefinition::<&str, bool>::new(name))?;
        }
        (DataType::Str, DataType::String_) => {
            txn.open_table(TableDefinition::<&str, String>::new(name))?;
        }
        (DataType::Str, DataType::Bytes) => {
            txn.open_table(TableDefinition::<&str, &[u8]>::new(name))?;
        }

        // ── &[u8] 键 (redb 4.x 新增) ──
        (DataType::BytesK, DataType::Bytes) => {
            txn.open_table(TableDefinition::<&[u8], &[u8]>::new(name))?;
        }
        (DataType::BytesK, DataType::Str) => {
            txn.open_table(TableDefinition::<&[u8], &str>::new(name))?;
        }
        (DataType::BytesK, DataType::I64) => {
            txn.open_table(TableDefinition::<&[u8], i64>::new(name))?;
        }
        (DataType::BytesK, DataType::U64) => {
            txn.open_table(TableDefinition::<&[u8], u64>::new(name))?;
        }

        // ── String 键 ──
        (DataType::String_, DataType::I32) => {
            txn.open_table(TableDefinition::<String, i32>::new(name))?;
        }
        (DataType::String_, DataType::I64) => {
            txn.open_table(TableDefinition::<String, i64>::new(name))?;
        }
        (DataType::String_, DataType::U32) => {
            txn.open_table(TableDefinition::<String, u32>::new(name))?;
        }
        (DataType::String_, DataType::U64) => {
            txn.open_table(TableDefinition::<String, u64>::new(name))?;
        }
        (DataType::String_, DataType::F32) => {
            txn.open_table(TableDefinition::<String, f32>::new(name))?;
        }
        (DataType::String_, DataType::F64) => {
            txn.open_table(TableDefinition::<String, f64>::new(name))?;
        }
        (DataType::String_, DataType::Bool) => {
            txn.open_table(TableDefinition::<String, bool>::new(name))?;
        }
        (DataType::String_, DataType::Str) => {
            txn.open_table(TableDefinition::<String, &str>::new(name))?;
        }
        (DataType::String_, DataType::String_) => {
            txn.open_table(TableDefinition::<String, String>::new(name))?;
        }
        (DataType::String_, DataType::Bytes) => {
            txn.open_table(TableDefinition::<String, &[u8]>::new(name))?;
        }

        // ── I64 键 ──
        (DataType::I64, DataType::I32) => {
            txn.open_table(TableDefinition::<i64, i32>::new(name))?;
        }
        (DataType::I64, DataType::I64) => {
            txn.open_table(TableDefinition::<i64, i64>::new(name))?;
        }
        (DataType::I64, DataType::U32) => {
            txn.open_table(TableDefinition::<i64, u32>::new(name))?;
        }
        (DataType::I64, DataType::U64) => {
            txn.open_table(TableDefinition::<i64, u64>::new(name))?;
        }
        (DataType::I64, DataType::F64) => {
            txn.open_table(TableDefinition::<i64, f64>::new(name))?;
        }
        (DataType::I64, DataType::Bool) => {
            txn.open_table(TableDefinition::<i64, bool>::new(name))?;
        }
        (DataType::I64, DataType::Str) => {
            txn.open_table(TableDefinition::<i64, &str>::new(name))?;
        }
        (DataType::I64, DataType::String_) => {
            txn.open_table(TableDefinition::<i64, String>::new(name))?;
        }
        (DataType::I64, DataType::Bytes) => {
            txn.open_table(TableDefinition::<i64, &[u8]>::new(name))?;
        }

        // ── U64 键 ──
        (DataType::U64, DataType::I32) => {
            txn.open_table(TableDefinition::<u64, i32>::new(name))?;
        }
        (DataType::U64, DataType::I64) => {
            txn.open_table(TableDefinition::<u64, i64>::new(name))?;
        }
        (DataType::U64, DataType::U32) => {
            txn.open_table(TableDefinition::<u64, u32>::new(name))?;
        }
        (DataType::U64, DataType::U64) => {
            txn.open_table(TableDefinition::<u64, u64>::new(name))?;
        }
        (DataType::U64, DataType::F64) => {
            txn.open_table(TableDefinition::<u64, f64>::new(name))?;
        }
        (DataType::U64, DataType::Bool) => {
            txn.open_table(TableDefinition::<u64, bool>::new(name))?;
        }
        (DataType::U64, DataType::Str) => {
            txn.open_table(TableDefinition::<u64, &str>::new(name))?;
        }
        (DataType::U64, DataType::String_) => {
            txn.open_table(TableDefinition::<u64, String>::new(name))?;
        }
        (DataType::U64, DataType::Bytes) => {
            txn.open_table(TableDefinition::<u64, &[u8]>::new(name))?;
        }

        // ── I32 键 ──
        (DataType::I32, DataType::I32) => {
            txn.open_table(TableDefinition::<i32, i32>::new(name))?;
        }
        (DataType::I32, DataType::I64) => {
            txn.open_table(TableDefinition::<i32, i64>::new(name))?;
        }
        (DataType::I32, DataType::Str) => {
            txn.open_table(TableDefinition::<i32, &str>::new(name))?;
        }
        (DataType::I32, DataType::Bool) => {
            txn.open_table(TableDefinition::<i32, bool>::new(name))?;
        }

        // ── U32 键 ──
        (DataType::U32, DataType::U32) => {
            txn.open_table(TableDefinition::<u32, u32>::new(name))?;
        }
        (DataType::U32, DataType::U64) => {
            txn.open_table(TableDefinition::<u32, u64>::new(name))?;
        }
        (DataType::U32, DataType::Str) => {
            txn.open_table(TableDefinition::<u32, &str>::new(name))?;
        }

        // ── Bool 键 ──
        (DataType::Bool, DataType::I64) => {
            txn.open_table(TableDefinition::<bool, i64>::new(name))?;
        }
        (DataType::Bool, DataType::Str) => {
            txn.open_table(TableDefinition::<bool, &str>::new(name))?;
        }
        (DataType::Bool, DataType::Bool) => {
            txn.open_table(TableDefinition::<bool, bool>::new(name))?;
        }

        // 不允许的组合
        (DataType::F32, _) | (DataType::F64, _) => {
            return Err(CliError::UnsupportedType(
                "FLOAT 类型不能作为键类型（redb 不支持 f32/f64 实现 Key）。".into(),
            ));
        }
        (DataType::Bytes, _) => {
            return Err(CliError::UnsupportedType(
                "BYTES/BLOB 作为键类型时请使用 BYTES_KEY（对应 redb 的 &[u8]）。".into(),
            ));
        }
        _ => {
            return Err(CliError::UnsupportedType(format!(
                "暂不支持的类型组合: {} 键 + {} 值",
                key_type.to_type_str(),
                value_type.to_type_str()
            )));
        }
    }
    Ok(())
}

/// 执行 DROP TABLE
pub fn execute_drop_table(db: &Database, name: &str) -> CliResult<QueryResult> {
    let txn = db.begin_write()?;
    let result = delete_table_in_txn(&txn, name);
    match result {
        Ok(message) => {
            txn.commit()?;
            Ok(message)
        }
        Err(err) => {
            txn.abort()?;
            Err(err)
        }
    }
}

pub fn execute_drop_table_in_txn(txn: &WriteTransaction, name: &str) -> CliResult<QueryResult> {
    delete_table_in_txn(txn, name)
}

fn delete_table_in_txn(txn: &WriteTransaction, name: &str) -> CliResult<QueryResult> {
    let handle = TableDefinition::<u64, u64>::new(name);
    match txn.delete_table(handle) {
        Ok(true) => Ok(QueryResult::with_message(format!("表 '{}' 已删除。", name))),
        Ok(false) => {
            let mm = MultimapTableDefinition::<&str, &str>::new(name);
            match txn.delete_multimap_table(mm) {
                Ok(true) => Ok(QueryResult::with_message(format!(
                    "Multimap 表 '{}' 已删除。",
                    name
                ))),
                Ok(false) => Err(CliError::TableNotFound(name.to_string())),
                Err(e) => Err(CliError::Table(e)),
            }
        }
        Err(e) => Err(CliError::Table(e)),
    }
}
