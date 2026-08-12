//! Report types and formatting for `pkg migrate-manager`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigratedEntry {
    pub name: String,
    pub git: String,
    pub dest: String,
}

/// One entry that could not be migrated, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedEntry {
    /// 1-based position in the source file.
    pub index: usize,
    pub git: String,
    pub reason: String,
}

/// Result of a migration run: successes, failures and notes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub input_display: String,
    pub out_display: String,
    pub report_display: String,
    pub succeeded: Vec<MigratedEntry>,
    pub failed: Vec<FailedEntry>,
    pub notes: Vec<String>,
}

impl MigrationReport {
    #[must_use]
    pub fn success_count(&self) -> usize {
        self.succeeded.len()
    }

    #[must_use]
    pub fn failure_count(&self) -> usize {
        self.failed.len()
    }

    /// Markdown written to the report file (`docs/migration-report.md`).
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut markdown = format!(
            "# manager.json 迁移报告\n\n- 输入:`{input}`\n- 输出目录:`{out}`\n- 结果:成功 {ok} / 失败 {fail}\n\n",
            input = self.input_display,
            out = self.out_display,
            ok = self.success_count(),
            fail = self.failure_count(),
        );
        markdown.push_str(&format!(
            "## 成功({})\n\n| # | name | git | dest |\n| --- | --- | --- | --- |\n",
            self.success_count()
        ));
        for (offset, entry) in self.succeeded.iter().enumerate() {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                offset + 1,
                entry.name,
                entry.git,
                entry.dest
            ));
        }
        markdown.push_str(&format!("\n## 失败({})\n\n", self.failure_count()));
        if self.failed.is_empty() {
            markdown.push_str("(无)\n");
        } else {
            markdown.push_str("| # | git | 原因 |\n| --- | --- | --- |\n");
            for entry in &self.failed {
                markdown.push_str(&format!(
                    "| {} | {} | {} |\n",
                    entry.index, entry.git, entry.reason
                ));
            }
        }
        if !self.notes.is_empty() {
            markdown.push_str("\n## 备注\n\n");
            for note in &self.notes {
                markdown.push_str(&format!("- {note}\n"));
            }
        }
        markdown
    }
}

impl fmt::Display for MigrationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== manager.json 迁移报告 ===")?;
        writeln!(f, "输入: {}", self.input_display)?;
        writeln!(
            f,
            "输出: {}({} 条记录)",
            self.out_display,
            self.success_count()
        )?;
        writeln!(
            f,
            "成功 {} / 失败 {}",
            self.success_count(),
            self.failure_count()
        )?;
        for entry in &self.succeeded {
            writeln!(
                f,
                "  [ok] {} <- {} -> {}",
                entry.name, entry.git, entry.dest
            )?;
        }
        for entry in &self.failed {
            writeln!(
                f,
                "  [fail] #{} {}: {}",
                entry.index, entry.git, entry.reason
            )?;
        }
        for note in &self.notes {
            writeln!(f, "  备注: {note}")?;
        }
        writeln!(f, "报告已写入: {}", self.report_display)?;
        Ok(())
    }
}
