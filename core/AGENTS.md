# CORE KNOWLEDGE BASE (生成器引擎)

**Generated:** 2026-08-12
**范围:** 仅 core/ 内部;项目级总览/命令/红线见根 `AGENTS.md`,不重复

## OVERVIEW

mpv-config 生成器引擎库:纯 Rust、零业务 IO,把配置源编译为 mpv.conf(conf 解析 → cond 求值 → 四层合并 → profiles/uosc 生成)。

## STRUCTURE

```
core/src/
├── lib.rs          # crate 根:pub mod 声明 + crate 级文档
├── conf/           # 注释无损 mpv.conf 解析器/序列化器(产品地基)
│   ├── mod.rs      # 公共 API:parse/serialize/ConfDoc/Entry 再导出
│   ├── parser.rs   # 行式解析 + ParseError + MAX_LINE_BYTES
│   ├── model.rs    # ConfDoc/Entry 数据模型(key/value/profile/raw_line)
│   ├── serializer.rs # 字节级 round-trip 输出
│   └── 仅测试 errors.rs roundtrip.rs tests.rs testutil.rs
├── cond.rs         # #@if platform== 条件指令求值
├── merge.rs        # 四层合并 base→platform→packages→user
├── platform.rs     # 平台检测 + config_root + mpv_available
├── profiles.rs     # profiles.yaml 解析 + [id] 块生成 + 状态读写
├── options_gui/    # 选项表 + 只写非默认 gui.conf
│   ├── mod.rs / table.rs / gui_conf.rs / ops.rs / validate.rs
└── uosc.rs         # 方案 → uosc 菜单联动
```

## WHERE TO LOOK

| 任务 | 位置 |
|------|------|
| 改解析/序列化行为 | `conf/parser.rs` + `conf/serializer.rs`(必同步 `conf/tests.rs` 锚定测试) |
| 改数据模型 | `conf/model.rs`(Entry 变体、raw_line 字段) |
| 条件指令语法/求值 | `cond.rs`(`evaluate(doc, platform)`;`#@if/#@else/#@endif`、`==`/`!=`、可嵌套) |
| 合并语义 | `merge.rs`(`merge_docs` + `MergeLayer::parse`;同名键原位替换、注释/空行永不丢) |
| 平台/配置目录 | `platform.rs`(`detect`/`config_root`/`mpv_available`;MPV_HOME 完全覆盖) |
| 方案卡片 | `profiles.rs`(`load`/`generate_profile_blocks`/`read_state`/`write_state`;块只置文件尾,`profile-restore=copy-equal` 首行) |
| 选项表单 | `options_gui/table.rs`(OptionsTable)、`gui_conf.rs`(只写非默认项)、`ops.rs`(差分)、`validate.rs` |
| uosc 菜单 | `uosc.rs`(`generate_uosc_conf`;apply-profile bindings) |
| 新增公共符号 | `lib.rs` 注册 pub mod + 更新 crate 级文档 |

## CONVENTIONS

- **纯 std**:核心逻辑零网络、零文件 IO——文件访问全在 cli/ 层,core 只收 ConfDoc/纯数据
- **raw_line 是注释无损机制核心**:每行原始文本存于 Entry,serialize 逐字节还原(含尾随空白/末尾换行);读改写一律 parse→serialize,不许破坏
- **生产代码零 panic**:禁 unwrap/expect/panic!(用 ok_or/typed error;中文消息:platform 的 `ConfigRootError`、profiles 的 `ProfilesError`)
- **测试**:各模块内 `#[cfg(test)]`;conf/ 有专属测试文件(roundtrip.rs 锚定两份真实配置:346 行/110KB),改解析器必须同步
- **≤250 LOC/文件**:超限拆模块(options_gui/ 已拆 table/gui_conf/ops/validate 四件)
- 平台枚举 linux/windows/macos;macos 缺配置只警告不报错(cond.rs D14 语法)

## ANTI-PATTERNS

- 不把 IO/网络/文件路径逻辑写进 core(属 cli/ 编排层)
- 不绕过 conf 解析器直接改字符串/行拼接(破坏注释无损,roundtrip 测试必炸)
- 不 panic、不引重依赖(zip/网络由 cli/ 用子进程处理)
- 不把 `#@` 指令当普通注释丢弃:cond.rs 按 `#@` 前缀识别,先保 round-trip 再求值
