# borderless — 无边框窗口

## 用途

去掉窗口的系统边框/标题栏,画面沉浸感更强,是"无缝嵌合
桌面/壁纸级播放"的基础。mpv 提供两级:
- `no-border`:隐藏系统边框(窗口仍可拖动,标题栏消失);
- `no-title-bar`:Windows 专用,隐藏标题栏但保留原生窗口特性
  (任务栏行为、缩放动画),比 no-border 更精细。

本仓库 base.conf 默认关闭系统边框:
`no-title-bar`(Windows 平台,注释:可保留 Windows 原生窗口特性),
配合 `osc=no` 后完全交给 uosc 等第三方界面(见教程《script-opts —
脚本配置》)。

## 语法与默认值

```conf
no-border
no-title-bar
border=<yes|no>
```

- 默认值:`border=yes`(有边框)
- 来源:mpv 手册 OPTIONS 章节 border / no-title-bar 条目
- 本仓库 base.conf 使用:`no-title-bar`,注释说明它"可以保留 Windows
  原生窗口特性",与 `--no-border` 的行为差异

## 示例

```conf
# 全平台无边框
no-border

# Windows:无标题栏但保留原生特性
no-title-bar
```

## 风险提示

- `no-border` 后窗口无法用标题栏拖动,只能按住视频区拖动
  (mpv 默认 `alt+鼠标拖拽` 或 window-drag 相关绑定);Linux 下
  部分 WM 无边框窗口的圆角/阴影行为不同。
- 无边框 + 无 osc = 完全没有可见控件,新手会"找不到退出键"——
  确保输入绑定(`q` 退出等)已配置。
- Windows 上 `no-title-bar` 与全屏/多显示器 DPI 场景偶有兼容
  问题,异常时退回 `no-border` 对照。

## 相关选项

- `ontop`:置顶(见教程《ontop — 窗口置顶》)
- `osc` / uosc:无边框后的界面方案(见教程《script-opts — 脚本配置》)
- `window-minimized` / `window-maximized`:窗口状态
