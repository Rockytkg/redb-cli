//! 数据变更 (INSERT / DELETE)。
//!
//! 类型覆盖策略：
//! - `&str`/`&[u8]` 键值因 HRTB+生命周期约束需专用函数处理
//! - 自有类型通过 `OwnedFromLit` trait + `for_all_owned_table_types!` 宏统一分派
//! - 两者合计覆盖 `for_all_table_types!` 中的全部组合

use redb::{Database, Key, TableDefinition, Value, WriteTransaction};
use std::borrow::Borrow;

use crate::engine::query::QueryResult;
use crate::error::{CliError, CliResult};
use crate::parser::ast::Literal;
use crate::for_all_owned_table_types;

// ── OwnedFromLit: Literal → 自有类型转换 ─────────────────────────────────

trait OwnedFromLit: Sized + 'static {
    fn from_lit(lit: &Literal) -> Option<Self>;
}

fn lit_to_string(v: &Literal) -> String {
    match v {
        Literal::Int(i) => i.to_string(),
        Literal::Float(f) => f.to_string(),
        Literal::String(s) => s.clone(),
        Literal::Null => String::new(),
    }
}

fn lit_to_i64(v: &Literal) -> Option<i64> {
    match v {
        Literal::Int(i) => Some(*i),
        Literal::Float(f) => Some(*f as i64),
        Literal::String(s) => s.parse().ok(),
        Literal::Null => Some(0),
    }
}

fn lit_to_u64(v: &Literal) -> Option<u64> {
    match v {
        Literal::Int(i) if *i >= 0 => Some(*i as u64),
        Literal::Float(f) if *f >= 0.0 => Some(*f as u64),
        Literal::String(s) => s.parse().ok(),
        Literal::Null => Some(0),
        _ => None,
    }
}

fn lit_to_f64(v: &Literal) -> Option<f64> {
    match v {
        Literal::Int(i) => Some(*i as f64),
        Literal::Float(f) => Some(*f),
        Literal::String(s) => s.parse().ok(),
        Literal::Null => Some(0.0),
    }
}

fn lit_to_bool(v: &Literal) -> Option<bool> {
    match v {
        Literal::Int(0) => Some(false),
        Literal::Int(_) => Some(true),
        Literal::Float(f) if *f == 0.0 => Some(false),
        Literal::Float(_) => Some(true),
        Literal::String(s) => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" | "" => Some(false),
            _ => None,
        },
        Literal::Null => Some(false),
    }
}

impl OwnedFromLit for String { fn from_lit(l: &Literal) -> Option<Self> { Some(lit_to_string(l)) } }
impl OwnedFromLit for i64 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_i64(l) } }
impl OwnedFromLit for u64 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_u64(l) } }
impl OwnedFromLit for f64 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_f64(l) } }
impl OwnedFromLit for bool { fn from_lit(l: &Literal) -> Option<Self> { lit_to_bool(l) } }
impl OwnedFromLit for i32 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_i64(l).map(|v| v as i32) } }
impl OwnedFromLit for u32 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_u64(l).map(|v| v as u32) } }
impl OwnedFromLit for f32 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_f64(l).map(|v| v as f32) } }
impl OwnedFromLit for i128 { fn from_lit(l: &Literal) -> Option<Self> {
    match l { Literal::Int(i) => Some(*i as i128), Literal::Float(f) => Some(*f as i128), Literal::String(s) => s.parse().ok(), Literal::Null => Some(0) }
}}
impl OwnedFromLit for u128 { fn from_lit(l: &Literal) -> Option<Self> {
    match l { Literal::Int(i) if *i >= 0 => Some(*i as u128), Literal::Float(f) if *f >= 0.0 => Some(*f as u128), Literal::String(s) => s.parse().ok(), Literal::Null => Some(0), _ => None }
}}
impl OwnedFromLit for i16 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_i64(l).map(|v| v as i16) } }
impl OwnedFromLit for u16 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_u64(l).map(|v| v as u16) } }
impl OwnedFromLit for i8 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_i64(l).map(|v| v as i8) } }
impl OwnedFromLit for u8 { fn from_lit(l: &Literal) -> Option<Self> { lit_to_u64(l).map(|v| v as u8) } }

// ── INSERT ────────────────────────────────────────────────────────────────

pub fn execute_insert(db: &Database, table_name: &str, key: &Literal, value: &Literal) -> CliResult<QueryResult> {
    execute_insert_inner(WriteTarget::Database(db), table_name, key, value)
}
pub fn execute_insert_in_txn(txn: &WriteTransaction, table_name: &str, key: &Literal, value: &Literal) -> CliResult<QueryResult> {
    execute_insert_inner(WriteTarget::Transaction(txn), table_name, key, value)
}

fn execute_insert_inner(target: WriteTarget<'_>, table_name: &str, key: &Literal, value: &Literal) -> CliResult<QueryResult> {
    let sk = lit_to_string(key);
    let sv = lit_to_string(value);

    if let Some(r) = try_insert_str_key(target, table_name, sk.as_str(), value, &sv) { return r; }
    if let Some(r) = try_insert_bytes_key(target, table_name, sk.as_bytes(), value, &sv) { return r; }
    if let Some(r) = try_insert_owned_key_ref_val(target, table_name, key, value, &sv) { return r; }

    macro_rules! try_insert {
        ($K:ty, $V:ty) => {
            if let (Some(k), Some(v)) = (<$K as OwnedFromLit>::from_lit(key), <$V as OwnedFromLit>::from_lit(value)) {
                if_ok!(target.try_insert::<$K, $V>(table_name, &k, &v));
            }
        };
    }
    for_all_owned_table_types!(try_insert);

    Err(CliError::Engine(format!("无法插入数据到 '{}'。请使用 DESCRIBE 查看表结构。", table_name)))
}

// ── INSERT: &str 键专用 ──

fn try_insert_str_key(target: WriteTarget<'_>, table_name: &str, key: &str, value: &Literal, sv: &str) -> Option<CliResult<QueryResult>> {
    macro_rules! try_v { ($V:ty) => {
        if let Some(v) = <$V as OwnedFromLit>::from_lit(value) {
            if let Ok(r) = insert_str_key_val::<$V>(target, table_name, key, &v) { return Some(Ok(r)); }
        }
    }}
    try_v!(i64); try_v!(u64); try_v!(f64); try_v!(bool);
    try_v!(i32); try_v!(u32); try_v!(f32);
    try_v!(i128); try_v!(u128);
    try_v!(i16); try_v!(u16); try_v!(i8); try_v!(u8);
    try_v!(String);

    if let Ok(r) = insert_str_str(target, table_name, key, sv) { return Some(Ok(r)); }
    if let Ok(r) = insert_str_bytes(target, table_name, key, sv.as_bytes()) { return Some(Ok(r)); }
    None
}

// ── INSERT: &[u8] 键专用 ──

fn try_insert_bytes_key(target: WriteTarget<'_>, table_name: &str, key: &[u8], value: &Literal, sv: &str) -> Option<CliResult<QueryResult>> {
    if let Ok(r) = insert_bytes_bytes(target, table_name, key, sv.as_bytes()) { return Some(Ok(r)); }
    if let Ok(r) = insert_bytes_str(target, table_name, key, sv) { return Some(Ok(r)); }
    if let Ok(r) = insert_bytes_key_val::<String>(target, table_name, key, &sv.to_string()) { return Some(Ok(r)); }
    if let Some(v) = lit_to_i64(value) { if let Ok(r) = insert_bytes_key_val::<i64>(target, table_name, key, &v) { return Some(Ok(r)); } }
    if let Some(v) = lit_to_u64(value) { if let Ok(r) = insert_bytes_key_val::<u64>(target, table_name, key, &v) { return Some(Ok(r)); } }
    None
}

// ── INSERT: 自有键 × 引用值 ──

fn try_insert_owned_key_ref_val(target: WriteTarget<'_>, table_name: &str, key: &Literal, value: &Literal, sv: &str) -> Option<CliResult<QueryResult>> {
    if !matches!(value, Literal::String(_)) { return None; }
    macro_rules! try_k { ($K:ty) => {
        if let Some(k) = <$K as OwnedFromLit>::from_lit(key) {
            if let Ok(r) = insert_owned_str(target, table_name, &k, sv) { return Some(Ok(r)); }
            if let Ok(r) = insert_owned_bytes(target, table_name, &k, sv.as_bytes()) { return Some(Ok(r)); }
        }
    }}
    try_k!(String); try_k!(i64); try_k!(u64);
    try_k!(i32); try_k!(u32); try_k!(bool);
    try_k!(i128); try_k!(u128); try_k!(i16); try_k!(i8);
    None
}

// ── WriteTarget ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum WriteTarget<'a> { Database(&'a Database), Transaction(&'a WriteTransaction) }

impl WriteTarget<'_> {
    fn try_insert<K: Key + 'static, V: Value + 'static>(self, table: &str, k: &K, v: &V) -> CliResult<QueryResult>
    where for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>, for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
    {
        match self {
            WriteTarget::Database(db) => { let txn = db.begin_write()?; let r = insert_txn::<K, V>(&txn, table, k, v); finish(txn, r) }
            WriteTarget::Transaction(txn) => insert_txn::<K, V>(txn, table, k, v),
        }
    }
    fn try_delete<K: Key + 'static, V: Value + 'static>(self, table: &str, key: &K) -> CliResult<QueryResult>
    where for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
    {
        match self {
            WriteTarget::Database(db) => { let txn = db.begin_write()?; let r = delete_txn::<K, V>(&txn, table, key); finish(txn, r) }
            WriteTarget::Transaction(txn) => delete_txn::<K, V>(txn, table, key),
        }
    }
}

fn finish(txn: WriteTransaction, r: CliResult<QueryResult>) -> CliResult<QueryResult> {
    match r { Ok(q) => { txn.commit()?; Ok(q) } Err(e) => { txn.abort()?; Err(e) } }
}

fn insert_txn<K: Key + 'static, V: Value + 'static>(txn: &WriteTransaction, table: &str, k: &K, v: &V) -> CliResult<QueryResult>
where for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>, for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<K, V>::new(table);
    let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(k, v)?;
    Ok(QueryResult::with_message(format!("已插入 1 行到 '{}'。", table)))
}

// ── 引用类型专用 INSERT（绕过 HRTB 生命周期约束）──

fn insert_str_key_val<V: Value + 'static>(target: WriteTarget<'_>, table: &str, key: &str, v: &V) -> CliResult<QueryResult>
where for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; let r = insert_str_key_val_txn::<V>(&txn, table, key, v); finish(txn, r) }
        WriteTarget::Transaction(txn) => insert_str_key_val_txn::<V>(txn, table, key, v),
    }
}
fn insert_str_key_val_txn<V: Value + 'static>(txn: &WriteTransaction, table: &str, key: &str, v: &V) -> CliResult<QueryResult>
where for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<&str, V>::new(table);
    let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(key, v)?;
    Ok(QueryResult::with_message(format!("已插入 1 行到 '{}'。", table)))
}

fn insert_str_str(target: WriteTarget<'_>, table: &str, key: &str, val: &str) -> CliResult<QueryResult> {
    let msg = format!("已插入 1 行到 '{}'。", table);
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; { let def = TableDefinition::<&str, &str>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; } finish(txn, Ok(QueryResult::with_message(msg))) }
        WriteTarget::Transaction(txn) => { let def = TableDefinition::<&str, &str>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; Ok(QueryResult::with_message(msg)) }
    }
}

fn insert_str_bytes(target: WriteTarget<'_>, table: &str, key: &str, val: &[u8]) -> CliResult<QueryResult> {
    let msg = format!("已插入 1 行到 '{}'。", table);
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; { let def = TableDefinition::<&str, &[u8]>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; } finish(txn, Ok(QueryResult::with_message(msg))) }
        WriteTarget::Transaction(txn) => { let def = TableDefinition::<&str, &[u8]>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; Ok(QueryResult::with_message(msg)) }
    }
}

fn insert_bytes_key_val<V: Value + 'static>(target: WriteTarget<'_>, table: &str, key: &[u8], v: &V) -> CliResult<QueryResult>
where for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; let r = insert_bytes_key_val_txn::<V>(&txn, table, key, v); finish(txn, r) }
        WriteTarget::Transaction(txn) => insert_bytes_key_val_txn::<V>(txn, table, key, v),
    }
}
fn insert_bytes_key_val_txn<V: Value + 'static>(txn: &WriteTransaction, table: &str, key: &[u8], v: &V) -> CliResult<QueryResult>
where for<'a> &'a V: Borrow<<V as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<&[u8], V>::new(table);
    let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    t.insert(key, v)?;
    Ok(QueryResult::with_message(format!("已插入 1 行到 '{}'。", table)))
}

fn insert_bytes_str(target: WriteTarget<'_>, table: &str, key: &[u8], val: &str) -> CliResult<QueryResult> {
    let msg = format!("已插入 1 行到 '{}'。", table);
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; { let def = TableDefinition::<&[u8], &str>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; } finish(txn, Ok(QueryResult::with_message(msg))) }
        WriteTarget::Transaction(txn) => { let def = TableDefinition::<&[u8], &str>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; Ok(QueryResult::with_message(msg)) }
    }
}

fn insert_bytes_bytes(target: WriteTarget<'_>, table: &str, key: &[u8], val: &[u8]) -> CliResult<QueryResult> {
    let msg = format!("已插入 1 行到 '{}'。", table);
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; { let def = TableDefinition::<&[u8], &[u8]>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; } finish(txn, Ok(QueryResult::with_message(msg))) }
        WriteTarget::Transaction(txn) => { let def = TableDefinition::<&[u8], &[u8]>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(key, val)?; Ok(QueryResult::with_message(msg)) }
    }
}

fn insert_owned_str<K: Key + 'static>(target: WriteTarget<'_>, table: &str, k: &K, val: &str) -> CliResult<QueryResult>
where for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
{
    let msg = format!("已插入 1 行到 '{}'。", table);
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; { let def = TableDefinition::<K, &str>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(k, val)?; } finish(txn, Ok(QueryResult::with_message(msg))) }
        WriteTarget::Transaction(txn) => { let def = TableDefinition::<K, &str>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(k, val)?; Ok(QueryResult::with_message(msg)) }
    }
}

fn insert_owned_bytes<K: Key + 'static>(target: WriteTarget<'_>, table: &str, k: &K, val: &[u8]) -> CliResult<QueryResult>
where for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
{
    let msg = format!("已插入 1 行到 '{}'。", table);
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; { let def = TableDefinition::<K, &[u8]>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(k, val)?; } finish(txn, Ok(QueryResult::with_message(msg))) }
        WriteTarget::Transaction(txn) => { let def = TableDefinition::<K, &[u8]>::new(table); let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?; t.insert(k, val)?; Ok(QueryResult::with_message(msg)) }
    }
}

// ── DELETE ─────────────────────────────────────────────────────────────────

pub fn execute_delete(db: &Database, table_name: &str, condition: &crate::parser::ast::Condition) -> CliResult<QueryResult> {
    execute_delete_inner(WriteTarget::Database(db), table_name, condition)
}
pub fn execute_delete_in_txn(txn: &WriteTransaction, table_name: &str, condition: &crate::parser::ast::Condition) -> CliResult<QueryResult> {
    execute_delete_inner(WriteTarget::Transaction(txn), table_name, condition)
}

fn execute_delete_inner(target: WriteTarget<'_>, table_name: &str, condition: &crate::parser::ast::Condition) -> CliResult<QueryResult> {
    let key_val = match condition {
        crate::parser::ast::Condition::Equals(_, v) => v,
        _ => return Err(CliError::Engine("DELETE 仅支持 WHERE key = <值>。".into())),
    };
    let sk = lit_to_string(key_val);

    if let Some(r) = try_delete_str_key(target, table_name, sk.as_str()) { return r; }
    if matches!(key_val, Literal::String(_)) {
        if let Some(r) = try_delete_bytes_key(target, table_name, sk.as_bytes()) { return r; }
    }
    if let Some(r) = try_delete_owned_ref_val(target, table_name, key_val) { return r; }

    macro_rules! try_delete {
        ($K:ty, $V:ty) => { if let Some(k) = <$K as OwnedFromLit>::from_lit(key_val) {
            if_ok!(target.try_delete::<$K, $V>(table_name, &k));
        }}
    }
    for_all_owned_table_types!(try_delete);

    Err(CliError::Engine(format!("无法在表 '{}' 中找到匹配的类型进行 DELETE。", table_name)))
}

fn try_delete_str_key(target: WriteTarget<'_>, table_name: &str, key: &str) -> Option<CliResult<QueryResult>> {
    macro_rules! try_v { ($V:ty) => { if let Ok(r) = delete_str_key::<$V>(target, table_name, key) { return Some(Ok(r)); } } }
    try_v!(&str); try_v!(i64); try_v!(u64); try_v!(f64); try_v!(bool); try_v!(&[u8]);
    try_v!(i32); try_v!(u32); try_v!(f32); try_v!(i128); try_v!(u128); try_v!(String);
    try_v!(i16); try_v!(u16); try_v!(i8); try_v!(u8);
    None
}

fn try_delete_bytes_key(target: WriteTarget<'_>, table_name: &str, key: &[u8]) -> Option<CliResult<QueryResult>> {
    macro_rules! try_v { ($V:ty) => { if let Ok(r) = delete_bytes_key::<$V>(target, table_name, key) { return Some(Ok(r)); } } }
    try_v!(&[u8]); try_v!(&str); try_v!(i64); try_v!(u64); try_v!(String);
    None
}

fn try_delete_owned_ref_val(target: WriteTarget<'_>, table_name: &str, key_val: &Literal) -> Option<CliResult<QueryResult>> {
    macro_rules! try_k { ($K:ty) => { if let Some(k) = <$K as OwnedFromLit>::from_lit(key_val) {
        if let Ok(r) = target.try_delete::<$K, &str>(table_name, &k) { return Some(Ok(r)); }
        if let Ok(r) = target.try_delete::<$K, &[u8]>(table_name, &k) { return Some(Ok(r)); }
    }}}
    try_k!(String); try_k!(i64); try_k!(u64); try_k!(i32); try_k!(u32);
    try_k!(bool); try_k!(i128); try_k!(u128); try_k!(i16); try_k!(i8);
    None
}

fn delete_str_key<V: Value + 'static>(target: WriteTarget<'_>, table: &str, key: &str) -> CliResult<QueryResult> {
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; let r = delete_str_key_txn::<V>(&txn, table, key); finish(txn, r) }
        WriteTarget::Transaction(txn) => delete_str_key_txn::<V>(txn, table, key),
    }
}
fn delete_str_key_txn<V: Value + 'static>(txn: &WriteTransaction, table: &str, key: &str) -> CliResult<QueryResult> {
    let def = TableDefinition::<&str, V>::new(table);
    let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    let existed = t.remove(key)?.is_some();
    Ok(QueryResult::with_message(if existed { format!("已从 '{}' 删除 1 行。", table) } else { format!("在 '{}' 中未找到匹配的行。", table) }))
}

fn delete_bytes_key<V: Value + 'static>(target: WriteTarget<'_>, table: &str, key: &[u8]) -> CliResult<QueryResult> {
    match target {
        WriteTarget::Database(db) => { let txn = db.begin_write()?; let r = delete_bytes_key_txn::<V>(&txn, table, key); finish(txn, r) }
        WriteTarget::Transaction(txn) => delete_bytes_key_txn::<V>(txn, table, key),
    }
}
fn delete_bytes_key_txn<V: Value + 'static>(txn: &WriteTransaction, table: &str, key: &[u8]) -> CliResult<QueryResult> {
    let def = TableDefinition::<&[u8], V>::new(table);
    let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    let existed = t.remove(key)?.is_some();
    Ok(QueryResult::with_message(if existed { format!("已从 '{}' 删除 1 行。", table) } else { format!("在 '{}' 中未找到匹配的行。", table) }))
}

fn delete_txn<K: Key + 'static, V: Value + 'static>(txn: &WriteTransaction, table: &str, key: &K) -> CliResult<QueryResult>
where for<'a> &'a K: Borrow<<K as Value>::SelfType<'a>>,
{
    let def = TableDefinition::<K, V>::new(table);
    let mut t = txn.open_table(def).map_err(|_| CliError::TypeMismatch(format!("表 '{}'", table)))?;
    let existed = t.remove(key)?.is_some();
    Ok(QueryResult::with_message(if existed { format!("已从 '{}' 删除 1 行。", table) } else { format!("在 '{}' 中未找到匹配的行。", table) }))
}

macro_rules! if_ok { ($e:expr) => { match $e { Ok(r) => return Ok(r), Err(_) => {} } }; }
use if_ok;
