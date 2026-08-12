# 平台差异分析:Windows 原版 vs Linux 移植版

> **状态:待用户确认清单**。本文逐条列出 `archive/mpv.conf.orig`(Windows 原版,1042 行)
> 与现役 `~/.config/mpv/mpv.conf`(Linux 移植版,346 行)之间的差异,并按
> **平台差异 / 个人偏好 / 无关改动** 三分类。拆分到 `config/` 四层结构时,
> 分类决定了差异的落位(平台层 / 条件指令 / base)。请逐条确认分类是否合理。

生成日期:2026-08-12

---

## 1. 平台差异(拆分时进入 windows.conf / linux.conf 或条件指令)

| # | 选项 | Windows 原版 | Linux 移植版 | 处理 |
|---|------|-------------|--------------|------|
| 1 | `gpu-api` | `d3d11`(原生渲染,推荐,SVP 需 d3d11) | `vulkan`(Linux 推荐,支持 gather 着色器) | base.conf 条件指令 `#@if platform==windows` |
| 2 | `hwdec` | `auto-copy-safe`,注释强调 d3d11va/d3d12va-copy | `auto-copy-safe`,注释强调 nvdec-copy/vaapi-copy | base.conf 条件指令 |
| 3 | `input-ipc-server` | `\\.\pipe\mpvsocket`(Windows 命名管道) | `/tmp/mpvsocket` | 平台层 |
| 4 | `audio-file-paths` | 分隔符 `;` | 分隔符 `:` | 平台层 |
| 5 | `sub-file-paths` | 分隔符 `;` | 分隔符 `:` | 平台层 |
| 6 | `ao` | `ao=wasapi`(启用) | `#ao=alsa`(注释,用系统默认) | 平台层 |
| 7 | `window-corners` | `window-corners=roundsmall`(启用) | `corner-rounding=0.5`(启用,通用选项) | 平台层 |
| 8 | `snap-window` | `snap-window=yes`(仅限 Windows) | 无 | windows.conf |
| 9 | `media-controls` / `backdrop-type` / `hidpi-window-scale` | Windows 特有选项(注释示例) | 无 | windows.conf |
| 10 | `d3d11-*` / `gpu-context` / `vd-lavc-dr` / `d3d11-adapter` / `vulkan-device` 等 | Windows 特有或 d3d11 专属(注释示例) | 无 | windows.conf |
| 11 | `d3d11-output-csp=pq` | Windows 直出 HDR(注释) | 无 | windows.conf |
| 12 | `image-subs-hdr-peak=sdr` | 启用 | 无 | windows.conf |
| 13 | `#fs` / `#no-border` / `#ontop` | Windows 窗口形态注释示例 | Linux 版另有 `#ontop=yes` 注释 | 平台层 |
| 14 | `#geometry=75%x70%` / `#autofit=70%` / `#autofit-larger=90%x90%` + `autofit-smaller=40%x30%` | Windows 窗口尺寸方案(含启用的 autofit-smaller) | Linux 启用 `geometry=60%x60%` / `keepaspect-window=yes` / `autofit-larger=90%x90%` / `force-window=immediate` | 平台层 |
| 15 | `#keepaspect` / `#keepaspect-window` / `#panscan` / `#loop` 等 | Windows 注释示例 | 无 | windows.conf |
| 16 | `load-context-menu` | 两侧一致(注释文本提及 Windows 平台默认 no) | 同 | base |
| 17 | `include="~~/profiles.conf"` / `input-conf="~~/input.conf"` / `use-filedir-conf` | 注释(profile 内联在 mpv.conf) | 启用(profile 独立文件) | 平台层;41 个 profile 块整体进 windows.conf(Linux 由 include 加载) |
| 18 | RTX-HDR / RTX-VSR 滤镜注释(`#vf-toggle=@rtx-hdr:...`) | `[当 --gpu-api=d3d11 时生效]` | `[Windows 专属,需 --gpu-api=d3d11] Linux 下不可用` | 平台层(注释文本不同) |
| 19 | 着色器路径 | `shaders/nnedi3/nnedi3-*.glsl`、`shaders/ravu/ravu-*.glsl` | `shaders/nnedi3/gather/*`、`shaders/ravu/gather/*`(gather 版,Vulkan 优化) | 条件指令 / 平台层 |
| 20 | `#d3d11-flip` / `#d3d11va-zero-copy` / `#d3d11-exclusive-fs` | Windows d3d11 专属(注释) | 无 | windows.conf |
| 21 | `[TV]`(display-info.dll) | 仅限 Windows | 无 | windows.conf |
| 22 | `#sub-font-provider`(fontconfig/DirectWrite 说明) | 注释示例,含 Windows/Linux 差异说明 | 无(内容已在注释中说明两平台) | base |

## 2. 个人偏好(拆分进 base 或平台层默认值,可在 user 层覆盖)

| # | 选项 | Windows 原版 | Linux 移植版 |
|---|------|-------------|--------------|
| 1 | `osd-color` | `"#FFFFFF"`(白) | `"#ef14d5"`(品红) |
| 2 | `osd-playlist-entry` | `filename` | `both` |
| 3 | `sub-font-size` | `50` | `44` |
| 4 | `screenshot-format` | `webp`(配 `#screenshot-webp-lossless=yes`) | `png` |
| 5 | `screenshot-webp-quality` | `85` | `100` |
| 6 | `osd-playing-msg` | 注释(未启用) | 启用(同一条 `osd-playing-msg` 模板) |
| 7 | `icc-cache-dir` | 注释(示例) | 启用 `"~~/cache/icc_cache"` |
| 8 | 缩放算法族 `scale/dscale/cscale/tscale/dither-depth/correct-downscaling/linear-downscaling` | 全部注释示例(由 `profile=HQ` 等配置组启用) | 顶层启用(等效 HQ 配置组值内联) |
| 9 | `profile=Target/Dither/Tscale/HQ/DeBand-low/HDR2SDR/NNEDI3` 启用行 | 启用 7 个常规配置组 | 不启用任何配置组(值已内联) |
| 10 | `j` / `k` / `v` 等字幕快捷键 | 启用 | 注释(Linux 用 `d` 绑定字幕可见性,`j/k` 保留给其它用途) |

## 3. 无关改动(内容一致或仅为顺序/空白差异,不影响语义)

- 两份文件均有 `vo=gpu-next`、`title=`、`osc=no`、`no-title-bar`、`input-ime=no`、
  `idle=yes`、`log-file`、`hr-seek`、`save-position-on-quit`、`watch-later-*`、
  `directory-mode`、`metadata-codepage`、`msg-level`、OSD 大部分、`audio-device`、
  `audio-channels`、`gapless-audio`、`audio-file-auto`、`alang`、`sub-codepage`、
  `sub-auto`、`slang`、字幕字体/颜色族、`screenshot-template`、`screenshot-tag-colorspace`、
  `gpu-shader-cache-dir`、`load-context-menu`、`ytdl-raw-options-append`、
  `input-default-bindings` 等 —— 值一致,归入 base。
- `screenshot-webp-compression=6`、`screenshot-webp-lossless` 注释等两侧一致。
- 条件配置组(41 个 profile 块:`[ICC]` 到 `[media-title]`)内容在 Windows 内联,
  Linux 由 `profiles.conf`(独立文件)承载 —— 差异来自承载方式,非选项语义。
- input.conf:绝大多数按键绑定两侧一致(277 行公共);差异集中在
  视频滤镜预设(Windows vs 的脚本文件与按键不同)、着色器路径(gather 版)、
  `Ctrl+Shift+v` 实现方式、`d`/`j`/`k`/`v` 等按键映射、鼠标右键行为、
  工具菜单(danmaku/trakt/mpv_cropscreen 等脚本存在性差异)。

---

## 拆分决策摘要

- **base.conf**:Windows 原版顶层内容(注释 100% 保留)+ `#@if` 条件指令(gpu-api、hwdec)。
- **windows.conf**:Windows 平台差异块 + 41 个 profile 块 + `profile=` 启用行。
- **linux.conf**:Linux 平台差异行(geometry 族、vulkan 相关、Linux 路径、include/input-conf 启用)。
- **input.conf**:公共按键 + 视频滤镜/着色器两节条件指令;**windows.input.conf** / **linux.input.conf**:差异行。
- 等价验证:`tools/verify-equivalence.sh linux|windows` 零差异通过(见 Evidence)。
