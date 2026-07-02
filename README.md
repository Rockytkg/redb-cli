# redb-cli

`redb-cli` 是一个用于 [redb](https://github.com/cberner/redb) 嵌入式键值数据库的交互式命令行工具，采用 **类 MySQL 的 SQL 语法**，让您可以像操作关系型数据库一样操作 redb 数据库。

## 安装

```bash
# 克隆项目
git clone <repo-url>
cd redb-cli

# 编译
cargo build --release

# 安装到系统路径（可选）
cargo install --path .
```

## 快速开始

```bash
# 打开（或自动创建）一个 redb 数据库文件
redb-cli mydata.redb

# 或者直接启动交互式 Shell（不指定文件）
redb-cli
```

## 支持的 SQL 语法

### 数据库管理

| 命令 | 说明 |
|------|------|
| `.open <filepath>` | 打开一个 redb 数据库文件 |
| `.info` | 显示数据库统计信息（表数量、页数、存储空间等） |
| `.compact` | 触发数据库压缩（回收碎片空间） |

### DDL — 表管理

```sql
-- 查看所有表
SHOW TABLES;

-- 查看表结构（键类型、值类型、行数）
DESCRIBE <表名>;
DESC <表名>;        -- 简写

-- 创建表：CREATE TABLE <表名> (<键类型>, <值类型>);
CREATE TABLE users (I64, STRING);
CREATE TABLE scores (STRING, I64);
CREATE TABLE flags (I64, BOOL);

-- 删除表
DROP TABLE <表名>;
```

### DML — 查询

```sql
-- 全表扫描
SELECT * FROM <表名>;

-- 纵向输出（MariaDB/MySQL 风格）
SELECT * FROM <表名>\G
SHOW TABLES\G

-- 带 LIMIT 和 OFFSET
SELECT * FROM <表名> LIMIT 10;
SELECT * FROM <表名> LIMIT 10 OFFSET 20;

-- 条件查询
SELECT * FROM <表名> WHERE key = <值>;
SELECT * FROM <表名> WHERE key > <值>;
SELECT * FROM <表名> WHERE key < <值>;
SELECT * FROM <表名> WHERE key >= <值>;
SELECT * FROM <表名> WHERE key <= <值>;
SELECT * FROM <表名> WHERE key BETWEEN <v1> AND <v2>;

-- 排序
SELECT * FROM <表名> ORDER BY key ASC;
SELECT * FROM <表名> ORDER BY key DESC;

-- 计数
SELECT COUNT(*) FROM <表名>;
```

### DML — 写入

```sql
-- 插入数据
INSERT INTO <表名> VALUES (<键>, <值>);

-- 删除数据
DELETE FROM <表名> WHERE key = <值>;
DELETE FROM <表名> WHERE key BETWEEN <v1> AND <v2>;
```

### 事务管理

```sql
BEGIN;      -- 开始写事务
COMMIT;     -- 提交事务
ROLLBACK;   -- 回滚事务
```

### 工具命令

```sql
HELP;       -- 显示帮助信息
CLEAR;      -- 清屏
EXIT;       -- 退出（同 QUIT;）
```

## 支持的数据类型

redb 是一个**强类型**键值存储。创建表时必须指定键和值的类型。

### 键类型（Key）

| SQL 类型名 | redb 类型 | 别名 |
|-----------|-----------|------|
| `I32` | `i32` | `INT32` |
| `I64` | `i64` | `INT`, `INTEGER`, `INT64` |
| `U32` | `u32` | `UINT32` |
| `U64` | `u64` | `UINT`, `UINT64` |
| `BOOL` | `bool` | `BOOLEAN` |
| `STRING` | `String` | `TEXT`, `STR` |

### 值类型（Value）

支持所有键类型，外加：

| SQL 类型名 | redb 类型 | 别名 |
|-----------|-----------|------|
| `F32` | `f32` | `FLOAT32` |
| `F64` | `f64` | `FLOAT`, `FLOAT64`, `REAL` |
| `STRING` | `&str` | `TEXT`, `STR` |
| `BYTES` | `&[u8]` | `BLOB`, `BINARY` |

> **重要限制**：`FLOAT` 和 `BYTES` 类型**只能用作值类型，不能用作键类型**。这是 redb 的类型系统决定的。

## 完整使用示例

```bash
$ redb-cli demo.redb
redb-cli v0.1.0 (redb 4.x)
输入 .help 或 HELP; 查看帮助。
输入 EXIT; 或 QUIT; 退出。

demo.redb> CREATE TABLE users (I64, STRING);
表 'users' 已创建 (键: i64, 值: String)。

demo.redb> INSERT INTO users VALUES (1, 'Alice');
已插入 1 行到 'users'。

demo.redb> INSERT INTO users VALUES (2, 'Bob');
已插入 1 行到 'users'。

demo.redb> INSERT INTO users VALUES (3, 'Charlie');
已插入 1 行到 'users'。

demo.redb> SELECT * FROM users;
+------+-----------+
| 键   | 值        |
+------+-----------+
| 1    | Alice     |
| 2    | Bob       |
| 3    | Charlie   |
+------+-----------+
3 行

demo.redb> SELECT COUNT(*) FROM users;
+------+
| 行数 |
+------+
| 3    |
+------+
1 行

demo.redb> SELECT * FROM users\G
*************************** 1. row ***************************
键: 1
值: Alice
*************************** 2. row ***************************
键: 2
值: Bob
*************************** 3. row ***************************
键: 3
值: Charlie
3 行

demo.redb> DELETE FROM users WHERE key = 2;
已从 'users' 删除 1 行。

demo.redb> SELECT * FROM users;
+------+-----------+
| 键   | 值        |
+------+-----------+
| 1    | Alice     |
| 3    | Charlie   |
+------+-----------+
2 行

demo.redb> DESCRIBE users;
表: users
  键类型:    i64
  值类型:    alloc::string::String
  行数:      2

demo.redb> SHOW TABLES;
+-------+-------+
| 表名  | 类型  |
+-------+-------+
| users | TABLE |
+-------+-------+
1 行

demo.redb> .info
数据库信息:
  普通表:      1
  Multimap 表: 0
  表总数:      1
  树高度:      2
  已分配页:    7
  存储字节:    28.00 B
  元数据字节:  1.33 KB
  碎片字节:    1014.64 KB
  页大小:      4096 B

demo.redb> DROP TABLE users;
表 'users' 已删除。

demo.redb> EXIT;
再见！
```

## 与 redb API 的对应关系

redb CLI 在底层使用 redb 的类型系统，通过**类型分发（type dispatch）** 机制来匹配用户指定的类型：

| 操作 | 实现策略 |
|------|---------|
| `SELECT` | 依次尝试常见类型组合（`String`/`i64`/`u64`/`&str`/`f64`/`bool`/`&[u8]`），使用第一个匹配的 |
| `INSERT` | 根据用户输入的键/值字面量，尝试对应的类型组合写入 |
| `DELETE` | 根据键的格式（整数/字符串），尝试所有匹配的值类型组合 |
| `DESCRIBE` | 尝试打开表以获取类型信息和行数 |
| `CREATE TABLE` | 直接按用户指定的类型创建 `TableDefinition` |

## 架构概览

```
src/
├── main.rs              — 入口（clap 参数解析，启动 REPL）
├── error.rs             — 统一错误类型（thiserror）
├── cli/
│   ├── repl.rs          — 交互式 REPL（rustyline）
│   └── completer.rs     — Tab 补全（SQL 关键字）
├── parser/
│   ├── lexer.rs         — 词法分析器（SQL → Token）
│   ├── ast.rs           — 抽象语法树定义
│   └── sql.rs           — 递归下降解析器（Token → AST）
└── engine/
    ├── session.rs       — 数据库会话管理
    ├── executor.rs      — 语句执行分发
    ├── query.rs         — SELECT / DESCRIBE 读操作
    ├── mutate.rs        — INSERT / DELETE 写操作
    ├── ddl.rs           — CREATE / DROP TABLE
    ├── meta.rs          — SHOW TABLES / .info
    └── display.rs       — ASCII 表格格式化输出
```

## 技术栈

- **[redb](https://crates.io/crates/redb)** — 纯 Rust 编写的嵌入式 ACID 键值数据库（Copy-on-Write B-tree, MVCC）
- **[clap](https://crates.io/crates/clap)** — CLI 参数解析
- **[rustyline](https://crates.io/crates/rustyline)** — 交互式行编辑（历史记录、Tab 补全、语法高亮）
- **[thiserror](https://crates.io/crates/thiserror)** — 错误类型 derive 宏

## 系统要求

- Rust 1.74+（2021 edition）
- Windows / macOS / Linux

## License

MIT
