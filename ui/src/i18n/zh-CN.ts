/**
 * 中文文案表(默认语言)。结构即 MessageSchema:en-US.ts 以
 * `MessageSchema` 类型标注,漏译/多译会在 vue-tsc 编译期报错。
 * `{name}` 为运行时参数占位符,由 useI18n 的 t() 替换。
 */
export const zhCN = {
  app: {
    brand: "mpv-config",
    langToggle: "中 / EN",
    langTitle: "切换语言",
  },
  nav: {
    profiles: "方案",
    config: "配置",
    store: "包商店",
    help: "帮助",
  },
  common: {
    loading: "加载中…",
    retry: "重试",
  },
  state: {
    emptyTitle: "这里空空如也",
  },
  profiles: {
    title: "方案",
    subtitle:
      "点击卡片启用/停用预设方案(可多选),方案作为独立 profile 块写入生成的 mpv.conf,不会改动你的手动配置。",
    enabledCount: "{n} 个已启用",
    apply: "应用并生成",
    generating: "生成中…",
    lastGen: "上次生成:{text}",
    loading: "加载方案中…",
    loadFailed: "方案加载失败",
    empty: "未找到方案定义(config/profiles.yaml)。",
    emptyHint: "在仓库 config/profiles.yaml 中添加方案后重新打开即可看到卡片。",
    missingDep: "「{name}」缺少依赖:{deps},请先到包商店安装",
    regenOk: "已重新生成配置:{summary}",
    enabledBadge: "已启用",
    missingBadge: "缺依赖",
    uoscMenuNote:
      "已生成 uosc 方案切换菜单(右键菜单「方案」子菜单);未检测到 uosc 时,gen 会提示跳过,不影响生成。",
  },
  config: {
    title: "配置",
    catGeneral: "通用",
    catVideo: "视频",
    catAudio: "音频",
    catSubtitle: "字幕",
    catPerformance: "性能",
    catNetwork: "网络",
    catWindow: "窗口",
    catOther: "其他",
    subtitle:
      "精选常用选项(共 {n} 项),保存时只把与默认不同的设置写入 user/gui.conf,不会覆盖手动编辑内容;gui.conf 优先级高于 user.conf。",
    rawEdit: "编辑原始文件 →",
    save: "保存并生成",
    saving: "处理中…",
    loading: "加载选项表中…",
    loadFailed: "配置加载失败",
    savedOk: "已保存并重新生成:{summary}",
    resetOk: "「{key}」已恢复默认",
    manualDoc: "mpv 手册",
    manualTitle: "mpv 手册",
    manualLink: "手册 ↗",
    defaultLabel: "默认:{value}",
    reset: "恢复默认",
    on: "开",
    off: "关",
    emptyCategory: "该章节暂无选项。",
    dirtyHint: "有未保存的修改,保存后写入 user/gui.conf。",
  },
  store: {
    title: "包商店",
    subtitle:
      "浏览、搜索并安装社区脚本与着色器包。安装与卸载直接调用本机包管理器,冲突与依赖会在安装前校验,不会写入现役 mpv 配置目录。",
    searchPlaceholder: "搜索包名称 / 描述…",
    searchAria: "搜索包",
    refresh: "刷新索引",
    refreshing: "刷新中…",
    filters: {
      all: "全部",
      installed: "已安装",
      updatable: "可更新",
      pending: "pending",
    },
    filterAria: "状态筛选",
    count: "{shown} / {total} 个包",
    indexUpdated: "索引已更新并校验通过:{path}",
    indexFailed: "索引更新失败:{error}",
    installOk: "{message}",
    installFailed: "安装失败:{error}",
    uninstallFailed: "卸载失败:{error}",
    updateFailed: "更新失败:{error}",
    regenHintInstall: "已安装「{name}」,请到「方案」页点击「应用并生成」重新生成配置使其生效。",
    regenHintUpdate: "已更新「{name}」,请到「方案」页重新生成配置使其生效。",
    loading: "加载包清单中…",
    loadFailed: "包清单加载失败",
    empty: "没有符合条件的包。",
    emptyIndex: "没有符合条件的包。点击「刷新索引」从 GitHub Releases 拉取最新包索引。",
    noDesc: "(无描述)",
    fileCount: "{n} 个文件",
    source: "来源:{repo}",
    status: {
      available: "未安装",
      installed: "已装 v{version}",
      updatable: "可更新",
      pending: "pending git",
    },
    versionUpdatable: "v{installed} → {next}",
    versionInstalled: "v{version}",
    versionPending: "待装 {version}",
    update: "更新",
    uninstall: "卸载",
    install: "安装",
    installClone: "安装(克隆)",
    busy: "处理中…",
  },
  help: {
    title: "帮助",
    subtitle: "查看使用指南、长尾选项教程与关于信息,快速上手 mpv-config。",
    quickStartTitle: "快速上手",
    quickStart: [
      "在「方案」页选择要启用的方案(依赖缺失的卡片会提示先到包商店安装)",
      "在「配置」页调整常用选项,或在下方打开原始文件编辑",
      "点击「应用并生成」,把 dist/ 目录内容放入 mpv 配置目录(Windows: portable_config;Linux/macOS: ~/.config/mpv)",
    ],
    uoscTitle: "uosc 联动",
    uoscDetected: "已检测到 uosc,方案切换菜单会在生成时自动写入(input.conf 末尾)。",
    uoscMissing: "未检测到 uosc,方案切换菜单将跳过(不影响其余生成)。安装 uosc 后重新生成即可。",
    tutorialsTitle: "长尾选项教程",
    tutorialsEmpty: "暂无教程。长尾选项请查阅 mpv 官方手册。",
    tutorialsLoading: "加载教程中…",
    aboutTitle: "关于",
    aboutVersion: "版本:{version}",
    aboutLicense:
      "配置体系 fork 自 lishengshang/mpv-config、hooke007/mpv-lazy 与 dyphire/mpv-config(MIT 许可 fork 链,见根 LICENSE.MD)。",
    updateCheck: "检查更新",
    updateChecking: "检查中…",
    updateErrorTitle: "检查更新失败",
    updateNoInfo: "索引未提供版本信息,无法检查更新。",
    updateUpToDate: "已是最新版本({version})。",
    updateNewVersion: "发现新版本:{current} → {latest}",
    updateWarning:
      "升级会替换 app 层文件并自动备份到缓存;user/ 个人层(个人配置与 API 密钥)不受影响。",
    updateChangelog: "查看更新日志 ↗",
    updateRun: "执行升级",
    updateRunning: "升级中…",
    updateConfirm: "确认执行升级?",
    updateConfirmYes: "确认",
    updateCancel: "取消",
    updateRegenHint: "升级后请重新生成配置,让新版本的方案与选项生效。",
    updateRegenerate: "重新生成配置",
    updateRegenerating: "重新生成中…",
    updateRegenerated: "配置已重新生成,方案与选项表单已生效",
    updateRolledBack: "app 层已从备份还原,当前版本未变化,可安全重试。",
  },
} as const;

/** Recursive widening of the zh-CN `as const` table: same key structure,
 * leaves widened to `string` / `readonly string[]`, so the en-US table can
 * hold different text while missing/extra keys stay compile errors. */
type DeepSchema<T> = T extends readonly unknown[]
  ? readonly string[]
  : T extends object
    ? { [K in keyof T]: DeepSchema<T[K]> }
    : string;

/** Structural shape of the message tables (see [`DeepSchema`]). */
export type MessageSchema = DeepSchema<typeof zhCN>;
