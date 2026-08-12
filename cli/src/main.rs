//! `mpv-config` binary: clap-driven subcommand framework.

use clap::{Args, Parser, Subcommand, ValueEnum};
use cli::doctor::{self, CheckResult, DoctorReport};
use cli::gen::{self, GenOptions};
use cli::pkg_cmds::lifecycle;
use cli::pkg_cmds::migrate;
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
    /// 包管理
    Pkg(PkgArgs),
    /// 检查更新:对比本地 VERSION 与远程索引的 latest_version
    CheckUpdate(CheckUpdateArgs),
    /// 执行升级:下载新版 zip → 备份 app 层 → 替换(保留 user/),失败自动回滚
    Upgrade(UpgradeArgs),
}

#[derive(Args, Debug)]
struct PkgArgs {
    #[command(subcommand)]
    command: PkgCommand,
}

#[derive(Subcommand, Debug)]
enum PkgCommand {
    /// 一键迁移现役 manager.json 为 packages/pending/ 待安装记录(T14 安装时消费)
    MigrateManager(MigrateArgs),

    /// 安装包:来源解析(pending git 记录 → 本地 packages/ → 索引)→ 校验 → 拷贝
    Install(InstallArgs),

    /// 卸载包:按 packages.lock 反查文件清单;共享文件仅当最后使用者才删除
    Uninstall(UninstallArgs),

    /// 更新包(不指定名称则更新全部已装包):对比最新版本 → 原子替换(备份到缓存)
    Update(UpdateArgs),

    /// 拉取并校验远程 index.json 到缓存(显式命令,绝不静默更新)
    UpdateIndex(UpdateIndexArgs),

    /// 校验 packages.lock 与实际安装文件的一致性(缺失/多余文件清单)
    Verify,

    /// 修复 packages.lock 反映的问题:清理 lock 未记录的多余文件(需 --yes)
    Repair(RepairArgs),
}

#[derive(Args, Debug)]
struct InstallArgs {
    /// 包名(对应 packages/pending/<name>.yaml、packages/<name>.yaml 或索引中的名称)
    name: String,
}

#[derive(Args, Debug)]
struct UninstallArgs {
    /// 包名(须在 packages.lock 中)
    name: String,
}

#[derive(Args, Debug)]
struct UpdateArgs {
    /// 包名(缺省更新全部已装包)
    name: Option<String>,
}

#[derive(Args, Debug)]
struct UpdateIndexArgs {
    /// 远程索引 URL(缺省官方索引仓库)
    #[arg(long, default_value = pkg::fetch::DEFAULT_INDEX_URL)]
    index_url: String,
}

#[derive(Args, Debug)]
struct MigrateArgs {
    /// manager.json 路径(缺省 ~/.config/mpv/manager.json)
    path: Option<PathBuf>,

    /// 输出目录,自动创建(缺省 packages/pending)
    #[arg(long, default_value = "packages/pending")]
    out: PathBuf,

    /// 迁移报告输出路径(缺省 docs/migration-report.md)
    #[arg(long, default_value = "docs/migration-report.md")]
    report: PathBuf,
}

#[derive(Args, Debug)]
struct RepairArgs {
    /// 删除 lock 未记录的多余文件(缺省只列出并拒绝执行)
    #[arg(long)]
    yes: bool,
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
struct CheckUpdateArgs {
    /// 远程索引 URL(缺省官方索引仓库)
    #[arg(long, default_value = pkg::fetch::DEFAULT_INDEX_URL)]
    index_url: String,
}

#[derive(Args, Debug)]
struct UpgradeArgs {
    /// 远程索引 URL(缺省官方索引仓库)
    #[arg(long, default_value = pkg::fetch::DEFAULT_INDEX_URL)]
    index_url: String,

    /// 确认执行升级:替换 app 层是破坏性操作,必须显式确认
    #[arg(long)]
    yes: bool,
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
        Command::Pkg(args) => run_pkg(args),
        Command::CheckUpdate(args) => cli::upgrade_cmds::run_check_update(&args.index_url),
        Command::Upgrade(args) => cli::upgrade_cmds::run_upgrade(&args.index_url, args.yes),
    };
    std::process::exit(code);
}

fn run_pkg(args: PkgArgs) -> i32 {
    match args.command {
        PkgCommand::MigrateManager(args) => run_migrate_manager(args),
        PkgCommand::Install(args) => run_install(args),
        PkgCommand::Uninstall(args) => run_uninstall(args),
        PkgCommand::Update(args) => run_update(args),
        PkgCommand::UpdateIndex(args) => run_update_index(args),
        PkgCommand::Verify => run_verify(),
        PkgCommand::Repair(args) => run_repair(args),
    }
}

fn pkg_roots() -> Result<(PathBuf, PathBuf), lifecycle::LifecycleError> {
    Ok((gen::repo_root(), lifecycle::default_cache_dir()?))
}

fn run_install(args: InstallArgs) -> i32 {
    let (root, cache) = match pkg_roots() {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    let options = lifecycle::InstallOptions {
        name: args.name,
        repo_root: root,
        cache_dir: cache,
        platform: platform::detect(),
    };
    match lifecycle::install(&options) {
        Ok(lifecycle::InstallOutcome::Installed(report)) => {
            println!("{report}");
            0
        }
        Ok(lifecycle::InstallOutcome::SkippedPlatform {
            name,
            package_platform,
            target,
        }) => {
            println!(
                "跳过安装:包 \"{name}\" 目标平台为 {package_platform},当前平台为 {}",
                target.display_name()
            );
            0
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
}

fn run_uninstall(args: UninstallArgs) -> i32 {
    let (root, _) = match pkg_roots() {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    let options = lifecycle::UninstallOptions {
        name: args.name,
        repo_root: root,
    };
    match lifecycle::uninstall(&options) {
        Ok(report) => {
            println!("{report}");
            0
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
}

fn run_update(args: UpdateArgs) -> i32 {
    let (root, cache) = match pkg_roots() {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    let options = lifecycle::UpdateOptions {
        name: args.name,
        repo_root: root,
        cache_dir: cache,
    };
    match lifecycle::update(&options) {
        Ok(report) => {
            print!("{report}");
            0
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
}

fn run_update_index(args: UpdateIndexArgs) -> i32 {
    let (_, cache) = match pkg_roots() {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    let options = lifecycle::UpdateIndexOptions {
        index_url: args.index_url,
        cache_dir: cache,
    };
    match lifecycle::update_index(&options) {
        Ok(path) => {
            println!("索引已更新并校验通过:{}", path.display());
            0
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
}

fn run_migrate_manager(args: MigrateArgs) -> i32 {
    let input = match args.path {
        Some(path) => path,
        None => match dirs::config_dir() {
            Some(base) => base.join("mpv/manager.json"),
            None => PathBuf::from("manager.json"),
        },
    };
    match migrate::run(&input, &args.out, &args.report) {
        Ok(report) => {
            println!("{report}");
            0
        }
        Err(error) => {
            eprintln!("错误: 迁移失败: {error}");
            1
        }
    }
}

fn run_verify() -> i32 {
    let (root, _) = match pkg_roots() {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    let lock = match lifecycle::lock::read(&root.join("packages.lock")) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    match lifecycle::verify(&lock, &root) {
        Ok(report) => {
            println!("{report}");
            if report.is_healthy() {
                0
            } else {
                1
            }
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
}

fn run_repair(args: RepairArgs) -> i32 {
    let (root, _) = match pkg_roots() {
        Ok(roots) => roots,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    let lock = match lifecycle::lock::read(&root.join("packages.lock")) {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!("错误: {error}");
            return 1;
        }
    };
    match lifecycle::repair(&lock, &root, args.yes) {
        Ok(report) => {
            println!("{report}");
            if report.missing.is_empty() {
                0
            } else {
                1
            }
        }
        Err(error) => {
            eprintln!("错误: {error}");
            1
        }
    }
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
                if file.copied {
                    println!("  {action} {} (复制)", file.path.display());
                } else {
                    println!(
                        "  {action} {} ({} 行)",
                        file.path.display(),
                        file.line_count
                    );
                }
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

    #[test]
    fn pkg_migrate_manager_parses_with_defaults() {
        let cli = Cli::try_parse_from(["mpv-config", "pkg", "migrate-manager"])
            .expect("bare migrate-manager parses");
        let Command::Pkg(args) = cli.command else {
            unreachable!("args parsed as pkg")
        };
        let PkgCommand::MigrateManager(migrate) = args.command else {
            unreachable!("args parsed as migrate-manager")
        };
        assert!(migrate.path.is_none(), "input path defaults to config dir");
        assert_eq!(migrate.out, PathBuf::from("packages/pending"));
        assert_eq!(migrate.report, PathBuf::from("docs/migration-report.md"));
    }

    #[test]
    fn pkg_migrate_manager_accepts_path_and_out() {
        let cli = Cli::try_parse_from([
            "mpv-config",
            "pkg",
            "migrate-manager",
            "/tmp/manager.json",
            "--out",
            "/tmp/pending",
            "--report",
            "/tmp/report.md",
        ])
        .expect("positional path and flags parse");
        let Command::Pkg(args) = cli.command else {
            unreachable!("args parsed as pkg")
        };
        let PkgCommand::MigrateManager(migrate) = args.command else {
            unreachable!("args parsed as migrate-manager")
        };
        assert_eq!(migrate.path, Some(PathBuf::from("/tmp/manager.json")));
        assert_eq!(migrate.out, PathBuf::from("/tmp/pending"));
        assert_eq!(migrate.report, PathBuf::from("/tmp/report.md"));
    }
}
