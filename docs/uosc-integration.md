# uosc 联动(uosc-integration)

## 原理

uosc 5.x(本仓库内置于 `scripts/uosc/`)的右键上下文菜单**不是**由
`script-opts/uosc.conf` 配置,而是解析 `input.conf` 中带 `#menu:`(或 `#!`)
注释的按键绑定行构建(见 `scripts/uosc/README.md` → "Adding items to menu"):

```text
#  按键       命令                   #menu: 菜单路径
#  apply-profile cinema          #menu: 方案 > 高清观影
```

- 键为 `#`:不绑定任何按键,仅作为菜单项存在。
- 命令 `apply-profile <id>`:mpv 原生命令(mpv ≥ 0.32;本项目声明最低 0.36),
  运行时切换方案。
- `方案 >` 前缀:归入「方案」子菜单,支持无限嵌套。

本仓库 `config/input.conf` 已有的"配置组切换"菜单项(如
`#  apply-profile FSRCNNX  #menu: 其它 > 常规配置组 > …`)正是同一套语法,
生成器完全复用该惯例。

## 生成产物(mpv-config gen)

启用方案 ≥ 1 且检测到 uosc(存在 `scripts/uosc.lua` 或
`scripts/uosc/main.lua`)时,`gen` 输出两处:

1. **`dist/input.conf` 末尾追加菜单项**(真正生效的位置):
   ```text
   # ===== uosc 方案切换菜单(由 mpv-config 生成,任务 23) =====
   #  apply-profile cinema          #menu: 方案 > 高清观影
   ```
2. **`dist/script-opts/uosc.conf`**(补丁,自文档化):若仓库已有
   `script-opts/uosc.conf`,先原样拷贝再在末尾追加补丁(绝不覆盖丢失原配置);
   补丁全部为注释行,uosc 本身会忽略,仅作说明与手工合并参考。

**降级**:未检测到 uosc 时,`gen` 输出警告「未检测到 uosc…已跳过 uosc 方案
切换菜单」,不报错、不中断;安装 uosc 后重新生成即可。

**删除菜单**:停用全部方案后重新生成,菜单项随方案块一并消失。

## 要求

- mpv ≥ 0.36(`apply-profile` 命令;与 README 最低版本声明一致)。
- uosc 5.x(菜单语法随版本演进,生成器按本仓库内置版本输出)。
- 方案 id/name 来自 `config/profiles.yaml`;名称含 `>` 会被 uosc 解析为
  子菜单嵌套(不建议使用)。
