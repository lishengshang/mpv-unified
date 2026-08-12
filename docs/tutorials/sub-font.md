# sub-font — 字幕字体(与 sub-bold)

## 用途

指定纯文本字幕使用的字体。中文用户最常见的诉求是:
1. 让字幕使用好看的中文字体(如思源黑体/Noto Sans CJK);
2. 让字幕更"粗"更易读(`sub-bold=yes`)。

注意 mpv 有个已知行为(issue #8637):`sub-font` 虽然名义上只对纯文本
字幕生效,但实际也会影响 ASS 字幕的**缺省默认字体**——即 ASS 未显式
指定字体时的回退值。本仓库 base.conf 正是利用这一点统一了字体观感。

## 语法与默认值

```conf
sub-font=<字体名>
sub-bold=<yes|no>
```

- 默认值:`sub-font` 为系统默认字体(通常无衬线),`sub-bold=no`
- 来源:mpv 手册 OPTIONS 章节 sub-font / sub-bold 条目
- 本仓库 base.conf 使用:`sub-font="Noto Sans CJK SC"`、`sub-bold=yes`

## 示例

```conf
# 思源黑体(Windows 安装"思源黑体"后可直接写中文名)
sub-font="Noto Sans CJK SC"
sub-bold=yes

# 圆体风格
sub-font="Microsoft YaHei UI"
```

字体名必须与系统已安装字体完全一致(大小写敏感),可用
`fc-list :lang=zh`(Linux)查看可用字体。

## 风险提示

- 字体名写错时 mpv **不会报错**,只是静默回退默认字体——表现为"改了没
  效果",排查时先确认字体名与系统安装名完全一致。
- `sub-bold=yes` 对已带粗体样式的 ASS 字幕影响有限;纯文本字幕则
  明显变粗,过粗会损失笔画清晰度。
- 改字体不影响 OSD 文本字体,那由 `osd-font` 控制(见教程《osd-font —
  OSD 字体与样式》)。

## 相关选项

- `sub-font-size`:字幕字号
- `osd-font`:OSD 文本字体(音量条、进度条等)
- `sub-ass-override`:ASS 字幕样式覆盖强度(见教程《sub-ass — ASS 字幕渲染》)
