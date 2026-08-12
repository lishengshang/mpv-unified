## MPV config ([English branch](https://github.com/dyphire/mpv-config/tree/eng))

### 项目介绍

本项目为 windows 下 [mpv](https://github.com/mpv-player/mpv) 播放器的配置文件，应放入`mpv.exe`所在目录的`portable_config`文件夹内，

或 mpv 配置默认路径`%APPDATA%/mpv/`下，这种方式全局生效。

使用`portable_config`时会覆盖全局配置方案。

PS：自行编辑配置文件时，注意编码格式应为 UTF-8，换行符为 Unix，否则 MPV 可能无法识别

**mpv 整合包下载**：[Releases](https://github.com/dyphire/mpv-config/releases)

### 快速开始(三平台)

> **最低支持 mpv 版本:0.36**(决策 D12⑤:各平台跟随较新稳定版;`apply-profile`
> 命令与 uosc 5.x 均要求 ≥ 0.36,建议直接使用最新稳定版)。

1. **下载**:到 [Releases](https://github.com/dyphire/mpv-config/releases) 下载
   对应平台的 zip(`mpv-config-<linux|windows|macos>-<version>.zip`)。
2. **解压**:解压后得到 `mpv-config/` 目录(app 层配置 + 生成器;`user/`
   由首次运行自动创建,升级时原样保留)。
3. **放置**(两种方式任选):
   - **Windows**:把 `portable_config` 目录放入 `mpv.exe` 所在目录(推荐,
     覆盖全局配置);或把生成内容放入 `%APPDATA%/mpv/`(全局生效)。
   - **Linux / macOS**:放入 `~/.config/mpv/`(或 `$XDG_CONFIG_HOME/mpv`)。
4. **使用**:运行 `mpv-config gen`(或使用图形界面"方案/配置/应用并生成"),
   把生成的 `dist/` 内容放入 mpv 配置目录即可;播放中右键 uosc 菜单
   「方案」子菜单可随时切换方案(见 [uosc 联动](docs/uosc-integration.md))。

> 自行编辑配置文件时,注意编码格式应为 UTF-8,换行符为 Unix,否则 MPV
> 可能无法识别。macOS 平台层标记 experimental,暂未真机验证。

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

### 致谢

本项目配置体系源自以下上游项目 (MIT 许可 fork 链继承, 见根 `LICENSE.MD`):

* [lishengshang/mpv-config](https://github.com/lishengshang/mpv-config) — Linux 移植版配置基础
  (现役 `~/.config/mpv/` 的移植来源, 本仓库的 config/ 四层结构即由此衍生)
* [hooke007/mpv-lazy](https://github.com/hooke007/mpv-lazy) — 配置手册与 vs/ 滤镜脚本
  (k7sfunc 补帧/超分方案、`vs-plugins/models` 模型约定)
* [dyphire/mpv-config](https://github.com/dyphire/mpv-config) — Windows 原版配置与脚本着色器集
  (本仓库主体内容来源)

### 包管理(`mpv-config pkg`,M2 阶段)

```bash
mpv-config pkg migrate-manager [manager.json]   # 一键迁移现役源为 packages/pending/ 待安装记录
mpv-config pkg update-index [--index-url URL]   # 拉取并校验远程 index.json 到缓存(显式命令)
mpv-config pkg install <name>                   # 安装:packages/pending/ → packages/ → 索引 三源优先序
mpv-config pkg uninstall <name>                 # 卸载(共享文件仅当最后使用者才删除)
mpv-config pkg update [name]                    # 更新(缺省全部);原子替换,旧版备份到 ~/.cache/mpv-config/backup/
```

安装目标为仓库根(`~~/` 展开),绝不触碰 `~/.config/mpv/`。安装结果记录于仓库根 `packages.lock`(T14 最小格式,T16 的 verify/repair 将接管该文件)。

MVP 限制(如实声明):
- **不自动补装依赖**:依赖缺失时报错并提示 `mpv-config pkg install <dep>`,需手动先装。
- 依赖为名称级(无版本区间);冲突检测只报不自动解决。
- pending(git)源无版本号,lock 中记为 `0.0.0` 占位,`update` 时总是重新克隆。
- whitelist/blacklist 为正则(来自 legacy manager.json 的 Java 风格模式,原样套用 Rust regex);非法正则报错而非静默忽略。
- 安装中途失败会回滚已拷贝文件与已备份的旧文件;若回滚本身失败,错误信息会指出残留位置。
- 共享文件 = 多个包 lock 记录同一路径:卸载仅在最后使用者离开时删除;同名文件更新时直接覆盖(旧内容先备份)。

### 目录结构

```
config/             # 配置源: base + 平台层 + input 层 (T6 生成最终 mpv.conf)
cli/ core/ pkg/ tools/   # mpv-config-manager 本体 (Rust + CLI)
user/               # user 层模板 (user.example.conf, 实体已 gitignore)
scripts/            # mpv 脚本 (lua/子目录) + display-info.dll (Windows)
script-opts/        # 脚本配置 (API key 类为注释模板形态, 实体不入库)
shaders/            # 着色器 (Ani4k/Anime4K/AnimeJaNai/igv/nnedi3/other/ravu)
fonts/              # 图标字体 (uosc/OSC 用)
icc/                # 色彩管理 ICC 配置
vs/                 # VapourSynth 滤镜脚本 (依赖外部 k7sfunc + models, 见 docs/platform-assets.md)
osc-style/ script-modules/ archive/   # 备用 OSC 样式 / 脚本模块 / 原版备份
CATEGORIES.md       # 脚本分类参考 (包商店 11 类, 决策 D8)
docs/               # 平台差异 / 平台特有资产 / 等价验证白名单
```

平台特有文件取舍详见 [`docs/platform-assets.md`](docs/platform-assets.md)。

