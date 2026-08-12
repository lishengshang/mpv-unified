# profiles — profile 块用法

## 用途

profile 是 mpv 配置的"子配置":一段命名配置块,启动/播放中按需
套用。语法即配置文件里的 `[name]` 段,内容与普通选项一致。
典型用途:电影/动漫两套画质参数、不同设备(耳机/外放)音频设置、
调试专用 profile(verbose 日志)。

本仓库把 profile 机制作为"方案"功能的载体:方案页(UI 主界面)
启用方案后,gen 会生成带 `profile-restore=copy-equal` 的 profile 块
追加到最终 mpv.conf,播放时用内置 select/菜单切换(运行时切换是
mpv 的活,GUI 只管配置)。

## 语法与默认值

```conf
[名字]
选项=值
profile-restore=copy-equal
```

- 来源:mpv 手册 PROFILES 章节;`profile-restore` 见 OPTIONS 章节
- 本仓库:base.conf 有大量 profile 块示例;gen 输出追加方案块时
  必带 `profile-restore=copy-equal`(方案退出时恢复被改动的选项)

## 示例

```conf
[电影]
video-sync=display-resample
interpolation
deband

[默认]
profile=电影        # profile 可以引用其他 profile
```

## 风险提示

- profile 名唯一且不能与选项重名(`[volume]` 这类会与选项冲突);
- 启用/退出 profile 的选项回滚行为由 `profile-restore` 控制,
  `copy-equal` 是"退出时恢复原值"的正确选择;
- profile 里写错选项不报错,只是静默无效——用 `--profile=名字`
  启动验证,或看 `--msg-level=all=info` 日志。

## 相关选项

- `profile-restore`:profile 退出时的恢复策略
- `apply-profile`:播放中应用 profile 的命令(脚本/快捷键)
- `load-profile` / `profile-cond`:条件自动套用 profile
