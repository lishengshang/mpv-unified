# watch-later — 记忆播放进度

## 用途

让 mpv 记住每个文件的播放位置,下次打开同一文件时询问/自动续播。
连续剧观看、长视频断点续播的刚需功能。本仓库 base.conf 不仅开启
记忆,还把记录目录移到仓库内缓存(`~~/cache/watch_later`),并限制
只记住必要的轨道选择,避免旧进度干扰新文件。

## 语法与默认值

```conf
save-position-on-quit[=<yes|no>]
watch-later-dir=<路径>
watch-later-options=<逗号分隔的选项列表>
resume-playback-check-mtime[=<yes|no>]
write-filename-in-watch-later-config
```

- 默认值:`save-position-on-quit=no`(不记忆)
- 来源:mpv 手册 OPTIONS 章节 watch-later 相关条目
- 本仓库 base.conf 实际值:
  `save-position-on-quit=yes`、`watch-later-dir="~~/cache/watch_later"`、
  `watch-later-options=start,vid,aid,sid`、
  `resume-playback-check-mtime=yes`

## 示例

```conf
# 基础记忆:进度 + 音视频轨 + 字幕轨
save-position-on-quit=yes
watch-later-options=start,vid,aid,sid

# 更进一步:把音量/滤镜也记下来
watch-later-options=start,vid,aid,sid,volume,vf,af
```

## 风险提示

- 同名文件内容被替换时,旧进度会错误恢复——`resume-playback-check-mtime`
  通过比对修改时间防错(base.conf 已开启),但改名/重新下载的文件仍
  可能串进度。
- `watch-later-options` 记入的选项越多,恢复时"意外状态"(如忘关的
  af 滤镜)越容易带回来;只记 start 与轨道最安全。
- 记忆数据目录别放系统临时目录:重启清理后进度全丢。仓库布局把它
  放在 `~~/cache/` 即为此考虑。

## 相关选项

- `save-position-on-quit`:总开关
- `write-filename-in-watch-later-config`:记录文件名,便于排查
- `save-watch-history`:历史播放记录(供 select 脚本浏览)
