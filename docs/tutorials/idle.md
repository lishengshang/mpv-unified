# idle — 空闲模式

## 用途

控制 mpv **没有任何媒体可播**时是否保持运行:
- `idle=no`(默认):命令行播放完直接退出;
- `idle=yes`:一直挂着,等待后续文件/命令;
- `idle=once`:启动时没给文件也不退出,但播完最后一个文件后退出。

GUI 场景(本仓库的形态)通常要 `idle=yes`,否则"打开播放器但还没
选文件"窗口就退出了;配合 `force-window` 让空窗口可见(见教程
《force-window — 强制窗口》)。

## 语法与默认值

```conf
idle=<no|yes|once>
```

- 默认值:`no`
- 来源:mpv 手册 OPTIONS 章节 idle 条目
- 本仓库 base.conf 使用:`idle=yes`,注释:"空闲待机(中止播放或所有
  文件播放后依旧保持 mpv 运行)";`user.example.conf` 中也示范了
  `idle=yes`

## 示例

```conf
# 播放器常驻(GUI 推荐)
idle=yes

# 命令行单次播放:给文件就播,不给就退
idle=once
```

## 风险提示

- `idle=yes` 时进程常驻,记得检查是否有 watch-later/缓存写入
  (见教程《watch-later — 记忆播放进度》);无媒体时的空闲窗口
  若没有 `force-window` 会是"看不见的窗口",任务栏也找不到。
- `idle=once` 在"先启动后塞文件"(IPC 场景)时会提前退出,IPC
  用户应保持 yes。
- 脚本/自动化场景:确认退出逻辑(如 `quit` 命令、超时退出),避免
  常驻进程堆积。

## 相关选项

- `force-window`:空闲时显示空窗口(见教程《force-window — 强制窗口》)
- `keep-open`:播放完的行为(见教程《keep-open — 播放结束后的行为》)
- `osc` / uosc:空闲窗口里的界面
