# sub-ass — ASS 字幕渲染(sub-ass-override)

## 用途

ASS/SSA 字幕自带完整样式(字体、颜色、位置、特效),由 libass 渲染。
`sub-ass-override` 控制 mpv 对 ASS 样式的**覆盖强度**,决定"字幕
长什么样我说了算"还是"字幕作者说了算":

- `no`:完全尊重字幕自带样式(特效字幕、卡拉OK的正确姿势);
- `yes`/`force`:强制使用你的字体/字号/颜色(避开丑样式,但会
  破坏特效与卡拉OK对齐);
- `scale`:保留样式但随窗口缩放;
- `strip`:去掉样式但保留位置(注:仅 strip 定位相关属性)。

## 语法与默认值

```conf
sub-ass-override=<no|yes|scale|force|strip>
sub-ass-force-margins[=<yes|no>]
sub-ass-vsfilter-aspect-compat[=<yes|no>]
```

- 默认值:`sub-ass-override=yes`,`sub-ass-vsfilter-aspect-compat=yes`
- 来源:mpv 手册 OPTIONS 章节 sub-ass-override 条目
- 本仓库 base.conf 的 watch-later 记忆列表与 reset-on-next-file
  中均包含 sub-ass-* 系列,属于"播放中可调"状态

## 示例

```conf
# 特效字幕(OP/ED、日剧对话弹跳)完整还原
sub-ass-override=no

# 一律用你自己的字体,无视字幕样式
sub-ass-override=force
sub-font="Noto Sans CJK SC"

# 折中:样式缩放跟随窗口
sub-ass-override=scale
```

## 风险提示

- `force` 会把卡拉OK逐字特效的时序打乱(字提前/延后高亮),
  弹幕/歌词字幕尤其明显。
- `no` 模式下你的 `sub-font-size`/`sub-font` 对 ASS 字幕基本失效
  (仅影响缺省字体回退),"改了字号没效果"先查这个开关。
- 个别压片组用 "全屏特效" 超大字幕,`no` 模式下无法缩小——
  这是作者意图,只能换 `strip`/`yes` 妥协。

## 相关选项

- `sub-font` / `sub-font-size`:纯文本字幕样式(见教程《sub-font — 字幕字体》)
- `sub-ass-force-margins`:忽略安全边距,贴边渲染
- `sub-pos`:字幕垂直位置
