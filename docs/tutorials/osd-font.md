# osd-font — OSD 字体与样式

## 用途

OSD(On-Screen Display)是 mpv 的画面叠加层:音量条、进度条、
跳转提示、快捷键反馈、菜单都画在 OSD 上。`osd-font` 控制 OSD
文本字体(与字幕字体 `sub-font` 相互独立),`osd-font-size`、
`osd-outline-*`、`osd-shadow-*`、`osd-back-color` 控制其样式。

中文字体选择直接影响 OSD 是否出现"豆腐块"(缺字形);本仓库
base.conf 使用 `Noto Sans Mono CJK SC`(等宽中文字体),进度条
对齐工整。

## 语法与默认值

```conf
osd-font=<字体名>
osd-font-size=<size>
osd-outline-size=<float>
osd-outline-color=<颜色>
osd-shadow-offset=<float>
osd-back-color=<颜色>
osd-border-style=<outline-and-shadow|opaque-box|background-box>
```

- 默认值:`osd-font-size=38`,`osd-border-style=outline-and-shadow`
- 来源:mpv 手册 OPTIONS 章节 osd-font / osd-font-size 条目
- 本仓库 base.conf 实际使用:`osd-font="Noto Sans Mono CJK SC"`、
  `osd-font-size=24`、`osd-outline-size=1.0`、
  `osd-outline-color="#1C1B1F"`、`osd-shadow-offset=0`、
  `osd-back-color="#1C1B1F"`

## 示例

```conf
# 仓库风格:小号等宽 + 深色描边
osd-font="Noto Sans Mono CJK SC"
osd-font-size=24
osd-outline-size=1.0
osd-outline-color="#1C1B1F"

# 默认观感(大字号)
osd-font-size=38
```

## 风险提示

- OSD 字体缺字形时显示 □ 占位:确认字体已安装且名称与系统一致
  (`fc-list` 检查,Linux)。
- `osd-shadow-offset=0` 时阴影消失,只剩描边;`osd-border-style`
  决定"描边还是底框"的取舍,底框模式更清晰但遮挡画面。
- OSD 字号与字幕字号独立:调字幕(见教程《sub-font-size — 字幕字号》)
  不会影响 OSD。

## 相关选项

- `osd-bar-w` / `osd-bar-h`:进度条尺寸(见教程《osd-bar — OSD 进度条》)
- `osd-duration`:OSD 文本显示时长
- `osd-on-seek`:跳转时的显示类型(bar/msg)
