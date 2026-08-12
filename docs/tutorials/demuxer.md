# demuxer — 分离器

## 用途

demuxer 负责把容器(MP4/MKV/TS/FLV...)里的音视频轨、字幕轨、
章节、元数据**分离**出来交给解码器。mpv 默认按容器自动选择
demuxer,绝大多数情况零配置;需要干预的场景:
- 损坏/不规范的容器导致无法识别(手动指定 demuxer 类型);
- 网络流/直播需要调整缓冲(见教程《cache — 网络播放缓存》);
- 字幕流/音频流抽取异常。

## 语法与默认值

```conf
demuxer=<auto|lavf|mkv|mpegts|...>
demuxer-lavf-o=<key=value>[:...]   # 传给 FFmpeg 的 demuxer 选项
demuxer-readahead-secs=<秒>
```

- 默认值:`demuxer=auto`(按扩展名/内容探测)
- 来源:mpv 手册 OPTIONS 章节 demuxer 条目
- 本仓库 base.conf 在标题模板中使用 `${?demuxer-via-network==yes:...}`
  变量——按"是否网络流"区分标题显示,说明 demuxer 探测状态是
  可感知的系统信息

## 示例

```conf
# 强制用 FFmpeg lavf demuxer(应对自定义扩展名)
demuxer=lavf

# 给 lavf 传选项(如忽略损坏索引)
demuxer-lavf-o=ignore_editlist=1
```

## 风险提示

- 强制指定 demuxer 后,自动探测的兜底逻辑失效;格式一变就出错,
  非排查场景不要写死。
- `demuxer-lavf-o` 的键值对大小写敏感,以 FFmpeg 源码/文档为准;
  写错静默无效,验证需看 `--msg-level=all=info` 日志。
- 播放损坏文件时的"花式报错"(音画不同步、轨道缺失)优先排查
  容器本身,而不是改 demuxer。

## 相关选项

- `demuxer-max-bytes` / `demuxer-readahead-secs`:缓冲控制(见教程
  《cache — 网络播放缓存》)
- `demuxer-via-network`:网络流状态变量(模板/脚本可引用)
- `force-media-title`:覆盖探测出的媒体标题
