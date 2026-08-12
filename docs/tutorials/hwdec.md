# hwdec — 硬件视频解码

## 用途

让 GPU 专用硬件(如 Intel 核显、NVIDIA/AMD 显卡)参与视频解码,降低
CPU 占用。4K/8K 高码率视频、H.265/AV1 编码在软解下 CPU 可能吃满甚至
掉帧,硬解是这类场景的关键开关。

`auto-copy-safe` 是推荐的通用值:自动选择可用的硬件解码 API,并以
"复制回内存"的安全模式工作——解码结果拷回显存/内存供后续滤镜处理,
兼容性好,基本不会花屏。禁用写作 `no`(纯软解)。

## 语法与默认值

```conf
hwdec=<auto|auto-safe|auto-copy|auto-copy-safe|no|d3d11va|...>
```

- 默认值:`no`(mpv 默认软解)
- 来源:mpv 手册 OPTIONS 章节 hwdec 条目
- 本仓库 base.conf 使用:`hwdec=auto-copy-safe`(Windows 与 Linux 两个
  平台层均为此值,但内部解析到的 API 不同)

## 示例

```conf
# 通用安全配置(推荐)
hwdec=auto-copy-safe

# 完全软解(排查花屏/兼容性问题时)
hwdec=no

# Windows 上显式指定 DXVA2/D3D11
hwdec=d3d11va
```

## 风险提示

- 某些驱动/显卡组合下硬解可能花屏、黑屏或颜色错误;此时先降级为
  `hwdec=no` 软解确认问题是否消失,再试 `auto-copy-safe` 或更换 API。
- `auto-copy` 系列比"零拷贝"模式(如 `d3d11va` 直接解码到显存)多一次
  拷贝,性能略低但兼容性显著更好;只有确认零拷贝稳定才建议换。
- 字幕/滤镜类脚本(如补帧、实时 shader)对硬解路径有额外要求,异常时
  优先排查解码 API 与渲染路径的组合(见教程《gpu-api — 图形接口》)。

## 相关选项

- `hwdec-codecs`:仅对指定编码启用硬解
- `vo`:硬解结果最终由视频输出驱动渲染(见教程《vo — 视频输出驱动》)
- `gpu-api`:图形接口与硬解路径互相影响
