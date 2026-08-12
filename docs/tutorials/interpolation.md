# interpolation — 补帧(画面防抖动)

## 用途

当视频帧率(如 23.976fps 电影、24fps 动画)与显示器刷新率
(60/120/144Hz)不匹配时,画面会出现周期性微卡顿(judder)。
`interpolation` 通过在帧之间插值,让画面平滑跟随显示器刷新率,
是 mpv 内置的"无运动补偿补帧"(非 MEMC,不生成新运动轨迹,
只做时域重采样)。

必须与 `video-sync=display-resample`(或 display-* 系列)配合才生效:
前者负责音频重采样补偿,后者负责时域插值。

## 语法与默认值

```conf
interpolation
video-sync=display-resample
tscale=<oversample|linear|...>
```

- 默认值:全部关闭;`tscale` 默认 `oversample`
- 来源:mpv 手册 OPTIONS 章节 interpolation / video-sync / tscale 条目
- 本仓库 base.conf:默认注释关闭;注释明确提示"SVP 补帧时推荐关闭"(
  外部 SVP 做运动补偿补帧时,mpv 内置插值会与之打架)

## 示例

```conf
# 60Hz 显示器看 24fps 电影,消除 judder
video-sync=display-resample
interpolation
tscale=oversample

# 追求更低延迟可换 linear,但平滑度下降
# tscale=linear
```

## 风险提示

- 插值会引入轻微模糊/振铃伪影(尤其快速平移画面),不是所有片源都
  值得开;动画片通常收益明显,实拍快速运动可能反而难受。
- 开启后 CPU/GPU 占用上升;核显设备在 4K 高帧率下可能扛不住。
- 与外部 MEMC 工具(SVP、RIFE 脚本)**不能同时开**,否则双重补帧
  画面严重破损。
- 电竞/低延迟场景(帧同步、zero-latency 需求)建议关闭。

## 相关选项

- `tscale`/`tscale-radius`/`tscale-antiring`:时域插值算法参数
- `video-sync-max-video-change`:限制音视频失配的最大调整幅度
- `interpolation-preserve`:gpu-next 下保留前一帧插值结果
