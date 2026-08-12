# deband — 去色带

## 用途

低码率视频(尤其动画片与暗场景)常见明显的**色带**(banding):渐变
区域出现一圈圈生硬色阶。`deband` 是 mpv 内置的去色带滤镜,通过轻微
模糊与抖动破坏色带边缘,代价极低,观感提升明显,属于"无脑开"的
一类选项。

## 语法与默认值

```conf
deband[=<yes|no>]
deband-iterations=<1-16>
deband-threshold=<0-4096>
deband-range=<1-64>
deband-grain=<0-4096>
```

- 默认值:`deband=no`(关闭);iterations=1, threshold=64, range=16, grain=48
- 来源:mpv 手册 OPTIONS 章节 deband 条目
- 本仓库 base.conf 第 3 行注释:"部分启用的参数没有 `=`,则该项实际为
  `yes`。例如 deband 默认关闭:--deband 实际等效 --deband=yes"

## 示例

```conf
# 最简单的开启方式
deband

# 动画片去色带增强(iterations 提高)
deband
deband-iterations=2
deband-grain=64
```

## 风险提示

- 去色带本质是"有损处理":`deband-iterations` 过高会让画面变"肉"、
  丢失细小纹理;实拍片不建议超过 1。
- `deband-threshold` 是色带判定阈值,过高会把正常渐变也当成色带,
  导致画面变平。
- 与 upscale 类着色器同时使用时,建议把 deband 放在着色器之后观察,
  顺序不当可能互相干扰(见教程《scale — 缩放算法》)。

## 相关选项

- `deband-iterations`/`deband-threshold`/`deband-range`/`deband-grain`:
  去色带的四个核心参数
- `dither`:输出抖动,与去色带互补(见教程《dither — 抖动与色深》)
