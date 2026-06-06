//! 日志初始化. 支持控制台 + 文件双输出, 通过 `RUST_LOG` 环境变量控制级别.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// 日志输出目标
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogTarget {
    /// 仅 stderr
    Stderr,
    /// 仅文件
    File,
    /// 双输出
    Both,
}

/// 初始化全局日志.
///
/// `RUST_LOG` 默认值: `info` (生产) / `rbd=debug,info` (开发).
///
/// 返回的 [`WorkerGuard`] 必须保留在 `main` 作用域, 丢弃后日志写入会停止.
pub fn init(target: LogTarget, log_file: Option<&Path>) -> anyhow::Result<WorkerGuard> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,rbd=debug"));

    match target {
        LogTarget::Stderr => {
            let (writer, guard) = tracing_appender::non_blocking(std::io::stderr());
            let layer = fmt::layer().with_writer(writer).with_ansi(true);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("初始化日志失败: {e}"))?;
            Ok(guard)
        }
        LogTarget::File => {
            let path = log_file.ok_or_else(|| anyhow::anyhow!("文件日志需要指定路径"))?;
            let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("rbd.log");
            std::fs::create_dir_all(dir)?;
            let appender = tracing_appender::rolling::daily(dir, file_name);
            let (writer, guard) = tracing_appender::non_blocking(appender);
            let layer = fmt::layer().with_writer(writer).with_ansi(false);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("初始化日志失败: {e}"))?;
            Ok(guard)
        }
        LogTarget::Both => {
            let path = log_file.ok_or_else(|| anyhow::anyhow!("文件日志需要指定路径"))?;
            let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("rbd.log");
            std::fs::create_dir_all(dir)?;
            let file_appender = tracing_appender::rolling::daily(dir, file_name);
            let (file_writer, _file_guard) = tracing_appender::non_blocking(file_appender);
            let (stderr_writer, stderr_guard) = tracing_appender::non_blocking(std::io::stderr());

            let stderr_layer = fmt::layer()
                .with_writer(stderr_writer)
                .with_ansi(true)
                .with_filter(tracing_subscriber::filter::LevelFilter::INFO);
            let file_layer = fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_filter(tracing_subscriber::filter::LevelFilter::DEBUG);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(stderr_layer)
                .with(file_layer)
                .try_init()
                .map_err(|e| anyhow::anyhow!("初始化日志失败: {e}"))?;

            // 优先丢弃 stderr guard, 关闭双句柄
            Ok(stderr_guard)
        }
    }
}
