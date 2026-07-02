/// 数据类型（用于 CREATE TABLE）
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    // 键类型
    I32,
    I64,
    U32,
    U64,
    Bool,
    String_,
    Str,    // &str   (redb 4.x Key)
    BytesK, // &[u8]  (redb 4.x Key)
    // 仅值类型
    F32,
    F64,
    Bytes, // &[u8] Value
}

impl DataType {
    /// 从 SQL 类型名解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "I32" | "INT32" => Some(DataType::I32),
            "I64" | "INT" | "INTEGER" | "INT64" => Some(DataType::I64),
            "U32" | "UINT32" => Some(DataType::U32),
            "U64" | "UINT" | "UINT64" => Some(DataType::U64),
            "F32" | "FLOAT32" => Some(DataType::F32),
            "F64" | "FLOAT" | "FLOAT64" | "REAL" => Some(DataType::F64),
            "BOOL" | "BOOLEAN" => Some(DataType::Bool),
            "STRING" | "TEXT" | "STR" => Some(DataType::String_),
            "STR_KEY" | "STRKEY" => Some(DataType::Str),
            "BYTES" | "BLOB" | "BINARY" => Some(DataType::Bytes),
            "BYTES_KEY" | "BYTESKEY" | "BLOB_KEY" => Some(DataType::BytesK),
            _ => None,
        }
    }

    pub fn to_type_str(&self) -> &'static str {
        match self {
            DataType::I32 => "i32",
            DataType::I64 => "i64",
            DataType::U32 => "u32",
            DataType::U64 => "u64",
            DataType::F32 => "f32",
            DataType::F64 => "f64",
            DataType::Bool => "bool",
            DataType::String_ => "String",
            DataType::Str => "&str",
            DataType::Bytes => "&[u8]",
            DataType::BytesK => "&[u8]",
        }
    }

    /// redb 4.x: 所有基础类型均可作为键
    /// 仅 F32/F64 不能作为键（它们只实现了 Value，未实现 Key）
    #[allow(dead_code)]
    pub fn is_valid_key(&self) -> bool {
        !matches!(self, DataType::F32 | DataType::F64)
    }
}

/// 字面量
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Null,
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Int(i) => write!(f, "{}", i),
            Literal::Float(v) => write!(f, "{}", v),
            Literal::String(s) => write!(f, "'{}'", s),
            Literal::Null => write!(f, "NULL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Equals(String, Literal),
    NotEquals(String, Literal),
    GreaterThan(String, Literal),
    GreaterEquals(String, Literal),
    LessThan(String, Literal),
    LessEquals(String, Literal),
    Between(String, Literal, Literal),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    pub column: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Table,
    Vertical,
}

/// SQL 语句 AST
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    DotOpen(String),
    DotInfo {
        output_mode: OutputMode,
    },
    DotCompact {
        output_mode: OutputMode,
    },
    DotHelp,
    ShowTables {
        output_mode: OutputMode,
    },
    Describe {
        table: String,
        output_mode: OutputMode,
    },
    CreateTable {
        name: String,
        key_type: DataType,
        value_type: DataType,
        output_mode: OutputMode,
    },
    DropTable {
        name: String,
        output_mode: OutputMode,
    },
    Select {
        table: String,
        condition: Option<Condition>,
        order_by: Option<OrderBy>,
        limit: Option<u64>,
        offset: Option<u64>,
        count_only: bool,
        output_mode: OutputMode,
    },
    Insert {
        table: String,
        key: Literal,
        value: Literal,
        output_mode: OutputMode,
    },
    Delete {
        table: String,
        condition: Condition,
        output_mode: OutputMode,
    },
    Begin,
    Commit,
    Rollback,
    Help,
    Clear,
    Exit,
}
