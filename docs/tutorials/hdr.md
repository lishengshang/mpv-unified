# hdr — HDR 映射(target-peak / tone-mapping)

## 用途

HDR 片源(HLG/PQ)在 SDR 显示器上播放时,亮度范围(最高可达 1000+
nits)远超 SDR 显示器(通常 200-400 nits),必须做**色调映射**
(tone-mapping)把 HDR 压进 SDR 范围。映射质量直接决定画面是否
过暗、高光是否死白、整体观感是否自然。

核心参数:
- `target-peak`:你的显示器峰值亮度(单位 nits),映射计算的基准;
- `tone-mapping`:映射曲线算法;
- `tone-mapping-param` / `tone-mapping-max-boost`:算法微调。

## 语法与默认值

```conf
target-peak=<auto|nits>
tone-mapping=<auto|clip|mobius|reinhard|hable|bt.2390|gamma|spline|...>
```

- 默认值:`target-peak=auto`(SDR 按 203 nits 估算),`tone-mapping=auto`
  (gpu-next 下实际为 spline)
- 来源:mpv 手册 OPTIONS 章节 target-peak / tone-mapping 条目
- 本仓库 base.conf:全套 HDR 参数以注释形式保留在 base.conf,注释建议
  "hdr 下建议根据实际显示效果指定具体 nits 值"

## 示例

```conf
# 普通 8bit SDR 显示器(约 250-300 nits)
target-peak=300
tone-mapping=hable

# 接近 HDR 的显示器(亮度较高),hable 微调
tone-mapping=hable
tone-mapping-param=0.3

# 保留高光细节优先
tone-mapping=bt.2390
```

## 风险提示

- `target-peak` 写高(如 1000)但显示器只有 300 nits,画面会整体偏暗;
  写低则高光压缩过度、细节丢失。按显示器实际标称值填写最稳妥。
- 启用 ICC 色彩管理时,`target-peak` 与 icc 参数会互相影响(mpv 对
  nits 高于 203 的配置会按 HDR 屏处理),见教程《icc-profile — 色彩管理》。
- 暗场景偏暗问题(issue #8009 一类)是 HDR 映射的经典老大难,
  `tone-mapping-max-boost` 可适度提升暗部,过高会发灰。

## 相关选项

- `tone-mapping-max-boost`:暗部细节提升(1.0-10.0)
- `target-contrast` / `target-trc`:对比度与传输曲线
- `inverse-tone-mapping`:SDR→HDR 反向映射(gpu-next,仅 HDR 设备)
