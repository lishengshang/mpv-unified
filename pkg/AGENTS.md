# PKG KNOWLEDGE BASE (包管理器库)

**Generated:** 2026-08-12
**Scope:** `pkg/` — 包管理器库(manifest/索引/拉取/依赖/锁/升级)。整体项目规范见根 `AGENTS.md`。

## OVERVIEW

`pkg` 是 mpv-config 的包管理器库:定义 package.yaml/index.json 规范,经 curl/unzip/git 子进程拉 GitHub Releases,做依赖解析与文件冲突检测,维护 packages.lock,并提供包商店目录合并与 app 升级能力。

## STRUCTURE

```
pkg/src/
├── lib.rs          # crate 文档 + 模块声明(所有失败为 typed error,零 panic)
├── manifest/       # mod.rs(package.yaml 双阶段校验) + tests.rs
├── index/          # mod.rs(index.json 解析/校验/名称查找) + tests.rs
├── fetch/          # mod.rs(索引拉取/缓存) error.rs package.rs(包拉取) testutil.rs + tests/
├── deps.rs         # 依赖解析/环检测/冲突(另有 deps/tests.rs)
├── lock/           # mod.rs(packages.lock 原子读写) verify.rs(verify/repair)
├── catalog.rs      # 包商店四源合并(纯函数)
├── upgrade.rs      # app 版本检查 + 引导升级(另有 upgrade/tests.rs)
└── tests.rs        # 集成测试
```

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| 改 package.yaml 规范/校验 | `manifest/mod.rs`:`Manifest::parse` 两阶段(serde_yaml 反序列化 → 逐字段校验,`PackageError` 带字段名);`Platform` 枚举 + `KNOWN_DEST_DIRS`(dest 顶层目录白名单,lock::verify 也用它扫孤儿文件) |
| 改 index.json 规范 | `index/mod.rs`:顶层 `latest_version`/`upgrade_zip_url`/`changelog_url` 是可选 T22 升级元数据;packages 条目 `name/repo/release_tag/homepage` |
| 拉索引/缓存 | `fetch/mod.rs`:`fetch_index` + 原子写 `index.json`;缓存根 `~/.cache/mpv-config`(`MPV_CONFIG_CACHE` 可覆盖);`Fetcher` trait + `MockFetcher`(测试零网络) |
| 拉单个包 | `fetch/package.rs`:`fetch_package` 流程 = release 查找(`latest`/pin tag)→ 选 asset(zip 优先)→ 下载 → `extract`(unzip/tar 子进程,zip-slip 条目预扫描)→ manifest 校验(可解析且 name 与索引一致)→ 原子落位 `packages/<name>-<version>/`;失败即清临时目录 |
| 依赖/冲突 | `deps.rs`:`resolve_install_order`(requires 建图拓扑排序,环报 `a -> b -> a` 路径);`check_conflicts`(conflicts 名称互斥)+ `check_file_conflicts`(dest 路径重叠,**相同路径=共享文件允许**);只报告不自动解决 |
| 锁文件 | `lock/mod.rs`:`Lock` 原子读写(tmp + rename),`files` 存仓库相对路径,`config_d` 记录 config.d fragment;`lock/verify.rs`:`verify` 报 missing(锁有文件无)/orphan(管理目录下无锁记录文件)两类,`repair` 需 `--yes` 否则 `RepairError::ConfirmRequired`,孤儿删除但**永不删锁记录文件与 config_d 碎片** |
| 包商店目录 | `catalog.rs`:`CatalogEntry` 合并 lock/pending(本地 git 记录)/local/index 四源 → `Status` 推导(Updatable/Installed/Pending/Available)排序列表;纯函数,解析归调用方(cli/Tauri) |
| app 升级 | `upgrade.rs`:`check_update`(本地 VERSION vs index `latest_version`);`perform_upgrade` = 下载→解压→结构校验→备份→覆盖 app 层(user 层不动)→失败回滚;确认在调用方(CLI `--yes`/UI 弹窗) |

## CONVENTIONS

- **零网络/压缩依赖**:HTTP 用 `curl` 子进程、解压用 `unzip`/`tar`(Win10+ 自带 curl/tar,unzip 需 Git Bash);`FetchError::CommandUnavailable` 提示缺命令
- **命令白名单**:只用 curl/unzip/tar/git,经 `Command::new` + 数组参数调用,**不经 shell**;测试用 `MockFetcher` 注入预设字节
- **原子写**:所有产物(cache 的 index.json、lock 文件)走 tmp + rename,失败不留半成品;fetch 临时工作目录失败必清理
- **typed error 枚举 + 中文 Display**:`FetchError`/`DepError`/`LockError`/`RepairError`/`PackageError` 带字段名或 URL 上下文
- **测试**:离线优先 — `fetch/testutil.rs` 手写 zip fixture(`zip_bytes`/`tar_gz_bytes`/`manifest_yaml`,自实现 crc32 无外部依赖);模块内 `#[cfg(test)]` + `deps/` `upgrade/` 等独立 tests.rs

## ANTI-PATTERNS

- 不在非测试代码 panic(`unwrap/expect`),错误一律 typed error 上抛
- 不经 shell 拼接执行任意命令(注入风险;子进程用参数数组)
- lock/索引不写绝对路径与隐私信息(只存仓库相对路径,`~~/` 前缀剥离)
- 不在 lock/index/缓存写入真实密钥或 token(与根红线一致,不提交)
- 不做自动升级/自动修复(repair 无 `--yes` 必须拒绝;升级确认归调用方)
- 不引入 reqwest/zip-rs 等重依赖(保持纯 std + 子进程)
