//! `mpv-config` binary: clap-driven subcommand framework.

use clap::{Args, Parser, Subcommand, ValueEnum};
use cli::doctor::{self, CheckResult, DoctorReport};
use cli::gen::{self, GenOptions};
use core::platform::{self, Platform};
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
    /// 只读自检:语法、条件指令、平台完整性、选项合法性、密钥审计
    Doctor(DoctorArgs),
}

#[derive(Args, Debug)]
struct DoctorArgs {
    /// 检查本地 VERSION 并输出升级说明
    #[arg(long)]
    upgrade_check: bool,

    /// mpv 可执行文件(--list-options 来源;缺省 "mpv")
    #[arg(long, default_value = "mpv")]
    mpv: String,
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
        Command::Doctor(args) => run_doctor(args),
    };
    std::process::exit(code);
}

fn run_doctor(args: DoctorArgs) -> i32 {
    let root = doctor::check_root();
    let report = doctor::run_with(&root, platform::detect(), &|| {
        doctor::mpv_list_options(&args.mpv)
    });
    print_report(&report);
    if args.upgrade_check {
        match doctor::upgrade_check(&root) {
            Ok(message) => println!("\n[升级检查] {message}"),
            Err(error) => eprintln!("\n[升级检查] 错误: {error}"),
        }
    }
    report.exit_code()
}

/// Print the human-readable doctor report (Chinese) and derive its status.
fn print_report(report: &DoctorReport) {
    println!("=== mpv-config doctor 检查报告 ===");
    println!("目标平台:{}", report.platform.display_name());
    for (index, check) in report.checks.iter().enumerate() {
        println!(
            "[{}/{}] {}: {}",
            index + 1,
            report.checks.len(),
            check.name,
            check_status(check)
        );
        println!("      {}", check.summary);
        for finding in &check.findings {
            println!("      - [{}] {}", finding.severity.label(), finding.message);
        }
    }
    let errors = report.error_count();
    let warnings = report.warning_count();
    println!("──────────────────────────────────────────────");
    println!(
        "结论:{errors} 错误,{warnings} 警告 → 退出码 {}",
        report.exit_code()
    );
}

fn check_status(check: &CheckResult) -> &'static str {
    if check.findings.is_empty() {
        "通过"
    } else if check
        .findings
        .iter()
        .any(|finding| finding.severity == doctor::Severity::Error)
    {
        "错误"
    } else {
        "警告"
    }
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
        let Command::Gen(args) = cli.command else {
            unreachable!("args parsed as gen")
        };
        assert_eq!(args.platform, Some(PlatformArg::Windows));
        assert!(args.dry_run);
        assert_eq!(args.out, PathBuf::from("dist"));
    }

    #[test]
    fn gen_without_platform_allows_host_detection() {
        let cli = Cli::try_parse_from(["mpv-config", "gen"]).expect("valid args parse");
        let Command::Gen(args) = cli.command else {
            unreachable!("args parsed as gen")
        };
        assert_eq!(args.platform, None);
        assert!(!args.dry_run);
    }

    #[test]
    fn macos_platform_accepts_macos_value() {
        let cli = Cli::try_parse_from(["mpv-config", "gen", "--platform", "macos"])
            .expect("macos value accepted");
        let Command::Gen(args) = cli.command else {
            unreachable!("args parsed as gen")
        };
        assert_eq!(args.platform, Some(PlatformArg::MacOS));
    }

    #[test]
    fn doctor_upgrade_check_flag_parses() {
        let cli = Cli::try_parse_from(["mpv-config", "doctor", "--upgrade-check"])
            .expect("doctor --upgrade-check parses");
        let Command::Doctor(args) = cli.command else {
            unreachable!("args parsed as doctor")
        };
        assert!(args.upgrade_check);
    }

    #[test]
    fn doctor_without_flag_parses_and_disables_upgrade_check() {
        let cli = Cli::try_parse_from(["mpv-config", "doctor"]).expect("bare doctor parses");
        let Command::Doctor(args) = cli.command else {
            unreachable!("args parsed as doctor")
        };
        assert!(!args.upgrade_check);
        assert_eq!(args.mpv, "mpv");
    }

    #[test]
    fn doctor_mpv_path_flag_parses() {
        let cli = Cli::try_parse_from(["mpv-config", "doctor", "--mpv", "/usr/bin/mpv"])
            .expect("doctor --mpv parses");
        let Command::Doctor(args) = cli.command else {
            unreachable!("args parsed as doctor")
        };
        assert_eq!(args.mpv, "/usr/bin/mpv");
    }
}
