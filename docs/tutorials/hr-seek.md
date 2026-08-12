# hr-seek — 精确跳转

## 用途

控制跳转(方向键/`[`/`]`/进度条拖动)是否精确到目标帧。默认
`hr-seek=default` 只在"看起来有利"时精确;`hr-seek=yes` 始终
精确——跳转后画面与进度条位置严格一致。

代价:精确跳转需要从最近的关键帧解码到目标位置,大跨度跳转
(如拖动进度条到 2 小时处)会有短暂解码等待;`hr-seek-framedrop`
控制这段解码是否丢帧(丢帧=更快,但可能短暂花屏/音画错位)。

本仓库 base.conf 使用:`hr-seek=yes` + `hr-seek-framedrop=no`,
并注释"SVP 补帧时推荐设置为 no"(丢帧会破坏补帧连续性)。

## 语法与默认值

```conf
hr-seek=<no|absolute|default|yes|always>
hr-seek-framedrop[=<yes|no>]
```

- 默认值:`hr-seek=default`,`hr-seek-framedrop=yes`
- 来源:mpv 手册 OPTIONS 章节 hr-seek / hr-seek-framedrop 条目
- 本仓库 base.conf 实际使用:`hr-seek=yes`、`hr-seek-framedrop=no`,
  注释:"跳转时丢帧,关闭利于修正音频延迟"

## 示例

```conf
# 始终精确跳转,跳转时不丢帧(观感优先)
hr-seek=yes
hr-seek-framedrop=no

# 精确 + 快(快速浏览素材)
hr-seek=yes
hr-seek-framedrop=yes
```

## 风险提示

- 大跨度精确跳转的解码开销在高码率 4K 上很明显,网络流尤甚
  (见教程《cache — 网络播放缓存》);素材粗剪场景用默认值更顺。
- `hr-seek-framedrop=no` 时,跳转后短暂音画不同步属正常(解码补帧
  期间),几秒内自愈;持续不同步则检查 `video-sync`(见教程
  《interpolation — 补帧》)。
- `hr-seek=always` 与 `no-hr-seek` 组合在个别版本有边界行为,
  生产配置用 `yes` 即可。

## 相关选项

- `hr-seek-framedrop`:跳转时是否丢帧
- `osd-on-seek` / `osd-fractions`:跳转反馈(见教程《osd-bar — OSD 进度条》)
- `video-sync`:音视频同步策略
