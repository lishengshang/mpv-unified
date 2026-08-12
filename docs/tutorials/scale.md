# scale — 缩放算法(scale/cscale/dscale)

## 用途

视频缩放是播放器最频繁的渲染运算:片源分辨率(720p/1080p/4K)与
窗口/全屏尺寸不一致时,需要插值放大(upscale)、缩小(downscale),
色度平面也要从 4:2:0 升频到 4:4:4(cscale)。算法选择直接影响
锐度、振铃(ringing)与性能。

三条链:
- `scale`:亮度/完整分辨率放大;
- `cscale`:色度升频(默认跟随 scale,建议单独设 bilinear 省性能);
- `dscale`:缩小(全屏看小分辨率视频不涉及,窗口缩小时才用到)。

## 语法与默认值

```conf
scale=<算法名>
cscale=<算法名>
dscale=<算法名>
scale-radius=<float>
scale-antiring=<0.0-1.0>
```

- 默认值:scale=lanczos, cscale=跟随 scale, dscale=hermite
- 来源:mpv 手册 OPTIONS 章节 scale 条目
- 本仓库 base.conf 注释推荐:放大用 ewa_lanczos / ewa_lanczossharp /
  ewa_hanning(jinc 系);"spline36 更适合实拍类,ewa 类更适合 anime 类";
  缩小推荐 catmull_rom 或 ewa_hanning

## 示例

```conf
# 动画片:jinc 系抗振铃好
scale=ewa_hanning
scale-radius=3.2383154841662362
scale-antiring=1.0

# 实拍片
scale=spline36

# 色度与缩小保持轻量
cscale=bilinear
dscale=catmull_rom
```

## 风险提示

- 高档算法(ewa 系)在 4K→8K 放大时 GPU 开销明显,核显注意帧率。
- 使用外部 upscale 类着色器(如 Anime4K)时基本无需再调内置 scale
  antiring,base.conf 注释明确提示了这一点。
- 缩放算法是"观感问题":没有绝对最优,换算法后用暂停帧对比
  (快捷键 `s` 截图或 `,`/`.` 逐帧)再决定。

## 相关选项

- `scale-radius`/`scale-antiring`/`cscale-radius`/`dscale-radius`:各链
  的半径与抗振铃
- `gpu-api`:Vulkan 下部分算法(gather 优化)表现更好(见教程
  《gpu-api — 图形接口》)
