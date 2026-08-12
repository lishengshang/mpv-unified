# mute — 静音

## 用途

`mute` 是 mpv 的静音开关,与系统静音相互独立:mpv 静音后系统其他
声音不受影响。适合"只看不听"(如整理片库、边看边干别的)或
快速静音场景(键盘 `m` 键)。

## 语法与默认值

```conf
mute[=<yes|no>]
```

- 默认值:`no`(不静音)
- 来源:mpv 手册 OPTIONS 章节 mute 条目
- 本仓库 base.conf:标题栏模板用到了 `${?mute==yes:🔇}` 变量——
  静音时窗口标题会显示 🔇 图标,状态一目了然

## 示例

```conf
# 启动即静音
mute=yes

# 启动即静音,但音量保持 80(取消静音即回到 80)
mute=yes
volume=80
```

## 风险提示

- mpv 静音与"音量 0"不同:`mute=yes` 取消后回到原音量,`volume=0`
  则永久为 0;两者效果相同但恢复行为不同。
- 启用 watch-later 记忆后,mute 状态会被记入下一文件恢复(若
  `watch-later-options` 包含 mute),见教程《watch-later — 记忆播放进度》。
- 检查"没声音"问题时先区分:mpv 静音(m 键状态)、mpv 音量、
  系统/设备音量三个层级。

## 相关选项

- `volume`:音量(见教程《volume — 音量》)
- `audio-device`:输出设备(见教程《audio-device — 音频设备》)
- `title`:标题栏模板中的 `${?mute==yes:...}` 条件变量
