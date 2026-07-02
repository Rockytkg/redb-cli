use redb::{Database, MultimapTableHandle, ReadableDatabase, TableHandle};

use crate::engine::query::QueryResult;
use crate::error::CliResult;

/// SHOW TABLES — 列出所有表
pub fn execute_show_tables(db: &Database) -> CliResult<QueryResult> {
    let txn = db.begin_read()?;
    let mut tables: Vec<Vec<String>> = Vec::new();

    if let Ok(iter) = txn.list_tables() {
        for handle in iter {
            tables.push(vec![handle.name().to_string(), "TABLE".into()]);
        }
    }
    if let Ok(iter) = txn.list_multimap_tables() {
        for handle in iter {
            tables.push(vec![handle.name().to_string(), "MULTIMAP".into()]);
        }
    }

    if tables.is_empty() {
        return Ok(QueryResult::with_message("数据库中没有任何表。".into()));
    }
    tables.sort_by(|a, b| a[0].cmp(&b[0]));
    Ok(QueryResult {
        columns: vec!["表名".into(), "类型".into()],
        rows: tables,
        message: None,
    })
}

/// .info — 数据库统计信息
pub fn execute_info(db: &Database) -> CliResult<QueryResult> {
    let txn = db.begin_read()?;
    let mut lines: Vec<String> = Vec::new();

    let normal = txn.list_tables().map(|i| i.count()).unwrap_or(0);
    let mm = txn.list_multimap_tables().map(|i| i.count()).unwrap_or(0);

    lines.push("数据库信息:".into());
    lines.push(format!("  普通表:      {}", normal));
    lines.push(format!("  Multimap 表: {}", mm));
    lines.push(format!("  表总数:      {}", normal + mm));

    drop(txn);
    if let Ok(txn) = db.begin_write() {
        if let Ok(stats) = txn.stats() {
            lines.push(format!("  树高度:      {}", stats.tree_height()));
            lines.push(format!("  已分配页:    {}", stats.allocated_pages()));
            lines.push(format!(
                "  存储字节:    {}",
                format_bytes(stats.stored_bytes())
            ));
            lines.push(format!(
                "  元数据字节:  {}",
                format_bytes(stats.metadata_bytes())
            ));
            lines.push(format!(
                "  碎片字节:    {}",
                format_bytes(stats.fragmented_bytes())
            ));
            lines.push(format!("  页大小:      {} B", stats.page_size()));
        }
        txn.abort()?;
    }

    Ok(QueryResult::with_message(lines.join("\n")))
}

/// .compact — 压缩数据库
pub fn execute_compact(db: &Database) -> CliResult<QueryResult> {
    let txn = db.begin_write()?;
    txn.commit()?;
    Ok(QueryResult::with_message(
        "写事务已提交——碎片页将在后续压缩中回收。".into(),
    ))
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut idx = 0;
    while size >= 1024.0 && idx < UNITS.len() - 1 {
        size /= 1024.0;
        idx += 1;
    }
    format!("{:.2} {}", size, UNITS[idx])
}
