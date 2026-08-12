## MPV config ([English branch](https://github.com/dyphire/mpv-config/tree/eng))

### 项目介绍

本项目为 windows 下 [mpv](https://github.com/mpv-player/mpv) 播放器的配置文件，应放入`mpv.exe`所在目录的`portable_config`文件夹内，

或 mpv 配置默认路径`%APPDATA%/mpv/`下，这种方式全局生效。

使用`portable_config`时会覆盖全局配置方案。

PS：自行编辑配置文件时，注意编码格式应为 UTF-8，换行符为 Unix，否则 MPV 可能无法识别

**mpv 整合包下载**：[Releases](https://github.com/dyphire/mpv-config/releases)

### 跨平台配置源(`config/` 四层结构)

> 本项目正在将单平台配置改造为"一份源 + 平台规则"的跨平台配置源。
> Windows 原版(110KB)已备份至 `archive/mpv.conf.orig`,由下面的源文件体系替代。

```
config/
├── base.conf               # 通用配置(绝大部分内容,注释 100% 保留)
│                           #   含平台条件指令:#@if platform==windows ... #@else ... #@endif
├── windows.conf            # Windows 差异:gpu-api=d3d11 类、d3d11-*/字体路径、便携模式
├── linux.conf              # Linux 差异:gpu-api=vulkan、hwdec、Linux 路径
├── input.conf              # 通用按键绑定(含视频滤镜/着色器两节平台条件指令)
├── windows.input.conf      # Windows 差异按键(右键菜单、mpv_cropscreen 等)
└── linux.input.conf        # Linux 差异按键(wl-paste 剪贴板、gather 着色器路径等)

user/
└── user.example.conf       # user 层模板(个人配置/API key 占位,无真实密钥;
                            #   实体 user/user.conf 已被 gitignore,永不入库)

tools/
└── verify-equivalence.sh   # 语义等价验证:合并 base+平台层后与现役配置做
                            #   选项级对比(顶层 + profile 内),差异落白名单则通过
```

- **层顺序**:`base` → `{platform}` → `package` → `user`,后层覆盖前层同名选项。
- **生成**:`mpv-config gen --platform <linux|windows|macos>` 输出最终 `mpv.conf`(待 T6 落地)。
- **验证**:`tools/verify-equivalence.sh linux`(对照现役 `~/.config/mpv/mpv.conf`)
  或 `tools/verify-equivalence.sh windows`(对照 `archive/mpv.conf.orig`)。
- **等价分析**:Windows 原版 vs Linux 移植版的逐条差异清单见 `docs/platform-diff.md`(待用户确认)。

### mpv 客户端

- 目前 mpv 没有官方发布的客户端，官网上有放一些推荐的第三方编译版：[https://mpv.io/installation](https://mpv.io/installation)
  - windows 上推荐使用 shinchiro 版： [shinchiro_mpv](https://github.com/shinchiro/mpv-winbuild-cmake/releases) ![releases](https://img.shields.io/github/v/release/shinchiro/mpv-winbuild-cmake)
  - 每日构建版：[zhongfly_mpv](https://github.com/zhongfly/mpv-winbuild) [![releases](https://img.shields.io/github/v/release/zhongfly/mpv-winbuild)](https://github.com/zhongfly/mpv-winbuild/releases)
  - 基于个人修改版 [mpv](https://github.com/dyphire/mpv/tree/patch) 构建版：[dyphire_mpv](https://github.com/dyphire/mpv-winbuild) [![releases](https://img.shields.io/github/v/release/dyphire/mpv-winbuild)](https://github.com/dyphire/mpv-winbuild/releases)
    - [修改版 mpv 相关说明](https://github.com/dyphire/mpv-config/discussions/7)
- 目前比较成熟的 mpv/libmpv 前端推荐： [mpv.net](https://github.com/mpvnet-player/mpv.net) [![mpv.net](https://flat.badgen.net/github/last-commit/mpvnet-player/mpv.net?scale=1.0&cache=1800)](https://github.com/mpvnet-player/mpv.net) [![releases](https://img.shields.io/github/v/release/mpvnet-player/mpv.net)](https://github.com/mpvnet-player/mpv.net/releases)
  - 个人 mpv.net 配置文件参考：https://github.com/dyphire/mpv-config/tree/mpvnet
- 浏览器调用 mpv 播放的方法推荐
  - [mpv-handler](https://github.com/akiirui/mpv-handler) 配合脚本 [play-with-mpv](https://greasyfork.org/zh-CN/scripts/416271-play-with-mpv)
  - [external-player](https://github.com/LuckyPuppy514/external-player)
- 单实例模式：[umpv](https://github.com/zhongfly/umpv-go)

### 脚本着色器说明

本项目使用的 mpv 脚本及功能介绍详见 wiki 内容： [脚本说明-wiki](https://github.com/dyphire/mpv-config/wiki/脚本说明)

本项目涉及的着色器见 mpv.conf 中相关内容

### 预览

 ![image-20231103224421000](https://cdn.jsdelivr.net/gh/dyphire/PicGo/img/2023/11/03/image-20231103224421000.png)

![image-20231103224540075](https://cdn.jsdelivr.net/gh/dyphire/PicGo/img/2023/11/03/image-20231103224540075.png)

![image-20231103224557019](https://cdn.jsdelivr.net/gh/dyphire/PicGo/img/2023/11/03/image-20231103224557019.png)

| 拼音搜索（支持首字母）                                                                                    | 字幕下载                                                                                           |
| ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| ![image](https://cdn.jsdelivr.net/gh/dyphire/PicGo/img/2023/11/03/image-20231103224614449.png) | ![image](https://cdn.jsdelivr.net/gh/dyphire/PicGo/img/2023/11/03/image-20231103224721066.png) |

### 参考

* [hooke007 配置手册](https://hooke007.github.io/mpv-lazy/mpv.html)
* [mpv 原版官方的开发版手册（英文）](https://mpv.io/manual/master/)
* [mpv 官方文档的汉化版-hooke007](https://github.com/hooke007/mpv_doc-CN)
