# mpv 脚本分类参考

> 此文件仅供查阅，修改/移动/删除不影响 mpv 功能。
> 快捷键绑定来自 `input.conf`，标记 `#` 的为注释掉的备用绑定。

---

## 一、UI / 界面增强

### uosc（子目录）
现代化 OSC 界面，替代 mpv 自带播放条。
| 快捷键 | 功能 |
|--------|------|
| `o` | 打开 uosc 文件浏览器 |
| `MENU` / 鼠标右键 | 开/关 uosc 菜单 |
| `Space` / 播放键 | 暂停 + 闪烁指示 |
| 滚轮 / `9` `0` | 音量 ±1 + 闪烁提示 |
| `[` `]` `{` `}` `BS` | 速度调节 + 闪烁提示 |
| `Alt+o` | 定位当前文件所在目录 |

### uosc_danmaku（子目录）
弹幕支持：从 Bilibili / 巴哈姆特 / 弹弹Play 加载弹幕渲染到视频上。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+d` | 弹幕发送面板 |
| `Ctrl+D` | 弹幕综合菜单 |

### thumbfast.lua
高性能实时缩略图生成器，供 uosc 时间轴悬停预览。
> 无独立快捷键，由 uosc 自动调用。

### file-browser（子目录）
OSD 风格文件浏览器，支持本地/网络目录。
| 快捷键 | 功能 |
|--------|------|
| `Tab` | 打开 OSD 文件浏览器 |

### simple-mpv-webui（子目录）
Web 远程控制界面。
> 无快捷键，通过浏览器访问。

---

## 二、播放列表 / 文件管理

### autoload.lua
播放文件时自动将同目录其他媒体加入播放列表（按字母排序）。
> 无快捷键，自动运行。

### playlistmanager.lua
播放列表显示与随机播放。
| 快捷键 | 功能 |
|--------|------|
| `n` | 随机播放 |
| `F5` | 显示播放列表 |

### open_dialog.lua
调用系统原生文件选择对话框。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+o` | 打开原生文件浏览器导入文件 |

### delete-current-file.lua
将当前播放文件移到回收站并从播放列表移除（需 `trash-cli`）。
> 与 `delete_current_file.lua` 功能相同（保留两个以防兼容问题）。

### delete_current_file.lua
同上，删除当前文件。
| 快捷键 | 功能 |
|--------|------|
| `Delete` | 删除当前文件（按 `1` 确认） |

### blacklist-extensions.lua
根据扩展名黑白名单过滤自动加载的文件。
> 无快捷键，自动运行。

### recentmenu.lua
近期播放文件菜单。
> 通过 uosc 菜单访问。

---

## 三、播放历史 / 书签

### simplehistory.lua
记录和管理播放历史。
| 快捷键 | 功能 |
|--------|------|
| `` ` `` | 打开历史菜单 |
| `Alt+l` | 开/关隐身模式 |
| `Ctrl+L` | 加载最后播放的文件 |
| `Ctrl+l` | 加载最后播放文件并恢复进度 |

### simplebookmark.lua
保存和管理播放进度书签。
| 快捷键 | 功能 |
|--------|------|
| `N` | 打开书签菜单 |
| `Ctrl+n` | 添加进度书签 |
| `Ctrl+N` | 添加文件书签 |

### history-bookmark.lua
精简版历史书签（sorayuki），自动记录和恢复播放位置。
> 无快捷键，自动运行。

---

## 四、字幕

### sub-assrt.lua
从 assrt.net 搜索下载字幕。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+f` | 打开字幕下载菜单 |

### sub_export.lua
导出视频内封字幕轨为外挂文件（需 ffmpeg）。
| 快捷键 | 功能 |
|--------|------|
| `Alt+m` | 导出当前内封字幕 |

### sub-select.lua
智能字幕轨选择——比 mpv 内置 `--slang` 更准确。
> 无快捷键，自动运行。

### sub-fonts-dir-auto.lua
根据字幕文件中的字体引用自动添加字体查找目录。
> 无快捷键，自动运行。

### autosubsync（子目录）
自动将字幕与音频对齐同步。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+m` | 打开字幕同步菜单 |

---

## 五、音频

### trackselect.lua
智能音轨选择：自动选非配音（non-dub）音轨，避免日语片选了英文轨。
> 无快捷键，自动运行。

### fix-avsync.lua
切换音频输出设备时自动修复 A/V 同步。
> 无快捷键，自动触发。

---

## 六、画面增强

### dynamic-crop.lua
使用 cropdetect 滤镜自动检测并裁切视频黑边。
| 快捷键 | 功能 |
|--------|------|
| `C` | 循环：启用 → 保留裁切禁用 → 完全禁用 |

### hdr-mode.lua
根据视频内容自动切换显示器 SDR/HDR 模式（Windows 专用）。
> 无快捷键，自动运行。

---

## 七、视频片段 / 导出

### slicing_copy.lua
标记起止点，用 ffmpeg 无损剪切导出视频片段。
| 快捷键 | 功能 |
|--------|------|
| `c` | 指定剪切起始/结束位置 |
| `a` | 开/关导出时包含音频 |
| `Ctrl+C` | 清除剪切标记 |

### mpv-animated.lua
导出 WebP/GIF 动图（需 ffmpeg）。
| 快捷键 | 功能 |
|--------|------|
| `w` | 设置动图起始时间 |
| `W` | 设置动图结束时间 |
| `Ctrl+w` | 导出动图 |
| `Ctrl+W` | 导出带字幕的动图 |

### chapter-make-read.lua
制作/编辑外部章节文件（chp/ogm），支持读取外部章节。
| 快捷键 | 功能 |
|--------|------|
| `Alt+c` | 标记章节时间 |
| `Alt+e` | 编辑章节标题 |
| `Alt+r` | 删除当前章节 |
| `Alt+w` | 导出 chp 章节文件 |
| `Alt+g` | 导出 ogm 章节文件 |

### chapterskip.lua
基于章节标记自动跳过片头片尾。
| 快捷键 | 功能 |
|--------|------|
| `F3` | 跳到下一个静音位置 |
| `Alt+q` | 切换章节跳过模式 |
| `Alt+n` | 标记片头片尾 |

---

## 八、播放控制

### evafast.lua
仿 B 站播放器：短按方向键跳转，长按加速快进。
> 增强 `Right`/`Left` 行为，无独立快捷键。

### undoredo.lua
跳转操作的撤销/重做。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+z` | 撤销跳转 |
| `Ctrl+x` | 重做跳转 |
| `Ctrl+Alt+z` | 循环跳转 |

### cycle-commands.lua
在多个命令间循环切换（用于配置组轮换）。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+P` | 循环切换配置组 (FSRCNNX → FSRCNNX+ → NNEDI3 → ravu-zoom → Anime4K) |

### pip.lua
画中画（PiP）模式，窗口置顶悬浮。
| 快捷键 | 功能 |
|--------|------|
| `p` | 开/关画中画 |

---

## 九、网络 / 在线播放

### quality-menu.lua
切换 YouTube/Bilibili 流媒体的视频和音频质量。
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+F` | 切换视频质量 |
| `Alt+F` | 切换音频质量 |

### sponsorblock_minimal.lua
自动跳过 YouTube/Bilibili 视频中的赞助片段、开场白等（基于 SponsorBlock）。
> 无快捷键，自动跳过。

### mpv-torrserver.lua
与 Torrserver 联动，播放磁力链接/种子。
> 通过 uosc 菜单使用。

### clipboard-magnet.lua
从剪贴板读取并播放磁力链接。
| 快捷键（注释备用） | 功能 |
|--------|------|
| `# Ctrl+Shift+v` | 播放剪贴板中的磁力链接 |

---

## 十、系统 / 辅助

### manager.lua
一键更新所有脚本和着色器。
| 快捷键 | 功能 |
|--------|------|
| `M` | 一键更新脚本和着色器 |

### auto-save-state.lua
定时自动保存播放状态，防止意外退出丢失进度。
> 无快捷键，后台自动运行。

### persist_properties.lua
跨会话持久化指定属性（音量、字幕大小等），重启后恢复。
> 无快捷键，自动运行。

### inputevent.lua
提供连击（双击/三击/五击）、长按等高级输入事件检测。
> 无快捷键，为其他脚本提供事件能力。

### mpv-cropscreen.lua
区域截图和图片拼接工具。
> 无快捷键绑定。

---

## 十一、VapourSynth 视频滤镜

> VapourSynth 滤镜通过 `input.conf` 的 VF 快捷键加载，需要 VapourSynth + k7sfunc + CUDA 插件才能运行。
> 已移除 Windows 专属的 DML / SVP / MIGX 脚本。

### 补帧类
| 快捷键 | 脚本 | 说明 |
|--------|------|------|
| `!` | MEMC_MVT_LQ.vpy | MVTools CPU 补帧，开销最低 |
| `@` | MEMC_RIFE_STD.vpy | RIFE 通用补帧，跨平台 |
| `$` | MEMC_RIFE_NV.vpy | RIFE NVIDIA CUDA 补帧 |
| `%` | MEMC_DRBA_NV.vpy | DRBA NVIDIA CUDA 补帧 |

### 画质增强类
| 快捷键 | 脚本 | 说明 |
|--------|------|------|
| `SHARP` | SR_ARTCNN_NV.vpy | ArtCNN NVIDIA 超分辨率 |
| `^` | NR_BM3D_NV.vpy | BM3D NVIDIA 降噪 |
| `&` | MIX_UAI_NV_TRT.vpy | UAI TensorRT NVIDIA 自定义AI超分 |

### 其他可用（无快捷键绑定）
| 脚本 | 说明 |
|------|------|
| ETC_DEINT_EX.vpy | 反交错处理 |
| MIX_UVR_MAD.vpy | 音频人声/伴奏分离 |
| NR_CCD_STD.vpy | CCD CPU 降噪（通用） |
| SR_ACNET_STD.vpy | ACNET CPU 超分（通用） |

### 清空滤镜
| 快捷键 | 说明 |
|--------|------|
| `~` | 清空所有 VF 滤镜 |

---

## 统计

| 类别 | 脚本数 | 有快捷键 |
|------|--------|----------|
| UI / 界面 | 5 | 3 |
| 播放列表 / 文件管理 | 6 | 4 |
| 播放历史 / 书签 | 3 | 2 |
| 字幕 | 5 | 3 |
| 音频 | 2 | 0 |
| 画面增强 | 2 | 1 |
| 视频片段 / 导出 | 4 | 4 |
| 播放控制 | 4 | 3 |
| 网络 / 在线 | 4 | 2 |
| 系统 / 辅助 | 5 | 1 |
| VapourSynth 滤镜 | 4 | 7 |
| **合计** | **44** | **30** |

*注：子目录模块（uosc、uosc_danmaku、file-browser、autosubsync、simple-mpv-webui）各算 1 个。*
*标记 `#` 的快捷键已被注释，实际不生效。*
