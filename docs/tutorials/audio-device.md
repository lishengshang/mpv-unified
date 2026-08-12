# audio-device — 音频输出设备

## 用途

指定 mpv 启动时使用的音频输出设备(声卡/HDMI/蓝牙等)。默认 `auto`
交给音频后端选;当系统有多个输出(耳机+音箱+HDMI)时,自动选择
经常不是你想要的设备,显式指定即可固定。

设备名与音频后端(ALSA/PulseAudio/WASAPI 等)相关,先用
`mpv --audio-device=help` 列出本机可用设备与准确名称。

## 语法与默认值

```conf
audio-device=<设备名>
audio-display-device=<设备名>   # 仅 Windows:跟随显示器的设备
```

- 默认值:`auto`
- 来源:mpv 手册 OPTIONS 章节 audio-device 条目
- 本仓库 base.conf 使用:`audio-device=auto`,注释:"此项用于指定
  启动时的音频输出设备"

## 示例

```conf
# 固定用某设备(名称来自 mpv --audio-device=help 输出)
audio-device=alsa/default

# Windows 上跟随显示器(HDMI 音频随全屏窗口切换)
audio-display-device=yes
```

## 风险提示

- 设备名随系统/驱动变化(蓝牙断开后设备名消失),固定名称可能让
  mpv 启动报"设备不存在"并回退;音频多设备用户建议保持 `auto`,
  或用脚本按连接状态切换。
- 修改后**重启播放器**才生效(启动时读取)。
- HDMI 音频建议同时检查声道布局(见教程《audio-channels — 声道布局》)
  与输出延迟(`audio-buffer`)。

## 相关选项

- `ao`:音频输出后端(alsa/pulse/wasapi/pipewire 等)
- `audio-channels`:声道布局
- `audio-buffer`:音频缓冲(降低爆音/卡顿)
