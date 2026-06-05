//! 版本信息

/// 编译时常量: RBD 包版本
pub const RBD_VERSION: &str = env!("CARGO_PKG_VERSION");
/// 编译时常量: RBD 包名
pub const RBD_NAME: &str = env!("CARGO_PKG_NAME");
/// 编译时常量: RBD 作者
pub const RBD_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

/// 显示版本信息
pub fn print() {
    println!("{RBD_NAME} {RBD_VERSION}");
    println!("RustBiliDownloader");
    println!("作者: {RBD_AUTHORS}");
    println!("License: MIT");
}
