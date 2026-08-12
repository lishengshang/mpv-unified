# dither — 抖动与色深(dither-depth)

## 用途

色彩转换(如 HDR→SDR、广色域→sRGB、10bit→8bit)会损失低位精度,
产生可见色阶。抖动(dithering)在输出前给像素加可控噪声,用"随机感"
换掉"断层感",是低成本消灭色阶的成熟手段。

`dither-depth` 控制抖动的目标色深:8 位显示器写 8,10 位写 10;
`no` 完全关闭;`auto` 由 mpv 探测(不一定正确,base.conf 有明确注释)。
`dither` 选择抖动算法:`fruit`(默认,质量/性能平衡)、
`error-diffusion`(质量最高、最慢)、`ordered`(最省性能)。

## 语法与默认值

```conf
dither-depth=<N|no|auto>
dither=<fruit|ordered|error-diffusion|no>
temporal-dither
```

- 默认值:`dither-depth=auto`,`dither=fruit`
- 来源:mpv 手册 OPTIONS 章节 dither-depth / dither 条目
- 本仓库 base.conf:默认注释关闭;注释明确"auto 的值取决于 --gpu-api,
  不一定正确",建议显示器几 bit 就写几

## 示例

```conf
# 8 位显示器(HDR→SDR 场景必开)
dither-depth=8

# 10 位面板 + 高质量抖动
dither-depth=10
dither=error-diffusion

# 静帧截图党:关闭抖动获得纯净输出
dither-depth=no
```

## 风险提示

- `error-diffusion` 在 4K 高刷下开销可观,低端核显慎用;
  `temporal-dither`(帧间抖动)还会引入轻微闪烁感。
- 把 `dither-depth` 写成高于面板实际色深的数值没有意义,只是浪费
  运算;低于面板色深(如 10bit 面板写 8)反而会人为引入色阶。
- 与《deband — 去色带》配合:去色带消除源带,抖动消除转换带。

## 相关选项

- `dither-size-fruit`:fruit 抖动矩阵大小(默认 6,即 64×64)
- `error-diffusion`:error-diffusion 的内核算法(sierra-lite 等)
- `temporal-dither-period`:时域抖动更新频率
