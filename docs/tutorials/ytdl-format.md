# ytdl-format — YouTube 在线视频格式选择

## 用途

mpv 内置 yt-dlp 支持,直接播放 YouTube/Bilibili 等网站的链接。
`ytdl-format` 把选择传给 yt-dlp 的 `-f` 参数,决定下载/播放哪种
音视频流组合:4K 高码率、1080p 优先、还是优先 AV1/VP9 编码。

常见需求:默认流质量过低(自动选 360p/480p)、想锁 1080p 以上、
或带宽有限想优先小体积编码。注意 mpv 播放多路流(HLS/DASH 分离的
视频+音频)会自动合并,无需额外配置。

## 语法与默认值

```conf
ytdl-format=<yt-dlp 格式选择表达式>
```

- 默认值:空(交给 yt-dlp 默认:`bestvideo+bestaudio/best`)
- 来源:mpv 手册 OPTIONS 章节 ytdl-format 条目(表达式语法见 yt-dlp
  官方 README 的 FORMAT SELECTION 章节)

## 示例

```conf
# 最高质量(默认行为,显式写出)
ytdl-format=bestvideo+bestaudio/best

# 最高 1080p(带宽/设备解码能力受限时)
ytdl-format="bestvideo[height<=1080]+bestaudio/best[height<=1080]"

# 优先 AV1(同码率下画质更好,但解码更吃 CPU)
ytdl-format="bestvideo[vcodec^=av01]+bestaudio/best"
```

## 风险提示

- 编码过滤器会误伤无对应流的视频:YouTube 部分视频无 4K VP9/AV1,
  表达式要保留 `best` 兜底(如上示例末尾的 `/best`),否则直接报错。
- 高分辨率流对硬解要求高;4K 在核显上先确认 `hwdec` 状态(见教程
  《hwdec — 硬件视频解码》)。
- 表达式用引号包裹,避免 `[]`/`+` 被 shell 或配置文件语法误解。

## 相关选项

- `ytdl-raw-options-append`:把任意 yt-dlp 参数透传(见教程
  《ytdl-raw-options — yt-dlp 参数透传》)
- `ytdl-format-force`:跳过 mpv 的格式解析,原样传给 yt-dlp
- `stream-record`:边播边录制
