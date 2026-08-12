# gapless-audio — 无缝音频播放

## 用途

连续播放专辑/演唱会分段文件时,文件切换瞬间的静音与爆音很破坏
体验。`gapless-audio` 让 mpv 在文件边界保持音频连续输出:

- `weak`(默认):仅在音频格式不变时无缝衔接;
- `yes`:强制无缝(必要时临时重采样/重映射);
- `no`:完全关断,切换时重新初始化音频输出。

本仓库 base.conf 使用 `gapless-audio=no`,注释解释了 weak 的机制:
"默认值 weak:当音频格式发生变化时初始化音频输出"——对视频为主
的使用场景,切换时的音频输出重置反而更干净。

## 语法与默认值

```conf
gapless-audio=<no|yes|weak>
```

- 默认值:`weak`
- 来源:mpv 手册 OPTIONS 章节 gapless-audio 条目
- 本仓库 base.conf 实际使用:`gapless-audio=no`

## 示例

```conf
# 音乐场景:强制无缝
gapless-audio=yes

# 视频为主(仓库默认)
gapless-audio=no

# 保守:格式一致时无缝
gapless-audio=weak
```

## 风险提示

- `yes` 在采样率不同的文件间会触发重采样,音质敏感用户可能察觉
  差异;且少数设备(如 HDMI 直通)下可能引入短暂杂音。
- 无缝播放依赖 demuxer 预读下一文件(见教程《demuxer — 分离器》),
  大文件/网络流场景切换仍可能有间隙。
- 播放列表跳转(非顺序)不受影响——无缝只在顺序播放时生效。

## 相关选项

- `audio-samplerate`:重采样率(强制无缝时的采样目标)
- `audio-file-auto`:外挂音轨(见教程《audio-file-auto — 外挂音轨》)
- `playlist` 相关:播放列表行为
