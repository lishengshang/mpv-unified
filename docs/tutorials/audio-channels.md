# audio-channels — 声道布局

## 用途

指定音频输出声道布局。mpv 默认按源自动匹配,但常见问题:
- 双声道音箱播放 5.1/7.1 片源时,部分声道消失或人声丢失;
- 声卡/HDMI 设备的声道能力被错误探测(如电视只报 2.0)。

`audio-channels` 强制指定布局,mpv 会做对应的 downmix/upmix 后输出。
本仓库 base.conf 用的是 `7.1,5.1,stereo` 备选链:按顺序尝试,设备
支持哪个就用哪个。

## 语法与默认值

```conf
audio-channels=<auto-safe|auto|布局名|布局列表>
```

- 默认值:`auto-safe`(自动选择安全布局,避免错误 upmix)
- 来源:mpv 手册 OPTIONS 章节 audio-channels 条目
- 本仓库 base.conf 实际使用:`audio-channels=7.1,5.1,stereo`,注释
  提示:"如果双声道系统播放多声道影片时有的声道声音没出现,尝试
  强制设定为双声道"

## 示例

```conf
# 双声道音箱强制立体声(声道丢失时用)
audio-channels=stereo

# 5.1 家庭影院
audio-channels=5.1

# 按设备能力依次尝试(推荐)
audio-channels=7.1,5.1,stereo
```

## 风险提示

- 布局名写错/设备不支持时 mpv 报"unsupported channel layout"并
  回退,不会崩溃;但每次启动都会刷错误日志,可据此排查。
- 强制 5.1/7.1 输出到双声道设备会触发 downmix;不同 downmix 算法
  对中置人声的保留程度不同,可配合 `audio-normalize-downmix`
  (见教程《volume — 音量》)调整。
- `auto-safe` 与 `auto` 的区别:`auto` 可能选择设备"声称支持但实际
  有问题"的布局,`auto-safe` 更保守,是官方推荐默认。

## 相关选项

- `audio-device`:输出设备选择(见教程《audio-device — 音频设备》)
- `audio-normalize-downmix`:downmix 时归一化音量
- `audio-samplerate`:重采样率
