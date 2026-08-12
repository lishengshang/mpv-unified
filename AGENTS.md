# PROJECT KNOWLEDGE BASE

**Generated:** 2026-08-12
**Commit:** 18f8cba
**Stack:** Rust (workspace: core/cli/pkg) + Tauri 2 / Vue3 / TS (ui/) + GitHub Actions

## OVERVIEW

跨平台即用 mpv 统一配置包 + 管理工具:一份配置源(base + platform 分层 + 条件指令)经生成器编译为各平台 mpv.conf;package.yaml 包管理器;Tauri GUI(方案卡片/表单/包商店/帮助)。Linux 为开发主线,macOS experimental。现役用户目录 `~/.config/mpv/` 是**只读参考**,任何代码/脚本不得写入。

## STRUCTURE

```
├── core/       # 生成器引擎(纯 Rust,零业务 IO):conf 解析/条件指令/四层合并/平台/方案/uosc/选项表
├── cli/        # mpv-config 二进制:gen/doctor/pkg/check-update/upgrade 子命令(薄编排层,调 core/pkg)
├── pkg/        # 包管理器库:manifest/index/fetch/deps/lock/upgrade
├── ui/         # Tauri 2 + Vue3 + TS(src-tauri = Rust 壳 + commands;src/ = 四页前端)
├── config/     # 分层配置源(产品核心资产,中文注释):base.conf/linux.conf/windows.conf/input.conf 变体/profiles.yaml/options-gui.yaml
├── user/       # 个人层(gitignore):user.conf/gui.conf/profiles-state.json,由工具生成
├── packages/   # 本地包 manifest(pending/ = manager.json 迁移产物)
├── docs/       # 架构文档 + tutorials/(36 篇中文教程)
├── tools/      # verify-equivalence.sh(等价验证)/build-dist.sh(zip 打包)/check-tutorials.sh
├── scripts/ shaders/ fonts/ script-opts/ vs/ icc/ osc-style/  # mpv 资产(内容)
└── .github/workflows/  # ci.yml(三平台测试)+ release.yml(tag 触发三平台 zip)
```

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| 改配置默认内容(选项/注释) | `config/base.conf`(平台差异 → linux.conf/windows.conf 或 `#@if platform==` 指令) |
| 加 mpv 选项表单项 | `config/options-gui.yaml` + `core/src/options_gui/` |
| 加/改方案卡片 | `config/profiles.yaml` + `core/src/profiles.rs`(生成 `[id]` profile 块,`profile-restore=copy-equal`) |
| 加 CLI 子命令 | `cli/src/main.rs`(clap)注册 + 逻辑模块(cli/src/*/) |
| 加包/改包格式 | `pkg/src/manifest/`(package.yaml 规范)+ 安装链 `cli/src/pkg_cmds/lifecycle/` |
| 加 Tauri command | `ui/src-tauri/src/`(lib.rs 注册)+ 前端 `ui/src/views/` |
| 长尾选项教程 | `docs/tutorials/*.md`(≥30 篇,结构校验 tools/check-tutorials.sh) |
| 打包/发布 | `tools/build-dist.sh` + `.github/workflows/release.yml`(tag `v*` 触发) |
| 升级契约 | `docs/upgrade.md`(app/user 分离,user 层永不覆盖) |

## CODE MAP

| 符号 | 位置 | 角色 |
|------|------|------|
| `conf::{parse,serialize,ConfDoc,Entry}` | core/src/conf/ | 注释无损解析器(字节级 round-trip,产品地基) |
| `cond::evaluate(doc, platform)` | core/src/cond.rs | `#@if platform==… / #@else / #@endif` 求值 |
| `merge::{merge_docs, MergeLayer}` | core/src/merge.rs | 四层合并 base→platform→packages→user(原位替换同名键,注释永不丢) |
| `platform::{detect, config_root}` | core/src/platform.rs | 平台检测 + 配置目录解析 |
| `profiles::{load, generate_profile_blocks, read/write_state}` | core/src/profiles.rs | 方案解析/块生成/状态(user/profiles-state.json) |
| `options_gui::{OptionsTable, read/write_gui_conf}` | core/src/options_gui/ | GUI 选项表 + 只写非默认的 gui.conf |
| `uosc::generate_uosc_conf` | core/src/uosc.rs | 方案 → uosc/input.conf 菜单项 |
| `manifest::{Manifest, parse}` | pkg/src/manifest/ | package.yaml 规范(serde_yaml,双阶段校验) |
| `deps::{resolve_install_order, check_conflicts}` | pkg/src/deps.rs | 拓扑排序/环检测/冲突(名称级依赖) |
| `lock::{Lock, verify, repair}` | pkg/src/lock/ | packages.lock 原子读写 + 文件一致性 |
| `fetch::{Fetcher, HttpFetcher, fetch_package}` | pkg/src/fetch/ | index.json + GitHub Releases(curl/unzip 子进程,零网络依赖) |
| `upgrade::{check_update, perform_upgrade}` | pkg/src/upgrade.rs | 版本检查 + app 层替换(user 层保留,失败回滚) |
| `catalog::CatalogEntry` | pkg/src/catalog.rs | 包商店四源合并(lock/pending/local/index) |

## CONVENTIONS

- **注释无损是硬红线**:任何 conf 读改写必须经 `core::conf` 走 parse→serialize,字节级 round-trip;解析器测试锚定两份真实配置(346 行/110KB)
- **生产代码零 panic**:禁止 `unwrap()/expect()/panic!` 于非测试代码(用 `ok_or/ok_or_else/unwrap_or` 与 typed error)
- **文件 ≤250 纯 LOC**:超限必须拆模块(历史任务全遵守,clippy --all-targets 门禁)
- **TDD**:核心逻辑先写测试;测试组织 = 模块内 `#[cfg(test)]` + core/tests、cli/tests、pkg/tests 集成
- **Rust 风格**:edition 2021、纯 std 优先(网络用 curl 子进程、zip 用 unzip,不引重依赖)、typed error enum + Display 中文
- **commit**:`<type>(<scope>): <summary>`(feat/fix/chore/refactor/docs);一个任务一个 commit;不 push(用户决定发布)
- **配置分层优先级**:gui.conf > user.conf > 方案 > package > platform > base(合并引擎消费顺序)
- **中文产品**:CLI/UI/教程/配置注释均中文(UI 有 zh/en 双语)
- **macOS 实验性**:macos.conf 缺失时 gen/doctor 只警告不报错

## ANTI-PATTERNS (THIS PROJECT)

- 绝不修改 `~/.config/mpv/`(只读参考;F4 审计会用 git status 验证)
- 绝不在 zip/仓库提交真实 API key(密钥只进 user/ 层,.example 模板除外)
- 不重造运行时方案切换(交给 mpv 内置 select 菜单 / cycle-profile 脚本)
- 不做全量 400+ 选项表单(精选 ~50-100 项,长尾走 docs/tutorials)
- 不引入 chezmoi/dotbot 等外部配置工具(借鉴语义,自研引擎)
- 不 panic、不留 TODO/FIXME

## COMMANDS

```bash
cargo test --workspace                          # 全部测试(350+)
cargo clippy --workspace --all-targets -- -D warnings   # 门禁:零告警
cargo fmt --check                               # 门禁:零 diff(根 workspace)
cd ui/src-tauri && cargo fmt --check            # ui 独立 workspace
export PATH="$HOME/.bun/bin:$PATH" && cd ui && bun run build   # 前端门禁
cargo run -p cli -- gen --platform linux --out /tmp/out        # 生成 linux 配置
cargo run -p cli -- doctor                      # 自检(0=健康/1=错误/2=警告)
bash tools/verify-equivalence.sh linux          # 配置源等价验证(拆分防退化)
bash tools/build-dist.sh linux /tmp/dist        # 组装即用 zip
cd ui && bun run tauri dev                      # GUI 开发
```

## NOTES (关键设计决策)

- **1C 混合配置模型**:成块平台差异拆文件(base+platform),零星差异用 `#@if platform==windows` 指令
- **四层叠加**:base → platform → packages → user;同名键后层原位覆盖,注释永不丢
- **升级机制**:app 层(可替换)+ user 层(保留);`perform_upgrade` 下载→备份→替换→回滚,user 层 md5 不变是硬断言
- **包格式**:package.yaml(name/version/platform/requires/conflicts/files/config);manager.json 迁移产物在 packages/pending/(17 源,git 源待安装时展开)
- **默认配置 = 用户自己的 110KB 配置**(拆分自 lishengshang/mpv-config fork);README 保留致谢,LICENSE.MD 继承 MIT
- 完整决策记录(D1-D18):`.omo/drafts/mpv-config-manager.md`;执行计划:`.omo/plans/mpv-config-manager.md`(28/28 完成)
- 已知非致命警告(冒烟时可见):simple_mpv_webui socket、history_bookmark gsub —— 不算失败
