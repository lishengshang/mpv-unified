# script-opts — 脚本配置

## 用途

mpv 的 Lua/JS 脚本(如 uosc、文件浏览器)通过 `script-opts/`
目录下的配置文件定制行为,命名约定 `<脚本名>.conf`,内容是
`key=value` 行(与 mpv.conf 语法相同,支持注释)。

本仓库规范:
- 所有脚本配置模板以 `.example` 后缀入库(如 `script-opts/uosc.conf.example`),
  实体文件由用户/安装流程生成,避免升级覆盖自定义;
- **API 密钥类模板**严格保持 .example 形态,实体文件被 gitignore
  (见教程《layers — 四层配置结构》与 docs/platform-assets.md);
- 包安装流程会把脚本的 config 片段合并进 `config.d/packages/`
  包层(见包商店文档)。

## 语法与默认值

```conf
# script-opts/<脚本名>.conf
key=value
```

- 路径约定:mpv 配置目录下的 `script-opts/` 子目录
- 来源:mpv 手册 OPTIONS 章节 `script-opts` 选项与各脚本 README
- 本仓库:`archive/script-opts/` 保留了全部历史脚本配置模板,
  当前 `script-opts/` 存放实体配置与 .example 模板

## 示例

```conf
# script-opts/uosc.conf(示例片段)
menu_items=subtitles,audio,tracks,playlist
show_volume=no
```

## 风险提示

- 脚本配置的 key 必须与脚本代码里 `mp.get_opt` 读取的完全一致,
  写错不报错、静默无效;每个脚本的可用 key 看它的 README。
- **密钥只写实体文件**:任何 `.example` 模板都不得含真实值,CI
  会扫描仓库(见 docs/platform-assets.md 密钥扫描约定)。
- 改名脚本后旧 `<脚本名>.conf` 不再生效但文件仍在,注意清理。

## 相关选项

- `script-opts`:mpv 中查看脚本配置的命令(播放中用 `show-text`)
- `scripts` / `script`:脚本加载选项
- `layers`:配置分层(见教程《layers — 四层配置结构》)
