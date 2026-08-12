# keep-open — 播放结束后的行为

## 用途

控制文件播放完毕后播放器做什么:立即退出、留在最后画面、还是回到
开始。搭配 `idle` 与 `force-window` 可组合出"播放器常驻"的各种形态
(见教程《idle — 空闲模式》与《force-window — 强制窗口》)。

`keep-open=yes` 的典型场景:看完一集想让画面停在片尾、配合截图、
或避免窗口频繁开关。

## 语法与默认值

```conf
keep-open=<no|yes|always>
```

- `no`:播放完立即继续下一个文件/退出(默认);
- `yes`:**最后一个**文件播完后保持打开,播放列表内的文件仍自动
  衔接;
- `always`:每个文件播完都停留,需要手动进入下一集。

默认值:`no`。来源:mpv 手册 OPTIONS 章节 keep-open 条目。

## 示例

```conf
# 列表播完停在最后画面,不退出
keep-open=yes

# 每集播完都停(手动下一集)
keep-open=always
```

## 风险提示

- `keep-open=yes` 时,播完的"下一集自动播放"依赖播放列表;从文件
  管理器单文件打开时,`yes` 与 `always` 行为一致(都会停住)。
- 停住时状态是"暂停在结尾",此时按 `>`(下一集)可继续,但
  watch-later 进度记忆会记住结尾位置(见教程《watch-later — 记忆播放进度》)。
- 挂 `--no-keep-open` 等命令行覆盖可在个别会话临时关闭,不影响
  user.conf 里的持久配置。

## 相关选项

- `idle`:空闲时是否保持运行(见教程《idle — 空闲模式》)
- `force-window`:无媒体时是否仍显示窗口
- `loop-file` / `loop-playlist`:循环播放
