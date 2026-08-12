# vo — 视频输出驱动(gpu-next)

## 用途

`vo` 决定视频画面的最终渲染后端。本仓库统一使用
`vo=gpu-next`:它是 mpv 新一代 GPU 渲染器,提供更好的色彩管线
(bt.1886 目标曲线、改进的 HDR 映射)、更高质量的缩放与更现代的
渲染路径,也是 `interpolation`、`inverse-tone-mapping` 等高级选项
完全生效的前提(部分选项仅在 gpu-next 下可用)。

## 语法与默认值

```conf
vo=<gpu|gpu-next|libmpv|...>
```

- 默认值:随版本演进,0.37+ 起 gpu-next 逐步成为默认;显式写出更稳妥
- 来源:mpv 手册 VIDEO OUTPUT DRIVERS 章节
- 本仓库 base.conf 第 11 行:`vo=gpu-next`(注释:许多后续选项只能在
  此项下正常工作)

## 示例

```conf
# 通用推荐
vo=gpu-next

# 排查渲染问题时回退旧渲染器对照
# vo=gpu
```

## 风险提示

- 老显卡驱动对 gpu-next 的部分特性(如 bt.1886 目标曲线)支持不佳,
  画面偏暗/偏色时先检查 `icc`/`target-trc` 相关设置(见教程
  《icc-profile — 色彩管理》),再考虑回退 `vo=gpu`。
- gpu-next 与 gpu 的默认参数不完全一致(如 tone-mapping 默认曲线
  auto 在 gpu-next 下实际为 spline),切换 vo 后观感差异属于正常。
- 一些第三方着色器只针对 gpu 编写,gpu-next 下可能无效或报错。

## 相关选项

- `gpu-api`:渲染 API 层(见教程《gpu-api — 图形接口》)
- `scale`/`cscale`/`dscale`:缩放算法(见教程《scale — 缩放算法》)
- `tone-mapping`/`target-peak`:HDR 映射(见教程《hdr — HDR 映射》)
