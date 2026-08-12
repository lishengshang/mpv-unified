# screenshot-format — 截图格式与质量

## 用途

mpv 内置截图(`s` 键)的格式与压缩质量设置。选对格式决定截图
是否保留透明/无损画质、文件大小与截图速度。日常分享用
`jpg`/`webp` 足够;做压片对比、逐帧分析、需要精确色彩时必须
无损格式(`png`/`webp-lossless`)。

## 语法与默认值

```conf
screenshot-format=<png|jpg|jpeg|webp|...>
screenshot-webp-compression=<0-6>
screenshot-tag-colorspace[=<yes|no>]
screenshot-tag-chapters[=<yes|no>]
```

- 默认值:`screenshot-format=png`;webp-compression=4
- 来源:mpv 手册 OPTIONS 章节 screenshot-* 条目
- 本仓库 base.conf 实际使用:`screenshot-webp-compression=6`(质量优先)、
  `screenshot-tag-colorspace=no`(避免部分看图软件误读色彩标签)

## 示例

```conf
# 默认:无损 PNG(体积大但画质无损)
screenshot-format=png

# 日常分享:高质量 WEBP,体积小
screenshot-format=webp
screenshot-webp-compression=6

# 极速截图:JPG
screenshot-format=jpg
```

## 风险提示

- 4K 高帧率连续截图(如逐帧)时 PNG 编码会拖慢;需要连拍用 jpg/webp。
- `screenshot-tag-colorspace=no` 后,部分自动色管软件(如浏览器
  直接预览)可能显示色偏,这是"无标签"的预期行为;专业流程反而
  需要保留标签(`yes`)。
- 截图的**保存位置与文件名**由 screenshot-template 控制,见教程
  《screenshot-template — 截图命名与目录》。

## 相关选项

- `screenshot-template`:保存路径与命名模板
- `screenshot-directory`:截图目录(更简单的写法)
- `screenshot-tag-chapters`:在截图元数据里记录章节号
