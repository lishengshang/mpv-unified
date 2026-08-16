# mpv-config 快速上手(QUICKSTART)

一个配置调好的 mpv 播放器整合包:解码画质方案、脚本(字幕下载/弹幕/续播等)、
着色器、uosc 现代界面都已配好,解压即用,随版本一键更新,个人定制永不丢失。

## Windows:三步开始

1. **解压** zip,得到 `mpv-config/` 目录(mpv 播放器本体已随包捆绑)。
2. **双击 `mpv.exe`**,把任意视频文件拖进窗口即可播放。
3. 播放中**右键**打开 uosc 菜单,「方案」子菜单可随时切换观影方案
   (高清观影 / 弹幕直播 / 低延迟游戏等)。

> 想放到别处?整个目录是自包含的,移动后依然可用;不要单独移动
> `portable_config/`(mpv 便携模式要求它与 mpv.exe 同级)。

## Linux / macOS

zip 不捆绑 mpv 本体,请先用系统包管理器安装 mpv(建议 ≥ 0.36):

```bash
# Arch: sudo pacman -S mpv    Debian/Ubuntu: sudo apt install mpv    macOS: brew install mpv
```

然后把 `portable_config/` 内的全部内容复制到 `~/.config/mpv/`(目录不存在
则新建);命令行运行 `mpv 视频.mp4` 即生效。

## 个性化你的配置

个人设置写在 `user/user.conf`(首次运行工具会自动从
`user/user.example.conf` 模板创建)。改完后在**本目录内**运行:

```bash
mpv-config gen            # 重新生成 portable_config/(user 层会自动叠加)
```

或打开图形界面 `mpv-config-gui.exe`,在「配置」页表单调整、「方案」页启停
方案,点「应用并生成」。

- 分层优先级:`gui.conf` > `user.conf` > 方案 > package > 平台层 > base
- **升级时 user/ 目录永不覆盖**,API key 等私货只写在这里
- 手动编辑配置文件时保持 UTF-8 编码、Unix 换行,否则 mpv 可能无法识别

## 更新

```bash
mpv-config check-update     # 检查新版本
mpv-config upgrade --yes    # 一键更新:配置+插件+着色器+mpv 本体(app 层整体替换,user 层保留)
```

mpv 播放器本体随本项目的版本发布更新(出处见 `MPV-BUILD.txt`),升级命令
一次全部更新,无需单独更新 mpv。

## 自检与排障

```bash
mpv-config doctor    # 体检:配置语法/条件指令/平台完整性/密钥(0=健康 2=仅警告)
```

常见问题:

- **双击 mpv.exe 闪退**:在 cmd 中运行 `mpv.com 文件.mp4` 查看报错输出。
- **脚本/界面异常**:确认是从本目录整体解压(portable_config/ 与 mpv.exe
  同级),再跑一次 `mpv-config gen`。
- **看不到中文教程**:图形界面「帮助」页内置全部长尾选项教程;仓库
  `docs/tutorials/` 亦可直接阅读。

更多细节见同目录 `README.md`。
