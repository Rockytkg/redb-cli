use crate::engine::query::QueryResult;
use crate::parser::ast::OutputMode;
use unicode_width::UnicodeWidthStr;

pub fn render_result(result: &QueryResult, mode: OutputMode) -> String {
    match mode {
        OutputMode::Table => render_table(result),
        OutputMode::Vertical => render_vertical(result),
    }
}

pub fn render_table(result: &QueryResult) -> String {
    if let Some(ref msg) = result.message {
        return msg.clone();
    }
    if result.columns.is_empty() {
        return "空结果。".into();
    }
    if result.rows.is_empty() {
        return "空集 (0 行)。".into();
    }

    let col_count = result.columns.len();
    let mut widths: Vec<usize> = result.columns.iter().map(|c| display_width(c)).collect();
    for row in &result.rows {
        for (i, value) in row.iter().enumerate().take(col_count) {
            widths[i] = widths[i].max(display_width(value));
        }
    }
    for w in &mut widths {
        *w = (*w).max(4);
    }

    // 分隔线
    let mut sep = String::from("+");
    for w in &widths {
        sep.push_str(&"-".repeat(w + 2));
        sep.push('+');
    }

    let mut out = String::new();
    out.push_str(&sep);
    out.push('\n');
    out.push('|');
    for (i, col) in result.columns.iter().enumerate() {
        push_cell(&mut out, col, widths[i]);
    }
    out.push('\n');
    out.push_str(&sep);
    out.push('\n');

    for row in &result.rows {
        out.push('|');
        for (i, width) in widths.iter().enumerate() {
            let value = row.get(i).map(String::as_str).unwrap_or("");
            push_cell(&mut out, value, *width);
        }
        out.push('\n');
    }
    out.push_str(&sep);

    let n = result.rows.len();
    if n > 0 {
        out.push_str(&format!("\n{} 行", n));
    }

    out
}

pub fn render_vertical(result: &QueryResult) -> String {
    if let Some(ref msg) = result.message {
        return msg.clone();
    }
    if result.columns.is_empty() {
        return "空结果。".into();
    }
    if result.rows.is_empty() {
        return "空集 (0 行)。".into();
    }

    let label_width = result
        .columns
        .iter()
        .map(|column| display_width(column))
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    for (row_index, row) in result.rows.iter().enumerate() {
        if row_index > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "*************************** {}. row ***************************\n",
            row_index + 1
        ));
        for (column_index, column) in result.columns.iter().enumerate() {
            let value = row.get(column_index).map(String::as_str).unwrap_or("");
            out.push_str(&" ".repeat(label_width.saturating_sub(display_width(column))));
            out.push_str(column);
            out.push_str(": ");
            out.push_str(value);
            out.push('\n');
        }
    }
    out.push_str(&format!("{} 行", result.rows.len()));
    out
}

fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn push_cell(out: &mut String, value: &str, width: usize) {
    out.push(' ');
    out.push_str(value);
    out.push_str(&" ".repeat(width.saturating_sub(display_width(value)) + 1));
    out.push('|');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_full_content_without_truncation() {
        let long = "中".repeat(140);
        let result = QueryResult {
            columns: vec!["键".into(), "值".into()],
            rows: vec![vec!["k".into(), long.clone()]],
            message: None,
        };

        let rendered = render_table(&result);

        assert!(rendered.contains(&long));
        assert!(!rendered.contains("..."));
    }

    #[test]
    fn renders_vertical_output_like_mysql_slash_g() {
        let result = QueryResult {
            columns: vec!["键".into(), "值".into()],
            rows: vec![
                vec!["1".into(), "Alice".into()],
                vec!["2".into(), "Bob".into()],
            ],
            message: None,
        };

        let rendered = render_vertical(&result);

        assert!(rendered.contains("*************************** 1. row ***************************"));
        assert!(rendered.contains("键: 1"));
        assert!(rendered.contains("值: Alice"));
        assert!(rendered.contains("*************************** 2. row ***************************"));
        assert!(rendered.ends_with("2 行"));
    }
}
