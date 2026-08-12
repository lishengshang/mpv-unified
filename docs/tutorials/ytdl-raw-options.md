# ytdl-raw-options — yt-dlp 参数透传

## 用途

`ytdl-raw-options` 把任意 yt-dlp 命令行参数原样传给 yt-dlp,是解锁
"mpv 没暴露的 yt-dlp 功能"的万能通道:代理、cookies、限速、字幕
下载、自定义 headers 等都能通过它配置。`-append` 后缀表示"追加"
而不是覆盖,可安全地与 GUI/命令行里已有的参数叠加。

## 语法与默认值

```conf
ytdl-raw-options-append=<yt-dlp 参数=值>
```

- 默认值:空
- 来源:mpv 手册 OPTIONS 章节 ytdl-raw-options 条目
- 本仓库 base.conf 实际使用:
  `ytdl-raw-options-append=cookies-from-browser=Firefox`
  (把浏览器 cookies 带给 yt-dlp,绕过登录/年龄限制;支持 Firefox、
  Chrome、Chromium、Edge、Opera、Vivaldi 等)

## 示例

```conf
# 走代理(格式:yt-dlp 的 --proxy 参数)
ytdl-raw-options-append=proxy=http://127.0.0.1:7890

# 用浏览器 cookies 登录
ytdl-raw-options-append=cookies-from-browser=Firefox

# 限速 2MB/s 避免占满带宽
ytdl-raw-options-append=limit-rate=2M
```

## 风险提示

- **不要在配置文件里写真实密钥/令牌**:代理用户名密码、API token
  属于敏感信息,应放 `user/user.conf`(已被 gitignore),绝不放
  base.conf 等入库文件(仓库 CI 会扫描密钥)。
- 参数值含空格或特殊字符时用引号;多个 `-append` 行会按顺序叠加。
- 参数名以 yt-dlp 官方为准(去掉前导 `--`),拼写错误不会报错,
  只是静默无效——验证时看播放日志(`--msg-level=all=info`)。

## 相关选项

- `ytdl-format`:格式选择(见教程《ytdl-format — 在线视频格式选择》)
- `ytdl-format-force`:强制原样传递
- `stream-record` / `ytdl-raw-options=...`:更精细的组合用法
