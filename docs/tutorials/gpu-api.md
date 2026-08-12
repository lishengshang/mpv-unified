# gpu-api — 图形接口

## 用途

选择 mpv 渲染使用的图形 API:OpenGL、Vulkan 或 Direct3D 11。它决定
了 shader 执行、硬件解码回传、色彩管理等底层路径走哪一套驱动栈,
是"画面相关疑难杂症"的第一排查位。

平台推荐:
- Linux:Vulkan 性能最佳,支持 gather 着色器优化,是首选;
- Windows:默认 d3d11,SVP 等外部补帧工具联动时也常要求 d3d11。

## 语法与默认值

```conf
gpu-api=<opengl|vulkan|d3d11>
```

- 默认值:`auto`(自动选择,Linux 通常落 Vulkan,Windows 落 d3d11)
- 来源:mpv 手册 OPTIONS 章节 gpu-api 条目
- 本仓库:base.conf 注释建议"SVP 补帧时推荐设置为 d3d11";平台层
  Windows 用 `gpu-api=d3d11`、Linux 用 `gpu-api=vulkan`

## 示例

```conf
# Linux 桌面(推荐)
gpu-api=vulkan

# Windows + SVP 补帧联动
gpu-api=d3d11

# 老显卡 / 驱动异常时回退
gpu-api=opengl
```

## 风险提示

- 显卡驱动不支持所选 API 时 mpv 启动会报错或白屏;升级驱动前先用
  `auto` 让 mpv 自己选。
- Vulkan 下部分旧版着色器/脚本不兼容;遇到 shader 异常可临时切
  opengl 对照。
- 改变 `gpu-api` 会影响 `dither-depth=auto` 的实际取值(mpv 的 auto
  探测依赖具体 API),见教程《dither — 抖动与色深》。

## 相关选项

- `gpu-context`:窗口系统与 API 的绑定上下文(wayland/x11/win32 等)
- `hwdec`:硬件解码路径与图形接口强相关
- `vo`:渲染后端必须与 gpu-api 配套(见教程《vo — 视频输出驱动》)
