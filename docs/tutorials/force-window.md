# force-window — 强制窗口

## 用途

`force-window` 让 mpv **在没有媒体文件时也创建窗口**,把播放器变成
一个常驻 GUI 应用。对桌面端使用(本仓库形态)几乎是必选项:
没有它,`idle=yes` 的 mpv 在无媒体时窗口不出现,用户面对一个
"看不见的播放器"。

配合形态:`idle=yes` + `force-window` = 窗口常驻,随时打开文件;
`keep-open=yes` 保证播完不退出(见教程《keep-open — 播放结束后的行为》)。

## 语法与默认值

```conf
force-window[=<yes|no|immediate>]
```

- `no`:无媒体不创建窗口(默认);
- `yes`:空闲时也创建窗口(空窗口,显示背景色);
- `immediate`:启动立即创建,不等首个窗口就绪(减少启动闪烁)。

默认值:`no`。来源:mpv 手册 OPTIONS 章节 force-window 条目。

## 示例

```conf
# 常驻 GUI(推荐组合)
idle=yes
force-window=yes
keep-open=yes

# 启动即出窗口,无闪烁
force-window=immediate
```

## 风险提示

- `force-window=yes` 时空窗口显示纯色背景,若配合 `no-border`
  (见教程《borderless — 无边框窗口》)会是一个"悬浮色块",
  需要有可点击的界面(uosc 空状态)或脚本兜底。
- 多窗口脚本/二次嵌入(如 libmpv 宿主)场景不要开,窗口管理会
  冲突。
- 无媒体时的空窗口仍占用 GPU 渲染资源;纯 CLI 服务器环境保持
  `no`。

## 相关选项

- `idle`:空闲行为(见教程《idle — 空闲模式》)
- `keep-open`:播完行为(见教程《keep-open — 播放结束后的行为》)
- `window-background` / `background`:空窗口背景色
