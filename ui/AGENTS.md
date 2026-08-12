# ui/ 知识库(Tauri 2 + Vue3 + TS 图形界面)

**定位:** 方案卡片 / 配置表单 / 包商店 / 帮助 四页 GUI;Rust 壳(src-tauri)+ 前端(src/)独立 cargo workspace,不属根 workspace。

## OVERVIEW

Tauri 2 + Vue3 + TS 图形界面:四页 = 方案卡片(ProfilesView)/ 配置表单(ConfigView)/ 包商店(StoreView)/ 帮助(HelpView)。

## STRUCTURE

```
ui/
├── package.json          # bun 管理;scripts: dev/build(preview/tauri)
├── vite.config.ts        # 固定端口 1420(strictPort),Tauri 依赖
├── index.html / tsconfig.json / tsconfig.node.json
├── src/
│   ├── main.ts           # 入口,挂载 App + router
│   ├── App.vue           # 侧边栏导航 + design tokens(:root CSS 变量)
│   ├── router/index.ts   # createWebHashHistory 四页路由, / → /profiles
│   ├── views/            # ProfilesView / ConfigView / StoreView / HelpView
│   ├── i18n/             # zh-CN.ts(含 MessageSchema)/ en-US.ts / index.ts(t + toggleLocale)
│   ├── composables/      # useToasts.ts(操作反馈 toast)
│   ├── components/       # StatePanel.vue / ToastStack.vue
│   └── env.d.ts
└── src-tauri/            # 独立 cargo workspace
    ├── src/lib.rs        # 全部 commands 注册(generate_handler)
    ├── src/commands.rs   # 方案/表单/更新升级 commands(直接调 core/cli 函数)
    ├── src/store.rs      # 包商店 commands(list/update/install/uninstall/update_package)
    ├── src/help.rs       # 帮助页 commands(get/save_user_conf, tutorials)
    ├── src/main.rs
    └── tauri.conf.json   # identifier com.mpvconfig.app, 1100x700 窗口
```

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| 方案卡片页(列出/应用方案,profiles-state.json) | `src/views/ProfilesView.vue` ↔ `commands::list_profiles/get/set_profile_state/regenerate` |
| 配置表单页(options-gui.yaml 表单 → 只写非默认 gui.conf) | `src/views/ConfigView.vue` ↔ `commands::list_options/get/save/reset_gui_value/uosc_status` |
| 包商店页(四源 catalog 合并:lock/pending/local/index) | `src/views/StoreView.vue` ↔ `store.rs`(内部走 `pkg::catalog` + `cli::pkg_cmds::lifecycle`) |
| 帮助页(教程列表/正文 + user.conf 编辑器 + 关于/升级) | `src/views/HelpView.vue` ↔ `help.rs`(editor 只碰 user/user.conf)+ `commands::check_update/perform_upgrade` |
| 文案/导航 | `src/i18n/zh-CN.ts`(`MessageSchema`)与 `en-US.ts` 成对改;`App.vue` 侧边栏用 `t("nav.*")` |
| 新 command 接线 | `src-tauri/src/lib.rs` 注册 + 对应模块;core/cli 侧函数签名见根 AGENTS.md |

## CONVENTIONS

- **前端零 shell/零网络**:不 `fetch`、不 spawn 进程;一切 IO 走 `invoke()` → Tauri commands;commands 直接调 core/pkg/cli crate 函数(见 store.rs 头注释)
- **i18n 编译期校验**:`MessageSchema` 模板字面量类型 → `t("nav.profiles")` 拼错、zh/en 漏译多译在 `bun run build`(vue-tsc --noEmit)报错
- **design tokens 唯一来源**:`App.vue` `:root` 变量 `--accent/--accent-soft/--surface/--surface-hover/--border/--text/--text-muted/--ok/--warn/--danger/--radius-card/--radius-control`;视图不写死色值
- **键盘可达性**:所有可交互元素 `:focus-visible` 必须有可见焦点环(App.vue 全局样式)
- **命令**:`bun run dev/build`;`bun run tauri dev`(GUI 开发);src-tauri 独立 workspace → `cd src-tauri && cargo fmt --check` 单独跑
- **错误处理**:Rust command 返回 `Result<_, String>`,前端 toast 展示(useToasts)

## ANTI-PATTERNS

- 前端不 `fetch` 网络(包下载/索引走 Rust 侧 curl 子进程)
- 不放密钥/凭据在前端代码或 dist
- 编辑器只写 user 层(user/user.conf),永不触碰 app 层配置
- 不引 Monaco 等重编辑器依赖 —— textarea 即可
- Rust 侧不 panic(生产代码零 unwrap/expect,lib.rs 的 `expect` 是 Tauri 固定样板)
- 不改 `~/.config/mpv/`(只读参考,见根 AGENTS.md)

## NOTES

- `main.rs` 调 `lib.rs::run()`;移动端入口宏已预留(`#[cfg_attr(mobile, ...)]`)
- 包商店 install/uninstall/update 的 rollback 与共享文件安全由 `cli` lifecycle 负责,UI 只展示结果(InstallOutcome 等 → toast)
- 教程数据源 `docs/tutorials/`,经 `help::list_tutorials/get_tutorial` 读取(结构校验 tools/check-tutorials.sh)
