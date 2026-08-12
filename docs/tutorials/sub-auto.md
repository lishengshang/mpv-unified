# sub-auto — 外挂字幕自动加载

## 用途

让 mpv 自动匹配同目录的外部字幕文件(.srt/.ass/.ssa 等),免去
手动拖入。`sub-auto` 控制匹配强度,`sub-file-paths` 指定额外搜索
目录,`slang` 决定多语言字幕的优先选择顺序。

本仓库 base.conf 的完整字幕加载策略:
- `sub-auto=fuzzy`:同目录模糊匹配(忽略大小写与后缀差异);
- `slang=chs,sc,zh-Hans,...,eng`:中文字幕优先,英文兜底;
- `sub-codepage=gb18030`:老字幕常见的 GB 编码自动转换。

## 语法与默认值

```conf
sub-auto=<no|exact|fuzzy|all>
sub-file-paths=<目录列表>
slang=<语言代码列表>
sub-codepage=<编码|auto>
```

- 默认值:`sub-auto=exact`(仅同名前缀),`sub-codepage=auto`
- 来源:mpv 手册 OPTIONS 章节 sub-auto / slang / sub-codepage 条目
- 本仓库 base.conf 实际使用:`sub-auto=fuzzy`、`slang=chs,sc,zh-Hans,
  cht,tc,zh-Hant,zh-CN,zh-TW,chi,zho,zh`、`sub-codepage=gb18030`

## 示例

```conf
# 宽松匹配 + 中文优先 + GB 编码兼容(推荐,即仓库默认)
sub-auto=fuzzy
slang=chs,sc,zh-Hans,cht,tc,zh-Hant,eng
sub-codepage=gb18030

# 只在指定的字幕目录里找
sub-file-paths="~~/subs"

# 只要精确同名(避免误挂其他版本的字幕)
sub-auto=exact
```

## 风险提示

- `fuzzy` 会匹配到 `xxx.eng.srt`、`xxx(1).srt` 等变体,多版本文件
  并存时可能挂错字幕;`slang` 优先级只在**加载时**生效。
- 同名不同集字幕(`01.ass` vs `01.zh.ass`)fuzzy 匹配结果不稳定,
  建议用 `mpv` 的字幕循环键 `J`/`j` 切换确认。
- `sub-codepage=gb18030` 对 UTF-8 文件无害(自动检测),但极老的字幕
  用非 GB 编码(如 Shift-JIS)时仍会乱码,需手动指定。

## 相关选项

- `audio-file-auto`:外挂音轨自动加载(同体系,见教程
  《audio-file-auto — 外挂音轨》)
- `sub-ass`:ASS 字幕渲染(见教程《sub-ass — ASS 字幕渲染》)
- `sub-visibility`:字幕显隐
