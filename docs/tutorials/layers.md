# layers — 四层配置结构

## 用途

本仓库的配置组织方式:一份源配置 + 平台规则 + 个人覆盖,gen 时
合并出最终 mpv.conf。四层顺序(后层覆盖前层):

1. `config/base.conf`:通用配置(所有平台共享,本仓库主体);
2. `config/{linux|windows|macos}.conf`:平台层(平台差异,如
   gpu-api=linux 用 vulkan / windows 用 d3d11);
3. 包层 `config.d/packages/`:已安装包的配置片段;
4. `user/user.conf`:你的个人覆盖(优先级最高)+ `user/gui.conf`
   (GUI 表单管理的片段,在 user.conf 之上)。

理解分层,你就知道"改哪里":跨平台通用的写 base(源码,改库文件),
个人偏好写 user(user 层,升级不覆盖)。

## 语法与默认值

- 生成:`mpv-config gen [--platform linux|windows|macos] [--out 目录]`
- 层内仍支持条件指令 `#@if platform==...`(见 core 文档)
- 本仓库:拆分产物见 `config/`,等价验证脚本 `tools/verify-equivalence.sh`

## 示例

```conf
# user/user.conf(个人覆盖层,示例)
sub-font-size=44          # 覆盖 base 的字幕字号
hwdec=no                  # 本机软解更稳
```

## 风险提示

- **只编辑 user 层**:GUI 的"高级配置"编辑器也锁定 user/user.conf,
  禁止直接改 base.conf 等源文件(UI 有说明);改源文件请走 git 流程。
- 覆盖规则是"同名键整行覆盖",注释不会消失——各层注释按顺序
  自然拼接,担心混乱就每层写清注释。
- `user/` 整个目录被 gitignore,密钥放这里不会入库(CI 另有扫描
  兜底,见 docs/platform-assets.md)。

## 相关选项

- `mpv-config doctor`:校验全部层文件语法(带行号报错)
- `mpv-config gen --dry-run`:预览将生成的合并结果
- 教程《profiles — profile 块用法》:方案的另一种组织方式
