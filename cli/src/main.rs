//! `mpv-config` binary: clap-driven subcommand framework.

use clap::{Args, Parser, Subcommand, ValueEnum};
use cli::gen::{self, GenOptions};
use core::platform::Platform;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mpv-config",
    version = "0.1.0-dev",
    about = "跨平台 mpv 配置生成与包管理工具"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 读取 config/ 各层(base → 平台 → 包 → user),生成最终 mpv.conf 与 input.conf
    Gen(GenArgs),
}

#[derive(Args, Debug)]
struct GenArgs {
    /// 目标平台:linux | windows | macos(缺省自动检测当前系统)
    #[arg(long, value_enum)]
    platform: Option<PlatformArg>,

    /// 输出目录,自动创建(缺省 ./dist)
    #[arg(long, default_value = "dist")]
    out: PathBuf,

    /// 只打印将生成的文件清单与行数,不写盘
    #[arg(long)]
    dry_run: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum PlatformArg {
    Linux,
    Windows,
    #[value(name = "macos")]
    MacOS,
}

impl From<PlatformArg> for Platform {
    fn from(value: PlatformArg) -> Self {
        match value {
            PlatformArg::Linux => Platform::Linux,
            PlatformArg::Windows => Platform::Windows,
            PlatformArg::MacOS => Platform::MacOS,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Gen(args) => run_gen(args),
    };
    std::process::exit(code);
}

fn run_gen(args: GenArgs) -> i32 {
    let GenArgs {
        platform,
        out,
        dry_run,
    } = args;
    let options = GenOptions {
        platform: platform.map(Platform::from),
        out,
        dry_run,
    };
    match gen::run(&options) {
        Ok(report) => {
            println!("已为 {} 生成配置:", report.platform.display_name());
            for warning in &report.warnings {
                eprintln!("警告: {warning}");
            }
            for file in &report.files {
                let action = if dry_run {
                    "将生成(未写盘)"
                } else {
                    "生成"
                };
                println!(
                    "  {action} {} ({} 行)",
                    file.path.display(),
                    file.line_count
                );
            }
            0
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn invalid_platform_value_is_rejected() {
        let error = Cli::try_parse_from(["mpv-config", "gen", "--platform", "bad"])
            .expect_err("bad platform must be rejected");
        assert!(error.to_string().contains("bad"), "{error}");
    }

    #[test]
    fn version_flag_reports_dev_version() {
        let error = Cli::try_parse_from(["mpv-config", "--version"])
            .expect_err("--version exits through clap");
        assert_eq!(error.kind(), ErrorKind::DisplayVersion);
        assert!(error.to_string().contains("0.1.0-dev"), "{error}");
    }

    #[test]
    fn gen_args_map_to_platform_with_default_out() {
        let cli = Cli::try_parse_from(["mpv-config", "gen", "--platform", "windows", "--dry-run"])
            .expect("valid args parse");
        let Command::Gen(args) = cli.command;
        assert_eq!(args.platform, Some(PlatformArg::Windows));
        assert!(args.dry_run);
        assert_eq!(args.out, PathBuf::from("dist"));
    }

    #[test]
    fn gen_without_platform_allows_host_detection() {
        let cli = Cli::try_parse_from(["mpv-config", "gen"]).expect("valid args parse");
        let Command::Gen(args) = cli.command;
        assert_eq!(args.platform, None);
        assert!(!args.dry_run);
    }

    #[test]
    fn macos_platform_accepts_macos_value() {
        let cli = Cli::try_parse_from(["mpv-config", "gen", "--platform", "macos"])
            .expect("macos value accepted");
        let Command::Gen(args) = cli.command;
        assert_eq!(args.platform, Some(PlatformArg::MacOS));
    }
}
