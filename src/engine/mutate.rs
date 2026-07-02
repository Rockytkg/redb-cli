use redb::{Database, Key, TableDefinition, Value, WriteTransaction};
use std::borrow::Borrow;

use crate::engine::query::QueryResult;
use crate::error::{CliError, CliResult};
use crate::parser::ast::Literal;

fn lit_to_string(v: &Literal) -> String {
    match v {
        Literal::Int(i) => i.to_string(),
        Literal::Float(f) => f.to_string(),
        Literal::String(s) => s.clone(),
        Literal::Null => String::new(),
    }
}
fn to_i64(v: &Literal) -> Option<i64> {
    match v {
        Literal::Int(i) => Some(*i),
        Literal::Float(f) => Some(*f as i64),
        Literal::String(s) => s.parse().ok(),
        Literal::Null => Some(0),
    }
}
fn to_u64(v: &Literal) -> Option<u64> {
    match v {
        Literal::Int(i) if *i >= 0 => Some(*i as u64),
        Literal::Float(f) if *f >= 0.0 => Some(*f as u64),
        Literal::String(s) => s.parse().ok(),
        Literal::Null => Some(0),
        _ => None,
    }
}

/// 执行 INSERT。依次尝试常见类型组合，命中即停。
pub fn execute_insert(
    db: &Database,
    table_name: &str,
    key: &Literal,
    value: &Literal,
) -> CliResult<QueryResult> {
    execute_insert_inner(WriteTarget::Database(db), table_name, key, value)
}

pub fn execute_insert_in_txn(
    txn: &WriteTransaction,
    table_name: &str,
    key: &Literal,
    value: &Literal,
) -> CliResult<QueryResult> {
    execute_insert_inner(WriteTarget::Transaction(txn), table_name, key, value)
}

fn execute_insert_inner(
    target: WriteTarget<'_>,
    table_name: &str,
    key: &Literal,
    value: &Literal,
) -> CliResult<QueryResult> {
    let sk = lit_to_string(key);
    let sv = lit_to_string(value);

    // ── &str 键 (redb 4.x 新增) ──
    if let Some(v) = to_i64(value) {
        if_ok!(target.try_insert_str_key(table_name, sk.as_str(), &v));
    }
    if let Some(v) = to_u64(value) {
        if_ok!(target.try_insert_str_key(table_name, sk.as_str(), &v));
    }
    if_ok!(target.try_insert_str_key_str_val(table_name, sk.as_str(), sv.as_str()));

    // ── String 键 ──
    if let Some(v) = to_i64(value) {
        if_ok!(target.try_insert::<String, i64>(table_name, &sk, &v));
    }
    if let Some(v) = to_u64(value) {
        if_ok!(target.try_insert::<String, u64>(table_name, &sk, &v));
    }
    if_ok!(target.try_insert::<String, String>(table_name, &sk, &sv));
    {
        let b = matches!(value, Literal::Int(i) if *i != 0);
        if_ok!(target.try_insert::<String, bool>(table_name, &sk, &b));
    }
    if let Literal::Float(f) = value {
        if_ok!(target.try_insert::<String, f64>(table_name, &sk, f));
    }
    // String 键 × &str 值
    if_ok!(target.try_insert_str_val::<String>(table_name, &sk, &sv));

    // ── i64 键 ──
    if let Some(k) = to_i64(key) {
        if let Some(v) = to_i64(value) {
            if_ok!(target.try_insert::<i64, i64>(table_name, &k, &v));
        }
        if let Some(v) = to_u64(value) {
            if_ok!(target.try_insert::<i64, u64>(table_name, &k, &v));
        }
        if_ok!(target.try_insert::<i64, String>(table_name, &k, &sv));
        if_ok!(target.try_insert_str_val::<i64>(table_name, &k, &sv));
    }

    // ── u64 键 ──
    if let Some(k) = to_u64(key) {
        if let Some(v) = to_u64(value) {
            if_ok!(target.try_insert::<u64, u64>(table_name, &k, &v));
        }
        if let Some(v) = to_i64(value) {
            if_ok!(target.try_insert::<u64, i64>(table_name, &k, &v));
        }
        if_ok!(target.try_insert::<u64, String>(table_name, &k, &sv));
        if_ok!(target.try_insert_str_val::<u64>(table_name, &k, &sv));
    }

    Err(CliError::Engine(format!(
        "无法插入数据到 '{}'。请使用 DESCRIBE 查看表结构。",
        table_name
    )))
}

#[derive(Clone, Copy)]
enum WriteTarget<'a> {
    Database(&'a Database),
    Transaction(&'a WriteTransaction),
}

impl WriteTarget<'_> {
    fn try_insert<K, V>(self, table: &str, k: &K, v: &V) -> CliResult<QueryResult>
    where
        K: Key + 'static,
        V: Value + 'static,
        for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
        for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
    {
        match self {
            WriteTarget::Database(db) => {
                let txn = db.begin_write()?;
                let result = insert_on_txn::<K, V>(&txn, table, k, v);
                finish_auto_txn(txn, result)
            }
            WriteTarget::Transaction(txn) => insert_on_txn::<K, V>(txn, table, k, v),
        }
    }

    fn try_insert_str_val<K: Key + 'static>(
        self,
        table: &str,
        k: &K,
        v: &str,
    ) -> CliResult<QueryResult>
    where
        for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
    {
        match self {
            WriteTarget::Database(db) => {
                let txn = db.begin_write()?;
                let result = insert_str_val_on_txn::<K>(&txn, table, k, v);
                finish_auto_txn(txn, result)
            }
            WriteTarget::Transaction(txn) => insert_str_val_on_txn::<K>(txn, table, k, v),
        }
    }

    fn try_insert_str_key<V: Value + 'static>(
        self,
        table: &str,
        key: &str,
        v: &V,
    ) -> CliResult<QueryResult>
    where
        for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
    {
        match self {
            WriteTarget::Database(db) => {
                let txn = db.begin_write()?;
                let result = insert_str_key_on_txn::<V>(&txn, table, key, v);
                finish_auto_txn(txn, result)
            }
            WriteTarget::Transaction(txn) => insert_str_key_on_txn::<V>(txn, table, key, v),
        }
    }

    fn try_insert_str_key_str_val(
        self,
        table: &str,
        key: &str,
        val: &str,
    ) -> CliResult<QueryResult> {
        match self {
            WriteTarget::Database(db) => {
                let txn = db.begin_write()?;
                let result = insert_str_key_str_val_on_txn(&txn, table, key, val);
                finish_auto_txn(txn, result)
            }
            WriteTarget::Transaction(txn) => insert_str_key_str_val_on_txn(txn, table, key, val),
        }
    }
}

fn finish_auto_txn(
    txn: WriteTransaction,
    result: CliResult<QueryResult>,
) -> CliResult<QueryResult> {
    match result {
        Ok(result) => {
            txn.commit()?;
            Ok(result)
        }
        Err(err) => {
            txn.abort()?;
            Err(err)
        }
    }
}

// ── 泛型 INSERT 辅助函数 ──

fn insert_on_txn<K, V>(txn: &WriteTransaction, table: &str, k: &K, v: &V) -> CliResult<QueryResult>
where
    K: Key + 'static,
    V: Value + 'static,
    for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
    for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<K, V>::new(table);
    let mut t = txn
        .open_table(def)
        .map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(k, v)?;
    Ok(QueryResult::with_message(format!(
        "已插入 1 行到 '{}'。",
        table
    )))
}

/// 值类型为 &str 的 INSERT
fn insert_str_val_on_txn<K: Key + 'static>(
    txn: &WriteTransaction,
    table: &str,
    k: &K,
    v: &str,
) -> CliResult<QueryResult>
where
    for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<K, &str>::new(table);
    let mut t = txn
        .open_table(def)
        .map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(k, v)?;
    Ok(QueryResult::with_message(format!(
        "已插入 1 行到 '{}'。",
        table
    )))
}

/// 键类型为 &str 的 INSERT（redb 4.x 新增能力）
fn insert_str_key_on_txn<V: Value + 'static>(
    txn: &WriteTransaction,
    table: &str,
    key: &str,
    v: &V,
) -> CliResult<QueryResult>
where
    for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<&str, V>::new(table);
    let mut t = txn
        .open_table(def)
        .map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(key, v)?;
    Ok(QueryResult::with_message(format!(
        "已插入 1 行到 '{}'。",
        table
    )))
}

/// 键值均为 &str 的 INSERT（redb 4.x 新增能力）
fn insert_str_key_str_val_on_txn(
    txn: &WriteTransaction,
    table: &str,
    key: &str,
    val: &str,
) -> CliResult<QueryResult> {
    let def = TableDefinition::<&str, &str>::new(table);
    let mut t = txn
        .open_table(def)
        .map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(key, val)?;
    Ok(QueryResult::with_message(format!(
        "已插入 1 行到 '{}'。",
        table
    )))
}

// ── DELETE ──

pub fn execute_delete(
    db: &Database,
    table_name: &str,
    condition: &crate::parser::ast::Condition,
) -> CliResult<QueryResult> {
    execute_delete_inner(WriteTarget::Database(db), table_name, condition)
}

pub fn execute_delete_in_txn(
    txn: &WriteTransaction,
    table_name: &str,
    condition: &crate::parser::ast::Condition,
) -> CliResult<QueryResult> {
    execute_delete_inner(WriteTarget::Transaction(txn), table_name, condition)
}

fn execute_delete_inner(
    target: WriteTarget<'_>,
    table_name: &str,
    condition: &crate::parser::ast::Condition,
) -> CliResult<QueryResult> {
    match condition {
        crate::parser::ast::Condition::Equals(_, key_val) => match key_val {
            Literal::Int(i) => {
                let k = *i;
                let sk = k.to_string();
                if_ok!(target.try_delete::<i64, &str>(table_name, &k));
                if_ok!(target.try_delete::<i64, i64>(table_name, &k));
                if_ok!(target.try_delete::<i64, String>(table_name, &k));
                if k >= 0 {
                    if_ok!(target.try_delete::<u64, &str>(table_name, &(k as u64)));
                    if_ok!(target.try_delete::<u64, String>(table_name, &(k as u64)));
                }
                if_ok!(target.try_delete_str_key(table_name, sk.as_str()));
                if_ok!(target.try_delete::<String, &str>(table_name, &sk));
                if_ok!(target.try_delete::<String, String>(table_name, &sk));
                if_ok!(target.try_delete::<String, i64>(table_name, &sk));
            }
            Literal::String(s) => {
                if_ok!(target.try_delete_str_key(table_name, s.as_str()));
                if_ok!(target.try_delete::<String, &str>(table_name, s));
                if_ok!(target.try_delete::<String, String>(table_name, s));
                if_ok!(target.try_delete::<String, i64>(table_name, s));
            }
            _ => return Err(CliError::Engine("DELETE 仅支持整数或字符串键。".into())),
        },
        _ => return Err(CliError::Engine("DELETE 仅支持 WHERE key = <值>。".into())),
    }
    Err(CliError::Engine(format!(
        "无法在表 '{}' 中找到匹配的类型进行 DELETE。",
        table_name
    )))
}

impl WriteTarget<'_> {
    fn try_delete<K, V>(self, table: &str, key: &K) -> CliResult<QueryResult>
    where
        K: Key + 'static,
        V: Value + 'static,
        for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
    {
        match self {
            WriteTarget::Database(db) => {
                let txn = db.begin_write()?;
                let result = delete_on_txn::<K, V>(&txn, table, key);
                finish_auto_txn(txn, result)
            }
            WriteTarget::Transaction(txn) => delete_on_txn::<K, V>(txn, table, key),
        }
    }

    fn try_delete_str_key(self, table: &str, key: &str) -> CliResult<QueryResult> {
        // Try various value types with &str key
        if_ok!(self.try_delete_str_key_val::<&str>(table, key));
        if_ok!(self.try_delete_str_key_val::<i64>(table, key));
        if_ok!(self.try_delete_str_key_val::<u64>(table, key));
        if_ok!(self.try_delete_str_key_val::<String>(table, key));
        Err(CliError::TypeMismatch(format!("表 '{}'", table)))
    }

    fn try_delete_str_key_val<V: Value + 'static>(
        self,
        table: &str,
        key: &str,
    ) -> CliResult<QueryResult> {
        match self {
            WriteTarget::Database(db) => {
                let txn = db.begin_write()?;
                let result = delete_str_key_val_on_txn::<V>(&txn, table, key);
                finish_auto_txn(txn, result)
            }
            WriteTarget::Transaction(txn) => delete_str_key_val_on_txn::<V>(txn, table, key),
        }
    }
}

fn delete_on_txn<K, V>(txn: &WriteTransaction, table: &str, key: &K) -> CliResult<QueryResult>
where
    K: Key + 'static,
    V: Value + 'static,
    for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
{
    let existed = {
        let def = TableDefinition::<K, V>::new(table);
        let mut t = txn
            .open_table(def)
            .map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
        let removed = t.remove(key)?;
        removed.is_some()
    };
    Ok(QueryResult::with_message(if existed {
        format!("已从 '{}' 删除 1 行。", table)
    } else {
        format!("在 '{}' 中未找到匹配的行。", table)
    }))
}

/// 带 &str 键的泛型值类型 DELETE
fn delete_str_key_val_on_txn<V: Value + 'static>(
    txn: &WriteTransaction,
    table: &str,
    key: &str,
) -> CliResult<QueryResult> {
    let existed = {
        let def = TableDefinition::<&str, V>::new(table);
        let mut t = txn
            .open_table(def)
            .map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
        let removed = t.remove(key)?;
        removed.is_some()
    };
    Ok(QueryResult::with_message(if existed {
        format!("已从 '{}' 删除 1 行。", table)
    } else {
        format!("在 '{}' 中未找到匹配的行。", table)
    }))
}

macro_rules! if_ok {
    ($e:expr) => {
        match $e {
            Ok(r) => return Ok(r),
            Err(_) => {}
        }
    };
}
use if_ok;
