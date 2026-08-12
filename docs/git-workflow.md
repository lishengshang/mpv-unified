# Git 开发规范

本文件是 mpv-config 仓库(origin: `github.com/lishengshang/mpv-unified`)的 git 开发规范,agent 与项目所有者共同遵守。

## 1. 分支模型(GitHub Flow)

- `main` 永远可发布:main 上的每一个提交都应当通过全部门禁、可以直接打 tag 发布
- 功能分支:`feature/描述`(如 `feature/package-verify`)。大功能(跨多目录、多轮改动)必须开分支
- 单人小改动(≤2 文件、单原子逻辑)可以直接提交到 main
- 分支命名建议:`feature/<kebab-case 描述>`

## 2. 提交规范

### 2.1 格式

```
<type>(<scope>): <摘要>
```

| type | 含义 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修 bug |
| `chore` | 杂务(依赖、格式化、构建脚本) |
| `docs` | 仅文档 |
| `refactor` | 重构,行为不变 |
| `test` | 仅测试 |
| `ci` | CI/工作流改动 |

| scope | 对应路径 |
|-------|----------|
| `core` | core/ |
| `cli` | cli/ |
| `pkg` | pkg/ |
| `ui` | ui/ |
| `config` | config/ 与配置资产 |
| `docs` | docs/、AGENTS.md |
| `tools` | tools/ |

摘要用英文、祈使句、小写开头、≤72 字符。示例:`feat(ui): add package store page`、`fix(cli): normalize installed-file paths in lock`、`docs: mark panic tech-debt resolved in AGENTS.md`。

### 2.2 原子提交硬规则

- **3+ 文件 → 至少 2 个提交;5+ 文件 → 至少 3 个提交**
- **不同目录(不同 scope)的改动必须拆成不同提交**
- 实现与它的测试放同一提交(测试+实现 = 一个不可分割的行为单元)
- 每个提交必须可以独立 revert:一个提交只做一件事,revert 它不破坏其他提交
- 不把无关改动(顺手格式化、无关重构)塞进正在做的提交

### 2.3 Commit Body

需要解释"为什么"时写 body,格式:

```
<type>(<scope>): <摘要>

<动机:为什么做这个改动、背景约束>

<改动:做了什么、关键取舍>

Closes #<issue>   # 如适用
```

简单改动可省略 body,只留一行标题。

## 3. Agent 纪律(重要)

- **Agent 只做本地 commit,绝不 push**。远程推送由项目所有者审查后执行
- **一个任务一个提交**;任务中途的探索性改动不零碎提交
- **提交前门禁全绿**(全过才允许 commit):

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
export PATH="$HOME/.bun/bin:$PATH" && cd ui && bun run build
```

- 涉及 `config/` 资产时额外跑 `bash tools/verify-equivalence.sh linux`(等价验证)
- **绝不提交敏感内容**:`user/`(仅 `user.example.conf` 可入库)、`*.local.conf`、密钥/API key、`target/`、`node_modules/`、`.omo/`
  - `.gitignore` 已挡 `/target`、`.omo/`、`user/*`、`*.local.conf`;提交前仍要 `git status` double-check 没有意外文件
- 提交前检查 `git status` 与 `git diff`,只 stage 本任务的文件(`git add <具体路径>`,不用 `git add -A` 盲加)

## 4. 版本发布(SemVer)

- 版本号遵循 SemVer:`MAJOR.MINOR.PATCH`;当前版本见根目录 `VERSION`
- **`git tag vX.Y.Z` 触发 CI 三平台 zip**(`.github/workflows/release.yml`,tag `v*`)
- 每版更新 `CHANGELOG.md`(新增条目 + 指向对应 commits)
- **tag 前检查清单**:
  1. `cargo test --workspace` 全绿
  2. 等价验证通过(`bash tools/verify-equivalence.sh linux`,窗口平台 `windows`)
  3. `bash tools/build-dist.sh <platform> /tmp/dist` 能产出可用 zip
  4. `git status` 干净,无未提交改动
  5. CHANGELOG.md 已更新,版本号已 bump

```bash
git tag v0.3.0
git push origin v0.3.0      # 由项目所有者执行
```

## 5. PR 流程(开源协作后)

1. 功能分支 `feature/描述` 上完成开发,本地门禁全绿
2. `git push origin feature/描述`(所有者或协作者执行)
3. `gh pr create` 按 PR 模板填写(动机/改动/验证)
4. CI 门禁(`.github/workflows/ci.yml` 三平台)通过
5. 评审通过后 **squash merge** 到 main(保持 main 历史线性干净)
6. 删除已合并的功能分支

## 6. 危险红线

- **绝不 `git push --force`**;确需覆盖远端时用 `git push --force-with-lease`(先确认没有他人提交)
- **绝不 rebase `main`**(main 只接受 merge/squash merge 与直接提交)
- 误操作(错删分支、reset 过头)用 `git reflog` 找回:`git reflog` → 找到目标 commit → `git reset --hard <hash>` / `git branch <name> <hash>` 恢复
- 不 amend 已推送的提交;不制造空提交(`--allow-empty`)

## 7. 常用命令速查

```bash
# 状态
git status                     # 工作区状态
git diff                       # 未暂存改动
git log --oneline -10          # 最近提交

# 提交
git add <具体路径>             # 精确 stage,不用 git add -A
git commit -m "feat(cli): ..." # 按 2.1 格式
git commit --amend             # 仅未推送时使用

# 分支
git checkout -b feature/xxx    # 开功能分支
git branch -d feature/xxx      # 合并后删除分支

# 回退
git restore <文件>             # 丢弃工作区改动
git revert <hash>              # 安全撤销已提交改动(生成反向提交)
git reset --hard <hash>        # 危险操作,先看 git reflog

# 发布
git tag v0.3.0                 # 打 tag(触发 release CI)
git push origin v0.3.0         # 推送 tag(所有者执行)
```
