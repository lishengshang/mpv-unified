# icc-profile — 色彩管理

## 用途

校色显示器携带 ICC/ICM 配置文件,包含显示器真实的色域与传输曲线。
`icc-profile` 让 mpv 按该文件做色彩转换,保证视频 RGB 输出到屏幕时
颜色准确;`icc-profile-auto` 则自动加载系统配置的校色文件。

本仓库自带 `icc/` 目录(ITU-R BT.709/2020、DCI-P3 等参考 ICC),可用
`icc-profile="~~/icc/..."` 手动指定。注意:mpv 校色目标曲线是
bt.1886(接近 gamma=2.4),这与 sRGB 传输曲线不同——base.conf 注释
明确警告:未校色的普通显示器开 `icc-profile-auto` 用系统 sRGB 校色
文件会导致画面显示异常(偏色/偏暗)。

## 语法与默认值

```conf
icc-profile=<路径>
icc-profile-auto[=<yes|no>]
icc-intent=<0-3>
icc-force-contrast=<no|0-1000000|inf>
```

- 默认值:全部关闭/默认(`icc-profile-auto=no`,`icc-intent=1` 相对色度)
- 来源:mpv 手册 OPTIONS 章节 icc-* 条目
- 本仓库 base.conf:默认注释关闭,并给出完整调参注释
  (icc-intent 推荐 0 感知度映射、icc-force-contrast=1000 解决偏暗)

## 示例

```conf
# 专业校色环境:自动加载系统校色文件
icc-profile-auto

# 手动指定仓库自带的参考 ICC
icc-profile="~~/icc/ITU-RBT709ReferenceDisplay.icc"

# 校色文件对比度过高导致画面偏暗时
icc-force-contrast=1000
```

## 风险提示

- **未校色的显示器不要开 `icc-profile-auto`**:系统默认 sRGB 校色文件
  与 mpv 的 bt.1886 目标不匹配,画面会偏色偏暗(issue #8009)。
- 广色域屏未校色时,更实际的做法是手动指定 `target-prim`/`target-trc`
  (见教程《hdr — HDR 映射》),而不是随便挂一个 ICC。
- `icc-profile` 会覆盖 `target-prim`、`target-trc` 与
  `icc-profile-auto`;改参数后需重启播放器生效。

## 相关选项

- `icc-3dlut-size`:3D LUT 尺寸(默认 auto,越大越准越慢)
- `icc-cache` / `icc-cache-dir`:LUT 缓存,加速启动
- `use-embedded-icc-profile`:媒体内嵌 ICC 的使用开关
