//! 类型注册中心 —— 所有受支持 (键, 值) 类型组合的唯一权威来源。
//!
//! `for_all_table_types!`    — 全量组合，用于 SELECT / DESCRIBE（只读操作）。
//! `for_all_owned_table_types!` — 自有类型子集，用于 INSERT / DELETE 的泛型分派。
//!
//! `&str` / `&[u8]` 键值因生命周期约束无法走泛型 `K: Key + 'static` 的写路径，
//! 故在 INSERT / DELETE 中通过专用函数单独处理，此处不再列入自有类型宏。
//! 两个宏合起来覆盖全部类型组合，新增类型需同步维护。

/// 全量类型组合 —— SELECT / DESCRIBE 使用。
#[macro_export]
macro_rules! for_all_table_types {
    ($op:ident) => {
        // ── &str 键 ──
        $op!(&str, &str);
        $op!(&str, i64);
        $op!(&str, u64);
        $op!(&str, f64);
        $op!(&str, bool);
        $op!(&str, &[u8]);
        $op!(&str, i32);
        $op!(&str, u32);
        $op!(&str, f32);
        $op!(&str, i128);
        $op!(&str, u128);
        $op!(&str, String);
        $op!(&str, i16);
        $op!(&str, u16);
        $op!(&str, i8);
        $op!(&str, u8);

        // ── &[u8] 键 ──
        $op!(&[u8], &[u8]);
        $op!(&[u8], &str);
        $op!(&[u8], i64);
        $op!(&[u8], u64);
        $op!(&[u8], String);

        // ── String 键 ──
        $op!(String, &str);
        $op!(String, i64);
        $op!(String, u64);
        $op!(String, f64);
        $op!(String, bool);
        $op!(String, &[u8]);
        $op!(String, i32);
        $op!(String, u32);
        $op!(String, f32);
        $op!(String, i128);
        $op!(String, u128);
        $op!(String, String);
        $op!(String, i16);
        $op!(String, u16);
        $op!(String, i8);
        $op!(String, u8);

        // ── i64 键 ──
        $op!(i64, &str);
        $op!(i64, i64);
        $op!(i64, u64);
        $op!(i64, f64);
        $op!(i64, bool);
        $op!(i64, &[u8]);
        $op!(i64, i32);
        $op!(i64, u32);
        $op!(i64, String);
        $op!(i64, i128);
        $op!(i64, i16);
        $op!(i64, i8);
        $op!(i64, f32);
        $op!(i64, u8);

        // ── u64 键 ──
        $op!(u64, &str);
        $op!(u64, i64);
        $op!(u64, u64);
        $op!(u64, f64);
        $op!(u64, bool);
        $op!(u64, &[u8]);
        $op!(u64, i32);
        $op!(u64, u32);
        $op!(u64, String);
        $op!(u64, f32);

        // ── i32 键 ──
        $op!(i32, &str);
        $op!(i32, i32);
        $op!(i32, i64);
        $op!(i32, u64);
        $op!(i32, bool);
        $op!(i32, String);

        // ── u32 键 ──
        $op!(u32, u32);
        $op!(u32, &str);
        $op!(u32, i64);
        $op!(u32, u64);
        $op!(u32, String);

        // ── bool 键 ──
        $op!(bool, bool);
        $op!(bool, i64);
        $op!(bool, &str);
        $op!(bool, u64);
        $op!(bool, String);

        // ── i128 / u128 键 ──
        $op!(i128, i128);
        $op!(i128, &str);
        $op!(u128, u128);
        $op!(u128, &str);

        // ── i16 / u16 / i8 / u8 键 ──
        $op!(i16, i16);
        $op!(i16, &str);
        $op!(u16, u16);
        $op!(i8, i8);
        $op!(u8, u8);
    };
}

/// 自有类型组合 —— INSERT / DELETE 泛型分派使用。
///
/// 与 `for_all_table_types!` 相比，去掉了 `&str` 和 `&[u8]` 键/值组合
///（这些组合需要专用函数处理，无法通过 `K: Key + 'static` 泛型路径）。
#[macro_export]
macro_rules! for_all_owned_table_types {
    ($op:ident) => {
        // ── String 键 ──
        $op!(String, i64);
        $op!(String, u64);
        $op!(String, f64);
        $op!(String, bool);
        $op!(String, i32);
        $op!(String, u32);
        $op!(String, f32);
        $op!(String, i128);
        $op!(String, u128);
        $op!(String, String);
        $op!(String, i16);
        $op!(String, u16);
        $op!(String, i8);
        $op!(String, u8);

        // ── i64 键 ──
        $op!(i64, i64);
        $op!(i64, u64);
        $op!(i64, f64);
        $op!(i64, bool);
        $op!(i64, i32);
        $op!(i64, u32);
        $op!(i64, String);
        $op!(i64, i128);
        $op!(i64, i16);
        $op!(i64, i8);
        $op!(i64, f32);
        $op!(i64, u8);

        // ── u64 键 ──
        $op!(u64, i64);
        $op!(u64, u64);
        $op!(u64, f64);
        $op!(u64, bool);
        $op!(u64, i32);
        $op!(u64, u32);
        $op!(u64, String);
        $op!(u64, f32);

        // ── i32 键 ──
        $op!(i32, i32);
        $op!(i32, i64);
        $op!(i32, u64);
        $op!(i32, bool);
        $op!(i32, String);

        // ── u32 键 ──
        $op!(u32, u32);
        $op!(u32, i64);
        $op!(u32, u64);
        $op!(u32, String);

        // ── bool 键 ──
        $op!(bool, bool);
        $op!(bool, i64);
        $op!(bool, u64);
        $op!(bool, String);

        // ── i128 / u128 键 ──
        $op!(i128, i128);
        $op!(u128, u128);

        // ── i16 / u16 / i8 / u8 键 ──
        $op!(i16, i16);
        $op!(u16, u16);
        $op!(i8, i8);
        $op!(u8, u8);
    };
}

/// Multimap 类型组合。
#[macro_export]
macro_rules! for_all_multimap_types {
    ($op:ident) => {
        $op!(&str, &str);
        $op!(String, &str);
        $op!(i64, &str);
        $op!(i64, i64);
        $op!(&str, i64);
        $op!(String, i64);
        $op!(u64, u64);
        $op!(u64, &str);
        $op!(i64, &[u8]);
        $op!(String, String);
        $op!(&str, &[u8]);
    };
}
