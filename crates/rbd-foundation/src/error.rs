//! 统一 Result 别名.
//!
//! 全项目使用 [`anyhow::Result`], 在二进制边界 (CLI) 用 `.context()` 附加信息.

/// RBD 库的统一 Result 别名.
///
/// 等价于 `anyhow::Result<T>`, 全项目统一.
/// v2.0 如需类型化错误, 届时重新设计并接入.
pub type Result<T> = anyhow::Result<T>;
