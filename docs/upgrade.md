# 升级路径:app/user 分离布局

> 本文档定义发布包的目录契约与升级流程。核心原则一句话:
> **app 层(可替换)与 user 层(保留)严格分离,升级只动 app 层,user 层永不随版本覆盖。**

## 1. 发布 zip 内部结构

解压后的 `mpv-config/` 目录分两层:

### app 层(可替换,升级时整体替换)

| 路径 | 内容 |
| --- | --- |
| `config/` | 分层配置源(`base.conf` / `{platform}.conf` / `input.conf`) |
| `scripts/` | Lua 脚本 |
| `script-opts/` | 脚本配置模板(`.example`) |
| `shaders/` | 着色器 |
| `fonts/` | 字体 |
| `vs/` | VapourSynth 脚本与依赖 |
| `mpv-config`(或 `mpv-config.exe`) | CLI 二进制 |
| `VERSION` | 版本号文件(`doctor --upgrade-check` 读取) |
| `LICENSE.MD` | 许可证 |

### user 层(保留,首次运行自动创建)

| 路径 | 内容 |
| --- | --- |
| `user/user.conf` | 个人配置(四层合并中优先级最高的覆盖层) |
| `user/gui.conf` | GUI 表单写入的配置片段(只写非默认值) |
| `user/profiles-state.json` | 方案卡片启用状态 |
| `user/script-opts/` | 个人脚本配置实体(含 API 密钥) |

user 层由 `mpv-config gen` **首次运行时自动创建**:`user/` 目录不存在时自动建立,
并把 `user/user.example.conf` 模板复制为 `user/user.conf`(模板缺失时写入默认头部注释)。
发布 zip 内**不含** user 层内容,只含 `user.example.conf` 模板。

## 2. 升级流程(手动 / UI 触发)

1. **下载新版 zip**(GitHub Releases)并解压到临时目录。
2. **备份旧版 app 层**(可选但推荐):
   - 手动:复制整个 `mpv-config/`(含 `user/`)到 `backup/`;
   - 或只备份 user 层:复制 `user/` 到安全位置。
3. **替换 app 层**:把新版 zip 中的 app 层文件覆盖到安装目录。
   注意:覆盖时**不要删除或覆盖 `user/` 目录**。
4. **user 层原样保留**:`user/` 下所有文件(含密钥)保持不动。
5. **验证**:
   - `mpv-config doctor --upgrade-check` 确认版本号;
   - `mpv-config gen` 重新生成配置,检查输出与警告。

## 3. 铁律

- **user 层永不随版本覆盖**:升级动作(手动或 UI)只允许替换 app 层文件。
- **API key 放 user 层**:所有密钥/令牌写入 `user/` 下的文件——该目录已被
  `.gitignore` 忽略,不会入库,也不会随升级丢失。
- **升级失败不丢数据**:唯一可能被覆盖的只有 app 层(可从 zip 重新解压);
  user 层在升级中不可被触碰,因此不存在数据丢失面。
- **user.conf 永不自动重写**:`gen` 只在 `user/user.conf` 不存在时创建;
  若文件已存在(哪怕内容非法导致 `gen` 报错),报错但**绝不覆盖**原文件。

## 4. 与 .gitignore 的关系

仓库 `.gitignore` 忽略 `user/*`(保留 `user/user.example.conf` 模板),
保证个人配置与密钥永不入库、永不进发布 zip。
