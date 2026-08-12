# cache — 网络播放缓存

## 用途

播放网络流(HTTP/HLS 等)时,mpv 先在内存/磁盘缓存中囤积数据再
播放,保证网络抖动时画面不卡。核心参数:
- `demuxer-max-bytes`:缓存目标大小(网络播放时);
- `demuxer-readahead-secs`:缓存的最大时长(秒);
- `demuxer-max-back-bytes` / `cache-backbuffer`(gpu-next 缓存路径);
- `cache-dir` / `cache-on-disk`:磁盘缓存(配合本地缓存目录)。

本仓库 base.conf 把相关行以注释形式保留(`demuxer-max-bytes=500MiB`、
`demuxer-readahead-secs=20`),默认值对大多数场景够用,卡顿时再调。

## 语法与默认值

```conf
demuxer-max-bytes=<KiB|MiB>
demuxer-readahead-secs=<秒>
cache-dir=<目录>
```

- 默认值:由 mpv 按协议自动设置(内存缓存,通常数十 MB)
- 来源:mpv 手册 OPTIONS 章节 demuxer-max-bytes / cache 相关条目
- 本仓库 base.conf 注释:"播放网络视频时的目标缓存大小(KiB 或 MiB)"
  "限制网络视频的最大缓存时间(秒数)"

## 示例

```conf
# 高码率网络流:大缓存抗抖动
demuxer-max-bytes=500MiB
demuxer-readahead-secs=20

# 低带宽环境:小缓存快速开始播放
demuxer-max-bytes=32MiB
```

## 风险提示

- 缓存过大→启动播放等待变长(先囤够才开始);缓存过小→网络抖动
  即卡。500MiB/20s 是大多数流媒体场景的平衡点。
- 内存缓存对 4K 高码率流(单集数 GB)毫无意义——播到哪丢到哪,
  磁盘缓存(`cache-on-disk` + `cache-dir`)才支持整段预取。
- 直播流不能无限回看:缓存只保留"当前位置向后"的数据,
  `demuxer-max-back-bytes` 控制可回退的时长。

## 相关选项

- `demuxer`:协议/容器分离器(见教程《demuxer — 分离器》)
- `cache-on-disk` / `cache-dir`:磁盘缓存路径
- `ytdl-format`:选择更小码率的流,从源头省缓存(见教程
  《ytdl-format — 在线视频格式选择》)
