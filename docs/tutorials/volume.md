# volume — 音量(volume-max / volume-step)

## 用途

mpv 的播放音量由 `volume`(0-130)控制,区别于系统音量。相关参数:
- `volume-max`:允许的最大音量。默认 130——某些片源录制电平低,
  mpv 音量推到 130 才有"足够响";但也会让误按 `9` 直接爆音;
- `volume-step`:每次按 `9`/`0` 调整的步长(默认 3);
- `audio-normalize-downmix`:多声道 downmix 到立体声时归一化音量,
  避免中置/环绕叠加导致爆音或人声变小。

## 语法与默认值

```conf
volume=<0-130>
volume-max=<100-130>
volume-step=<1-...>
audio-normalize-downmix[=<yes|no>]
```

- 默认值:`volume-max=130`,`volume-step=3`,`audio-normalize-downmix=no`
- 来源:mpv 手册 OPTIONS 章节 volume / volume-max / volume-step 条目
- 本仓库 base.conf 在 watch-later-options 的注释中列出 volume,
  说明它属于"可被记忆的播放中状态"

## 示例

```conf
# 限制最高音量,防爆音
volume-max=100

# 默认音量 80,步长 5
volume=80
volume-step=5

# 5.1/7.1 片源在立体声设备上人声偏小
audio-normalize-downmix=yes
```

## 风险提示

- `volume-max` 写小于当前音量的值会被钳制;调整后确认实际音量。
- 音量是播放中状态,watch-later 记忆开启时(见教程
  《watch-later — 记忆播放进度》)会按文件恢复,注意别把 130 的
  记忆带到下一个文件。
- 爆音排查顺序:volume > volume-max > downmix 叠加 > 系统/设备音量。

## 相关选项

- `mute`:静音开关(见教程《mute — 静音》)
- `audio-normalize-downmix`:downmix 归一化
- `softvol`(已废弃,现代版本直接用 volume)
