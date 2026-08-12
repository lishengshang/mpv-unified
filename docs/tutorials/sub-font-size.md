# sub-font-size — 字幕字号

## 用途

控制纯文本字幕(非 ASS/SSA)的默认字号。mpv 默认字幕会缩放到窗口高度
的约 2.2%,字号写 44 大约相当于窗口高度 4.4% 的字。习惯"字幕偏大"
的用户(如投影仪、大屏观看)最常见的就是调这个选项。

在 mpv 中 `sub-font-size` 与播放时的属性 `sub-scale`(相对倍率)是两套
调节体系:本选项是字号基线,`sub-scale` 是在其之上的倍率。两者都写时
最终字号 = `sub-font-size` × `sub-scale`。

## 语法与默认值

```conf
sub-font-size=<size>
```

- 默认值:55(单位:窗口高度的 1/1000,即 55 ≈ 5.5% 的窗口高度)
- 来源:mpv 手册 OPTIONS 章节的 sub-font-size 条目
- 本仓库 `user.example.conf` 中的示例值:`sub-font-size=44`(≈ 4.4% 窗口高度)

## 示例

```conf
# 常规 1080p 窗口观看
sub-font-size=44

# 投影仪 / 远距离观看,明显加大
sub-font-size=70
```

配合字体一起调整更协调,见教程《sub-font — 字幕字体》。

## 风险提示

- 这个选项只影响**纯文本字幕**。ASS/SSA 字幕自带字体与字号样式,
  默认由 `sub-ass-override` 的样式覆盖逻辑接管,修改 `sub-font-size`
  对带样式 ASS 字幕可能无效或效果有限。
- 字号过大时会遮挡画面主体,建议配合 `sub-pos`(字幕垂直位置)微调。
- 播放中按 `9` / `0` 调整的也是本选项(可持久化到 watch-later)。

## 相关选项

- `sub-scale`:相对倍率,两者相乘才是最终字号
- `sub-font`:字幕字体
- `sub-pos`:字幕垂直位置(0-100)
