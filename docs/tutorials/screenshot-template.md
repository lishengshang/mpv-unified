# screenshot-template — 截图命名与目录

## 用途

控制截图(`s` 键)保存到哪里、叫什么名字。`screenshot-template`
支持 mpv 的展开变量(`${filename}`、`${media-title}`、`%P` 播放进度、
`%n` 递增序号等),是"截图自动归档"的关键:
- 单独目录(不污染工作目录);
- 按媒体标题命名(重名自动加序号);
- 按时间进度命名(便于回溯)。

`~~/` 前缀指 mpv 配置目录(即仓库布局),模板里的目录不存在时会
自动创建。

## 语法与默认值

```conf
screenshot-template=<模板>
screenshot-directory=<目录>
```

- 默认值:`screenshot-template=mpv-shot%nn`(存到当前目录)
- 来源:mpv 手册 OPTIONS 章节 screenshot-template / screenshot-directory 条目
- 本仓库 base.conf 实际使用:
  `screenshot-template="~~/files/screen/%{media-title}-%P-%n"`
  (存到仓库 files/screen/,按标题+进度+序号命名)

## 示例

```conf
# 简单:固定目录 + 文件名
screenshot-directory="~~/files/screen"
screenshot-template="%{filename}-%n"

# 带播放进度的命名(方便回溯)
screenshot-template="%{media-title}-%P-%n"
```

## 风险提示

- 模板变量拼写错误时,mpv 按字面量命名(不报错),目录会多出一堆
  奇怪名字的文件——首次配置后截一张验证。
- 文件名含 `/` 的媒体标题(如网络 URL)会被当作路径分隔,mpv 会
  自动净化,但目录层级可能超出预期;必要时只用 `%{filename}`。
- 截图写入失败(目录权限/磁盘满)时 mpv 仅报日志错误,不会弹窗,
  排查看播放日志。

## 相关选项

- `screenshot-format`:截图格式(见教程《screenshot-format — 截图格式与质量》)
- `screenshot-tag-chapters` / `screenshot-tag-colorspace`:元数据标签
- `save-watch-history`:历史记录目录(同用 `~~/files/` 体系)
