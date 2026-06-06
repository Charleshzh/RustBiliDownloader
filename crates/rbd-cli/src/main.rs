//! # rbd
//!
//! RBD 命令行入口 (`rbd`).
//!
//! **设计**: M0 仅搭骨架, 实际命令在 M9 集成.
//! 决策: **Q10 仅中文**, 所有错误信息 / 日志 / 帮助均中文.

use clap::{CommandFactory, Parser, Subcommand};
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

mod args;
mod commands;
mod config;
mod i18n;
mod progress;

/// RBD - 纯 Rust B 站视频下载器
#[derive(Parser, Debug)]
#[command(
    name = "rbd",
    version,
    about,
    long_about = None,
    after_help = "示例:\n  rbd download BV1GJ411x7h7\n  rbd download --interactive https://www.bilibili.com/video/BV1GJ411x7h7\n  rbd login\n  rbd info https://www.bilibili.com/bangumi/play/ss39443\n  rbd batch urls.txt\n  rbd completions bash",
    after_long_help = "主页: https://github.com/Charleshzh/RustBiliDownloader"
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,

    /// 详细输出 (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// 配置文件路径 (默认 ~/.config/rbd/rbd.toml)
    #[arg(short = 'c', long, global = true)]
    config: Option<std::path::PathBuf>,

    /// 鉴权 profile 名 (默认: default)
    #[arg(long, global = true)]
    auth_profile: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// 下载视频
    #[command(alias = "d")]
    Download {
        /// 视频 URL 或 BV/AV 号
        url: String,

        /// 输出目录
        #[arg(short = 'o', long)]
        output_dir: Option<std::path::PathBuf>,

        /// 画质代码 (默认 80=1080P)
        #[arg(short = 'q', long)]
        quality: Option<u32>,

        /// 视频编码优先级 (avc / hevc / av1)
        #[arg(long)]
        vcodec_priority: Option<String>,

        /// 多线程数 (默认 8)
        #[arg(short = 'n', long)]
        num_workers: Option<u32>,

        /// 仅下载视频
        #[arg(long)]
        video_only: bool,

        /// 仅下载音频
        #[arg(long)]
        audio_only: bool,

        /// 不下载弹幕
        #[arg(long)]
        no_danmaku: bool,

        /// 不下载字幕
        #[arg(long)]
        no_subtitle: bool,

        /// 不下载封面
        #[arg(long)]
        no_cover: bool,

        /// 交互式选 track
        #[arg(short = 'i', long)]
        interactive: bool,

        /// 使用 aria2c (需本地 aria2c 守护进程)
        #[arg(long)]
        aria2c: bool,
    },

    /// WEB 扫码登录
    Login {
        /// profile 名 (默认: default)
        #[arg(short = 'p', long)]
        profile: Option<String>,
    },

    /// TV 扫码登录
    LoginTv {
        /// profile 名
        #[arg(short = 'p', long)]
        profile: Option<String>,
    },

    /// 查看鉴权状态
    AuthStatus,

    /// 登出 (删除本地 cookie)
    Logout {
        /// profile 名 (不指定则登出全部)
        #[arg(short = 'p', long)]
        profile: Option<String>,
    },

    /// 解析 URL (不下载)
    Info {
        /// 视频 URL
        url: String,
    },

    /// 显示版本
    Version,

    /// 生成 Shell 补全脚本
    Completions {
        /// Shell 类型
        shell: clap_complete::Shell,
    },

    /// 批量下载 (每行一个 URL)
    Batch {
        /// 包含 URL 列表的文件
        file: PathBuf,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // 初始化日志
    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    std::env::set_var("RUST_LOG", log_level);
    let _guard = rbd_foundation::log::init(rbd_foundation::log::LogTarget::Stderr, None)
        .expect("初始化日志失败");

    let _cfg = match config::load(cli.config.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("加载配置失败: {e}");
            return ExitCode::from(1);
        }
    };

    let result: anyhow::Result<()> = match cli.command {
        Cmd::Download {
            url,
            output_dir,
            quality,
            vcodec_priority,
            num_workers,
            video_only,
            audio_only,
            no_danmaku,
            no_subtitle,
            no_cover,
            interactive,
            aria2c,
        } => {
            commands::download::run(commands::download::DownloadArgs {
                url,
                output_dir,
                quality,
                vcodec_priority,
                num_workers,
                video_only,
                audio_only,
                no_danmaku,
                no_subtitle,
                no_cover,
                interactive,
                aria2c,
            })
            .await
        }
        Cmd::Login { profile } => commands::login::run_web(profile).await,
        Cmd::LoginTv { profile } => commands::login::run_tv(profile).await,
        Cmd::AuthStatus => commands::login::status(),
        Cmd::Logout { profile } => commands::login::logout(profile),
        Cmd::Info { url } => commands::info::run(&url).await,
        Cmd::Version => {
            println!("rbd {} (RustBiliDownloader)", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Cmd::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "rbd", &mut io::stdout());
            Ok(())
        }
        Cmd::Batch { file } => commands::batch::run(file).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{:#}", e);
            ExitCode::from(1)
        }
    }
}
