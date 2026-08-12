//! Exit-code contract of the `doctor` suite: 0 = healthy, 1 = errors,
//! 2 = warnings only.

mod common;

use common::{healthy_root, run, TestDir};

#[test]
fn healthy_repo_passes_all_checks_with_exit_0() {
    let dir = TestDir::new("healthy");
    healthy_root(dir.path());

    let report = run(dir.path());

    assert_eq!(report.exit_code(), 0);
    for check in &report.checks {
        assert!(
            check.findings.is_empty(),
            "{}: {:?}",
            check.name,
            check.findings
        );
    }
    assert_eq!(report.checks.len(), 5);
}

#[test]
fn syntax_error_reports_layer_and_line() {
    let dir = TestDir::new("syntax");
    healthy_root(dir.path());
    common::write(&dir.path().join("config/linux.conf"), "hwdec=\"unclosed\n");

    let report = run(dir.path());

    let messages = common::finding_messages(&report, "语法检查");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("config/linux.conf") && m.contains("line 1")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn user_conf_syntax_error_reports_layer_and_line() {
    let dir = TestDir::new("syntax-user");
    healthy_root(dir.path());
    common::write(
        &dir.path().join("user/user.conf"),
        "vo=gpu\nbad=\"unclosed\n",
    );

    let report = run(dir.path());

    let messages = common::finding_messages(&report, "语法检查");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("user/user.conf") && m.contains("line 2")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn missing_base_conf_is_an_error() {
    let dir = TestDir::new("no-base");
    healthy_root(dir.path());
    std::fs::remove_file(dir.path().join("config/base.conf")).expect("remove base");

    let report = run(dir.path());

    let messages = common::finding_messages(&report, "平台完整性");
    assert!(
        messages.iter().any(|m| m.contains("config/base.conf")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn missing_current_platform_layer_is_an_error() {
    let dir = TestDir::new("no-platform");
    healthy_root(dir.path());
    std::fs::remove_file(dir.path().join("config/linux.conf")).expect("remove linux");

    let report = run(dir.path());

    let messages = common::finding_messages(&report, "平台完整性");
    assert!(
        messages.iter().any(|m| m.contains("config/linux.conf")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn missing_macos_layer_warns_only() {
    let dir = TestDir::new("no-macos");
    healthy_root(dir.path());
    std::fs::remove_file(dir.path().join("config/macos.conf")).expect("remove macos");

    let report = run(dir.path());

    let messages = common::finding_messages(&report, "平台完整性");
    assert!(
        messages.iter().any(|m| m.contains("config/macos.conf")),
        "{messages:?}"
    );
    assert_eq!(
        common::findings_of(&report, "平台完整性")[0].severity,
        cli::doctor::Severity::Warning
    );
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn missing_mpv_skips_option_check_with_warning() {
    let dir = TestDir::new("no-mpv");
    healthy_root(dir.path());

    let report = cli::doctor::run_with(
        dir.path(),
        core::platform::Platform::Linux,
        &common::source_none(),
    );

    let messages = common::finding_messages(&report, "选项合法性");
    assert!(
        messages.iter().any(|m| m.contains("跳过选项校验")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 2);
}
