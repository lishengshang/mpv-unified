# 平台特有文件筛查 (Task 8)

> 审计日期: 2026-08-12
> 范围: `scripts/` `shaders/` `vs/` `script-opts/` 及配置层中平台特有项。
> 目的: 供打包/发行时按平台取舍 (决策 D13: 资产是产品的一部分)。

## 一、Windows 特有项

| 文件 | 类型 | 说明 |
|------|------|------|
| `scripts/display-info.dll` | 二进制 (PE32+ x86-64 DLL) | display-info 脚本的 WinAPI 依赖, 仅 Windows 需要 |
| `script-opts/ytdl_hook.conf` | 配置 | `yt-dlp.exe` 更新命令引用 |
| `script-opts/file_browser.conf` | 配置 | `windir` 插件依赖 `cmd.exe` 的 `dir` 命令 |
| `archive/script-opts/changerefresh.conf` | 配置 (归档) | 依赖 `nircmd.exe` |
| `archive/scripts/display-name.lua` | 脚本 (归档) | 依赖 `MultiMonitorTool.exe` (nirsoft) |
| `archive/scripts/youtube-download.lua` | 脚本 (归档) | 依赖 `wt.exe` (Windows Terminal) |
| `scripts/uosc/README.md` | 文档 | 安装器为 `windows.ps1` (irm \| iex) |
| `scripts/uosc/main.lua` | 脚本 | 内建二进制选择: `ziggy-windows.exe` |

## 二、Linux 特有项

| 文件 | 类型 | 说明 |
|------|------|------|
| `tools/verify-equivalence.sh` + `tools/verify_equivalence.py` | 工具 | 配置等价验证 (bash + python3) |
| `scripts/uosc/README.md` | 文档 | Unix 安装器 `unix.sh` (curl \| bash) |
| `config/linux.conf` | 配置源 | `gpu-api=vulkan`、`wl-paste` 剪贴板、Linux 字体/路径差异 |
| `config/linux.input.conf` | 配置源 | Linux 差异按键 (wl-paste 等) |

> 另见 `docs/platform-diff.md` (配置层平台差异逐条清单)。

## 三、VapourSynth (`vs/`) 依赖

`vs/*.vpy` 均为纯 Python 脚本, **无内嵌 .dll/.so 路径引用**; 平台依赖在外部环境:

- **k7sfunc**: `import k7sfunc as k7f` — 外部 VapourSynth Python 插件库 (非仓库内)
- **模型文件**: 注释指向 `.../vs-plugins/models/` 目录 (如 `SR_AnimeJaNaiV3_NV.vpy`、`MIX_UAI_NV_TRT.vpy`)
- **硬件**: `_NV` 变体 (NVENC/TRT) 需 NVIDIA GPU; `_DML` 变体需 Windows DirectML; `_STD`/`_EX` 为 CPU/通用
- 仓库内 18 个 .vpy: `ETC_DEINT_EX` `MEMC_DRBA_DML/NV` `MEMC_MVT_LQ` `MEMC_RIFE_DML/NV/STD`
  `MEMC_SVP_PRO` `MIX_SR_MEMC_NV` `MIX_UAI_DML/NV_TRT` `MIX_UVR_MAD` `NR_BM3D_NV` `NR_CCD_STD`
  `SR_Anime1080Fixer_NV` `SR_AnimeJaNaiV3_NV` `SR_ARTCNN_NV` `SR_HFA2k_NV`

## 四、字体路径引用

- Windows: `config/windows.conf` (d3d11 相关字体/便携路径)、`archive/mpv.conf.orig`
- Linux: `config/linux.conf` (Linux 字体路径)
- 字体资产: `fonts/` (图标字体为主); 系统字体 (Noto CJK 等) 不入库, 由系统提供

## 五、资产来源 (Task 8 迁移清单)

现役 `/home/mio/.config/mpv/` (只读源) 与仓库合并, **以仓库为主、补缺**:

### scripts/ (active 独有 → 复制入仓库)
- `clipboard-magnet.lua` — 剪贴板磁链嗅探
- `clipboard-paste.lua` — 剪贴板粘贴
- `sub-select.lua` — 字幕选择 (配套 `script-opts/sub_select.conf` + `sub-select.json`)

### script-opts/ (active 独有 → 复制入仓库)
- `sub_select.conf` — sub-select 配置
- `sub-select.json` — sub-select 规则

### fonts/ (active 独有图标字体 → 复制入仓库)
- `Material-Design-Iconic-Font.ttf` (99KB)
- `MaterialIconsRound-Regular.otf` (400KB)
- `modernx-osc-icon.ttf` (4.7KB)

### 分类文档
- `CATEGORIES.md` → 仓库根 (决策 D8: 包商店 11 类分类参考)

### 明确不复制 (及原因)
| 项 | 原因 |
|----|------|
| `scripts/manager.lua` | manager.json 配套, 新体系由 T12 迁移器替代 |
| `script-opts/file_browser_favourites.txt` | 个人收藏数据, 非脚本配置模板 |
| `fonts/NotoSansCJK-Bold.ttc` `NotoSerifCJK-Bold.ttc` | 20-27MB 系统字体, 非仓库资产 |
| `scripts/delete_current_file.lua` | repo 已有 `delete-current-file.lua` (以仓库命名为主) |
| `script-opts/sub_assrt.conf` (现役实体含真实 api_token) | 实体密钥不入库; 仓库保留 .example 形态模板 (key 已注释) |

### 目录对比结果 (无需复制的部分)
- `shaders/`: 7 子目录 (Ani4k/Anime4K/AnimeJaNai/igv/nnedi3/other/ravu) 与仓库逐一 diff **全部一致**
- `vs/`: active 9 个 .vpy 已全部在仓库; 仓库另有 9 个 active 没有的 (DML/SVP 等)
- `archive/` `icc/` `osc-style/` `script-modules/`: 仓库已有, 未变动

## 六、密钥安全说明

- 扫描 (命令见 `.omo/evidence/task-8-mpv-config-manager.txt`) 确认仓库**无个人实体密钥**。
- `script-opts/sub_assrt.conf` 仅保留注释态示例 key (上游脚本内置预设)。
- `trakt_scrobble` / `uosc_danmaku` 中的 client_id/tmdb key 为上游开源脚本自带的
  **公共应用凭据** (随上游发行, 非个人密钥), 按"不修改脚本内容"约束保留原样。
