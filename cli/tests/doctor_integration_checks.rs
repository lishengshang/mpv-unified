//! Per-check defect tests of the `doctor` suite: conditional directives,
//! option validity and the secret audit.

mod common;

use common::{finding_messages, findings_of, healthy_root, run, TestDir};

#[test]
fn unclosed_directive_reports_opening_line() {
    let dir = TestDir::new("cond-unclosed");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("config/base.conf"),
        "#@if platform==linux\nvo=gpu\n",
    );

    let report = run(&dir.path());

    let messages = finding_messages(&report, "条件指令检查");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("config/base.conf") && m.contains("line 1")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn orphan_endif_is_reported() {
    let dir = TestDir::new("cond-orphan");
    healthy_root(&dir.path());
    common::write(&dir.path().join("config/linux.conf"), "#@endif\n");

    let report = run(&dir.path());

    let messages = finding_messages(&report, "条件指令检查");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("config/linux.conf") && m.contains("line 1")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn input_layer_directive_error_is_reported() {
    let dir = TestDir::new("input-cond");
    healthy_root(&dir.path());
    common::write(&dir.path().join("config/input.conf"), "#@endif\n");

    let report = run(&dir.path());

    let messages = finding_messages(&report, "条件指令检查");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("config/input.conf") && m.contains("line 1")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn package_fragment_syntax_error_is_reported() {
    let dir = TestDir::new("package-error");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("config.d/packages/a.conf"),
        "bad=\"quote\n",
    );

    let report = run(&dir.path());

    let messages = finding_messages(&report, "语法检查");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("config.d/packages/a.conf") && m.contains("line 1")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn unknown_option_warns_and_exits_2() {
    let dir = TestDir::new("unknown-option");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("user/user.conf"),
        "no-osd-bar\ndefinitely-not-an-option=1\n",
    );

    let report = run(&dir.path());

    let messages = finding_messages(&report, "选项合法性");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("definitely-not-an-option")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("no-osd-bar")),
        "no- prefix must not warn: {messages:?}"
    );
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn profile_pseudo_options_do_not_warn() {
    let dir = TestDir::new("profile-pseudo");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("user/user.conf"),
        "[profile test]\nprofile-restore=copy-equal\nnot-an-option=1\n",
    );

    let report = run(&dir.path());

    let messages = finding_messages(&report, "选项合法性");
    assert!(
        messages.iter().any(|m| m.contains("not-an-option")),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("profile-restore")),
        "{messages:?}"
    );
    assert_eq!(messages.len(), 1);
}

#[test]
fn secret_file_in_user_dir_warns() {
    let dir = TestDir::new("secret-user");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("user/keys.txt"),
        "sk-abcdef1234567890abcdef1234567890\n",
    );

    let report = run(&dir.path());

    let messages = finding_messages(&report, "密钥审计");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("user/keys.txt") && m.contains("sk-")),
        "{messages:?}"
    );
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn secret_scan_covers_config_dir() {
    let dir = TestDir::new("secret-config");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("config/linux.conf"),
        "hwdec=auto\ntoken=abc123\n",
    );

    let report = run(&dir.path());

    let messages = finding_messages(&report, "密钥审计");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("config/linux.conf") && m.contains("token")),
        "{messages:?}"
    );
}

#[test]
fn example_secret_files_are_ignored() {
    let dir = TestDir::new("secret-example");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("user/keys.example.txt"),
        "sk-abcdef1234567890abcdef1234567890\n",
    );

    let report = run(&dir.path());

    assert!(
        findings_of(&report, "密钥审计").is_empty(),
        "{:?}",
        findings_of(&report, "密钥审计")
    );
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn clean_prose_comments_do_not_trigger_secret_scan() {
    let dir = TestDir::new("secret-clean");
    healthy_root(&dir.path());
    common::write(
        &dir.path().join("user/user.conf"),
        "# 写真实 API key / token。\n## 占位:YOUR_WHISPER_TOKEN\nno-osd-bar\n",
    );

    let report = run(&dir.path());

    assert!(
        findings_of(&report, "密钥审计").is_empty(),
        "{:?}",
        findings_of(&report, "密钥审计")
    );
}
