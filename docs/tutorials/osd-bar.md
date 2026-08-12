# osd-bar — OSD 进度条与跳转提示

## 用途

mpv 的进度条(osd-bar)画在 OSD 层,显示播放进度/缓冲状态;
`osd-on-seek` 决定跳转时显示进度条还是文本时间,`osd-duration`
控制 OSD 信息停留时长。本仓库 base.conf 把默认的大进度条改成
细条(`osd-bar-h=2`)+ 贴底(`osd-bar-align-y=-1`)+ 禁用系统自带
条(`no-osd-bar` 关闭按键时的"整条覆盖式"进度条,由 `osd-on-seek=msg-bar` 接管)。

## 语法与默认值

```conf
no-osd-bar
osd-on-seek=<no|bar|msg|msg-bar>
osd-bar-w=<0-100>
osd-bar-h=<0-100>
osd-bar-align-y=<-1..1>
osd-duration=<毫秒>
```

- 默认值:`osd-on-seek=bar`,`osd-duration=1000`
- 来源:mpv 手册 OPTIONS 章节 osd-bar-* / osd-on-seek / osd-duration 条目
- 本仓库 base.conf 实际使用:`no-osd-bar`、`osd-on-seek=msg-bar`、
  `osd-bar-w=100`、`osd-bar-h=2`、`osd-bar-align-y=-1`、
  `osd-duration=2000`、`osd-fractions=yes`

## 示例

```conf
# 仓库风格:细条贴底 + 跳转时条+时间
no-osd-bar
osd-on-seek=msg-bar
osd-bar-w=100
osd-bar-h=2
osd-bar-align-y=-1

# 恢复默认:整条进度条
# 删除 no-osd-bar 与 osd-on-seek 行即可
```

## 风险提示

- `no-osd-bar` 只关"按键触发的整条进度条",播放时底部的细进度条
  仍由 `osd-bar-w/h` 控制;想完全隐藏改用 `osd-bar=no`(旧选项)。
- `osd-duration` 太短(如 500)会看不清跳转确认,太长(如 5000)
  则 OSD 频繁遮挡画面。
- `osd-fractions=yes` 让时间显示到毫秒,便于逐帧核对进度。

## 相关选项

- `osd-font`:OSD 文本样式(见教程《osd-font — OSD 字体与样式》)
- `osd-status-msg`:自定义状态栏模板(时间/帧率等信息)
- `osd-on-seek`:跳转提示类型
