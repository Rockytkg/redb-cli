//! 数据查询 (SELECT / DESCRIBE)。
//!
//! 通过 `for_all_table_types!` 和 `for_all_multimap_types!` 遍历
//! 全部受支持的类型组合，保证与 INSERT / DELETE 覆盖范围一致。

use redb::{
    Database, MultimapTableDefinition, MultimapTableHandle, ReadableDatabase,
    ReadableMultimapTable, ReadableTable, ReadableTableMetadata, TableDefinition, TableHandle,
};

use crate::error::{CliError, CliResult};
use crate::parser::ast::{Condition, Literal, OrderBy, OrderDirection};
use crate::for_all_table_types;
use crate::for_all_multimap_types;

pub type Row = Vec<String>;

#[derive(Debug)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
    pub message: Option<String>,
}

impl QueryResult {
    pub fn from_rows(rows: Vec<Row>) -> Self {
        QueryResult {
            columns: vec!["键".into(), "值".into()],
            rows,
            message: None,
        }
    }
    pub fn with_message(msg: String) -> Self {
        QueryResult {
            columns: vec![],
            rows: vec![],
            message: Some(msg),
        }
    }
    pub fn count_only(count: u64) -> Self {
        QueryResult {
            columns: vec!["行数".into()],
            rows: vec![vec![count.to_string()]],
            message: None,
        }
    }
}

// ── 检查表是否存在 ──

fn table_exists(db: &Database, table: &str) -> bool {
    if let Ok(txn) = db.begin_read() {
        if let Ok(iter) = txn.list_tables() {
            for h in iter {
                if h.name() == table {
                    return true;
                }
            }
        }
        if let Ok(iter) = txn.list_multimap_tables() {
            for h in iter {
                if h.name() == table {
                    return true;
                }
            }
        }
    }
    false
}

// ── SELECT ─────────────────────────────────────────────────────────────────

pub fn execute_select(
    db: &Database,
    table_name: &str,
    condition: Option<&Condition>,
    order_by: Option<&OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
    count_only: bool,
) -> CliResult<QueryResult> {
    validate_select_options(condition, order_by)?;

    // 遍历全部表类型组合——与 INSERT / DELETE 使用同一份权威列表
    macro_rules! attempt {
        ($K:ty, $V:ty) => {
            match try_open_typed::<$K, $V>(
                db, table_name, condition, order_by, limit, offset, count_only,
            ) {
                Ok(r) => return Ok(r),
                Err(CliError::TableNotFound(_)) => {
                    return Err(CliError::TableNotFound(table_name.into()))
                }
                Err(_) => {}
            }
        };
    }

    for_all_table_types!(attempt);

    // 再遍历 Multimap 类型组合
    macro_rules! attempt_mm {
        ($K:ty, $V:ty) => {
            match mm_open_typed::<$K, $V>(
                db, table_name, condition, order_by, limit, offset, count_only,
            ) {
                Ok(r) => return Ok(r),
                Err(CliError::TableNotFound(_)) => {
                    return Err(CliError::TableNotFound(table_name.into()))
                }
                Err(_) => {}
            }
        };
    }

    for_all_multimap_types!(attempt_mm);

    if table_exists(db, table_name) {
        Err(CliError::Engine(format!(
            "表 '{}' 存在，但其键/值类型组合未被当前版本支持。\n\
             该表可能使用了复合类型（如元组），请尝试用程序代码读取。",
            table_name
        )))
    } else {
        Err(CliError::TableNotFound(table_name.to_string()))
    }
}

fn try_open_typed<K: redb::Key + 'static, V: redb::Value + 'static>(
    db: &Database,
    table: &str,
    condition: Option<&Condition>,
    order_by: Option<&OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
    count_only: bool,
) -> CliResult<QueryResult> {
    let txn = db.begin_read()?;
    let t = match txn.open_table(TableDefinition::<K, V>::new(table)) {
        Ok(t) => t,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(CliError::TableNotFound(table.into()))
        }
        Err(_) => return Err(CliError::Engine("类型不匹配".into())),
    };
    if count_only && condition.is_none() {
        return Ok(QueryResult::count_only(t.len()?));
    }
    let mut rows: Vec<ScannedRow> = Vec::new();
    for item in t.iter()? {
        let (k, v) = item?;
        let row = ScannedRow::new(clean_debug(k.value()), clean_debug(v.value()));
        if matches_condition(&row, condition) {
            rows.push(row);
        }
    }
    Ok(rows_to_result(rows, order_by, limit, offset, count_only))
}

fn mm_open_typed<K: redb::Key + 'static, V: redb::Key + 'static>(
    db: &Database,
    table: &str,
    condition: Option<&Condition>,
    order_by: Option<&OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
    count_only: bool,
) -> CliResult<QueryResult> {
    let txn = db.begin_read()?;
    let t = match txn.open_multimap_table(MultimapTableDefinition::<K, V>::new(table)) {
        Ok(t) => t,
        Err(redb::TableError::TableDoesNotExist(_)) => {
            return Err(CliError::TableNotFound(table.into()))
        }
        Err(_) => return Err(CliError::Engine("类型不匹配".into())),
    };
    if count_only && condition.is_none() {
        let mut total = 0u64;
        for r in t.iter()? {
            let (_, vals) = r?;
            total += vals.count() as u64;
        }
        return Ok(QueryResult::count_only(total));
    }
    let mut rows: Vec<ScannedRow> = Vec::new();
    for r in t.iter()? {
        let (key, vals) = r?;
        for v in vals {
            let v = v?;
            let row = ScannedRow::new(clean_debug(key.value()), clean_debug(v.value()));
            if matches_condition(&row, condition) {
                rows.push(row);
            }
        }
    }
    Ok(rows_to_result(rows, order_by, limit, offset, count_only))
}

// ── 辅助类型与逻辑 ──

#[derive(Debug)]
struct ScannedRow {
    key: String,
    value: String,
}

impl ScannedRow {
    fn new(key: String, value: String) -> Self {
        Self { key, value }
    }
    fn into_row(self) -> Row {
        vec![self.key, self.value]
    }
}

fn validate_select_options(
    condition: Option<&Condition>,
    order_by: Option<&OrderBy>,
) -> CliResult<()> {
    if let Some(condition) = condition {
        let column = match condition {
            Condition::Equals(column, _)
            | Condition::NotEquals(column, _)
            | Condition::GreaterThan(column, _)
            | Condition::GreaterEquals(column, _)
            | Condition::LessThan(column, _)
            | Condition::LessEquals(column, _)
            | Condition::Between(column, _, _) => column,
        };
        ensure_key_column(column)?;
    }
    if let Some(order_by) = order_by {
        ensure_key_column(&order_by.column)?;
    }
    Ok(())
}

fn ensure_key_column(column: &str) -> CliResult<()> {
    if column.eq_ignore_ascii_case("key") || column == "键" {
        Ok(())
    } else {
        Err(CliError::Engine(format!(
            "仅支持按键列查询/排序，收到列 '{}'.",
            column
        )))
    }
}

fn rows_to_result(
    mut rows: Vec<ScannedRow>,
    order_by: Option<&OrderBy>,
    limit: Option<u64>,
    offset: Option<u64>,
    count_only: bool,
) -> QueryResult {
    if matches!(order_by.map(|o| &o.direction), Some(OrderDirection::Desc)) {
        rows.reverse();
    }
    if count_only {
        return QueryResult::count_only(rows.len() as u64);
    }
    let offset = offset.unwrap_or(0) as usize;
    let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
    let rows = rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(ScannedRow::into_row)
        .collect();
    QueryResult::from_rows(rows)
}

fn matches_condition(row: &ScannedRow, condition: Option<&Condition>) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    match condition {
        Condition::Equals(_, value) => {
            compare_literal(&row.key, value) == Some(std::cmp::Ordering::Equal)
        }
        Condition::NotEquals(_, value) => {
            compare_literal(&row.key, value) != Some(std::cmp::Ordering::Equal)
        }
        Condition::GreaterThan(_, value) => {
            compare_literal(&row.key, value) == Some(std::cmp::Ordering::Greater)
        }
        Condition::GreaterEquals(_, value) => matches!(
            compare_literal(&row.key, value),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
        Condition::LessThan(_, value) => {
            compare_literal(&row.key, value) == Some(std::cmp::Ordering::Less)
        }
        Condition::LessEquals(_, value) => matches!(
            compare_literal(&row.key, value),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        Condition::Between(_, start, end) => matches!(
            (
                compare_literal(&row.key, start),
                compare_literal(&row.key, end)
            ),
            (
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal),
                Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
            )
        ),
    }
}

fn compare_literal(key: &str, literal: &Literal) -> Option<std::cmp::Ordering> {
    match literal {
        Literal::Int(value) => key
            .parse::<i128>()
            .ok()
            .map(|key| key.cmp(&(*value as i128))),
        Literal::Float(value) => key
            .parse::<f64>()
            .ok()
            .and_then(|key| key.partial_cmp(value)),
        Literal::String(value) => Some(key.cmp(value)),
        Literal::Null => Some(key.cmp("")),
    }
}

fn clean_debug(v: impl std::fmt::Debug) -> String {
    let s = format!("{:?}", v);
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        s[1..s.len() - 1].to_string()
    } else {
        s
    }
}

// ── DESCRIBE ───────────────────────────────────────────────────────────────

pub fn execute_describe(db: &Database, table_name: &str) -> CliResult<QueryResult> {
    macro_rules! try_d {
        ($K:ty, $V:ty) => {
            match describe_typed::<$K, $V>(db, table_name) {
                Ok(r) => return Ok(r),
                Err(_) => {}
            }
        };
    }

    for_all_table_types!(try_d);

    if !table_exists(db, table_name) {
        return Err(CliError::TableNotFound(table_name.into()));
    }
    Err(CliError::Engine(format!(
        "表 '{}' 存在，但其类型组合无法识别。可能使用了复合类型（元组/数组/Option 等）。",
        table_name
    )))
}

fn describe_typed<K: redb::Key + 'static, V: redb::Value + 'static>(
    db: &Database,
    table: &str,
) -> CliResult<QueryResult> {
    let txn = db.begin_read()?;
    let t = txn.open_table(TableDefinition::<K, V>::new(table))?;
    let count = t.len()?;
    Ok(QueryResult::with_message(format!(
        "表: {}\n  键类型:    {}\n  值类型:    {}\n  行数:      {}",
        table,
        std::any::type_name::<K>(),
        std::any::type_name::<V>(),
        count
    )))
}

// ── 测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{ddl, mutate};
    use crate::parser::ast::{DataType, Literal};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DB_ID: AtomicU64 = AtomicU64::new(1);

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "redb_cli_{}_{}_{}.redb",
            name,
            std::process::id(),
            NEXT_DB_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn setup_users(path: &PathBuf) -> Database {
        let _ = fs::remove_file(path);
        let db = Database::create(path).unwrap();
        ddl::execute_create_table(&db, "users", &DataType::I64, &DataType::String_).unwrap();
        mutate::execute_insert(
            &db,
            "users",
            &Literal::Int(1),
            &Literal::String("Alice".into()),
        )
        .unwrap();
        mutate::execute_insert(
            &db,
            "users",
            &Literal::Int(2),
            &Literal::String("Bob".into()),
        )
        .unwrap();
        mutate::execute_insert(
            &db,
            "users",
            &Literal::Int(3),
            &Literal::String("Charlie".into()),
        )
        .unwrap();
        db
    }

    #[test]
    fn select_applies_where_order_limit_and_count() {
        let path = temp_db_path("query");
        let db = setup_users(&path);
        let condition = Condition::GreaterEquals("key".into(), Literal::Int(2));
        let order_by = OrderBy {
            column: "key".into(),
            direction: OrderDirection::Desc,
        };

        let rows = execute_select(
            &db,
            "users",
            Some(&condition),
            Some(&order_by),
            Some(1),
            None,
            false,
        )
        .unwrap();
        assert_eq!(rows.columns, vec!["键", "值"]);
        assert_eq!(
            rows.rows,
            vec![vec!["3".to_string(), "Charlie".to_string()]]
        );

        let count = execute_select(&db, "users", Some(&condition), None, None, None, true).unwrap();
        assert_eq!(count.columns, vec!["行数"]);
        assert_eq!(count.rows, vec![vec!["2".to_string()]]);

        drop(db);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn insert_can_participate_in_explicit_write_transaction() {
        let path = temp_db_path("txn");
        let _ = fs::remove_file(&path);
        let db = Database::create(&path).unwrap();
        ddl::execute_create_table(&db, "users", &DataType::I64, &DataType::String_).unwrap();

        let txn = db.begin_write().unwrap();
        mutate::execute_insert_in_txn(
            &txn,
            "users",
            &Literal::Int(1),
            &Literal::String("Alice".into()),
        )
        .unwrap();
        txn.commit().unwrap();

        let rows = execute_select(&db, "users", None, None, None, None, false).unwrap();
        assert_eq!(rows.rows, vec![vec!["1".to_string(), "Alice".to_string()]]);

        drop(db);
        let _ = fs::remove_file(path);
    }
}
