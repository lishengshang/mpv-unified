#!/usr/bin/env bash
# build-dist.sh — 组装三平台成品 zip(任务 24,决策 D3/D12⑥)
#
# 用法: bash tools/build-dist.sh <platform> <out-dir>
#   platform: linux | windows | macos
#   out-dir:  zip 输出目录(自动创建)
#
# 流程:
#   1. gen 生成当前平台成品到 portable_config/(mpv 便携模式自动识别该目录)
#   2. 拷贝 app 层资产(config/ scripts/ shaders/ fonts/ script-opts/ vs/ ...)
#   3. 拷贝 LICENSE.MD / VERSION / README.md / CATEGORIES.md + QUICKSTART.md
#      + user.example.conf 模板 + docs/tutorials(GUI 帮助页数据源,可选)
#   4. 拷贝 CLI 二进制(必需,CI 中由 cargo build --release 产出)+ GUI 二进制(可选)
#   5. Windows 附加: 捆绑 mpv 本体(shinchiro x86_64 稳定版,mpv.exe/mpv.com/
#      d3dcompiler_43.dll + MPV-BUILD.txt 出处记录;可用 MPV_PACKAGE_URL 覆盖)
#   6. 密钥扫描(不产半成品)
#   7. 打包 zip: mpv-config-<platform>-<version>.zip(version 读 VERSION 文件)
#   8. 自检:解压断言结构(无 .git/.omo/target/node_modules/user 实体,仅保留模板;
#      Windows 包断言 mpv.exe 存在)
#
# 铁律: zip 不含 user 层实体与密钥;脚本幂等(临时目录 mktemp + trap 清理)。
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [ $# -ne 2 ]; then
    echo "用法: bash tools/build-dist.sh <platform> <out-dir>" >&2
    echo "  platform: linux | windows | macos" >&2
    exit 1
fi
PLATFORM="$1"
OUT_DIR="$2"

case "$PLATFORM" in
    linux|windows|macos) ;;
    *) echo "错误: 未知平台 '$PLATFORM'(应为 linux|windows|macos)" >&2; exit 1 ;;
esac

VERSION="$(tr -d ' \r\n' < VERSION)"
[ -n "$VERSION" ] || { echo "错误: VERSION 文件为空" >&2; exit 1; }

EXE_SUFFIX=""
[ "$PLATFORM" = "windows" ] && EXE_SUFFIX=".exe"

ZIP_NAME="mpv-config-${PLATFORM}-${VERSION}.zip"
ZIP_PATH="$OUT_DIR/$ZIP_NAME"
mkdir -p "$OUT_DIR"

# ---- 临时目录(幂等 + 失败不留残余) ----
STAGE_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/mpv-config-dist.XXXXXX")"
STAGE="$STAGE_ROOT/mpv-config"
trap 'rm -rf "$STAGE_ROOT"' EXIT
mkdir -p "$STAGE"

fail() { echo "错误: $*" >&2; exit 1; }
warn() { echo "警告: $*" >&2; }

echo "== mpv-config 成品组装 [$PLATFORM] v$VERSION =="

# ---- 1. gen 生成成品配置(失败即退出,不产 zip) ----
# 输出到 portable_config/:mpv 便携模式下,mpv.exe 同级的 portable_config/
# 即配置目录,解压后的 zip 根目录本身就是"可双击播放的播放器目录"。
GEN_BIN=""
if [ -x "target/release/mpv-config$EXE_SUFFIX" ]; then
    GEN_BIN="target/release/mpv-config$EXE_SUFFIX"
    echo "gen: 使用 release 二进制 $GEN_BIN"
else
    echo "gen: 未找到 release 二进制,退回 cargo run(CI 中应先 cargo build --release)"
    GEN_BIN="cargo"
fi

if [ "$GEN_BIN" = "cargo" ]; then
    cargo run -p cli -- gen --platform "$PLATFORM" --out "$STAGE/portable_config"
else
    "$GEN_BIN" gen --platform "$PLATFORM" --out "$STAGE/portable_config"
fi

# ---- 2. app 层资产(缺 config/ 为关键错误;其余可选目录仅警告) ----
copy_dir() { # $1=源目录 $2=关键?(1/0)
    if [ -d "$1" ]; then
        cp -r "$1" "$STAGE/"
    elif [ "$2" = "1" ]; then
        fail "关键资产目录缺失: $1"
    else
        warn "可选资产目录缺失,跳过: $1"
    fi
}
copy_dir config 1
copy_dir scripts 0
copy_dir shaders 0
copy_dir fonts 0
copy_dir script-opts 0
copy_dir vs 0
copy_dir icc 0
copy_dir osc-style 0
copy_dir script-modules 0

# ---- 3. 根文档(许可继承 D12⑥:LICENSE.MD + VERSION 必须) ----
copy_file() { # $1=源 $2=关键?
    if [ -f "$1" ]; then
        cp "$1" "$STAGE/"
    elif [ "$2" = "1" ]; then
        fail "关键文件缺失: $1"
    else
        warn "可选文件缺失,跳过: $1"
    fi
}
copy_file LICENSE.MD 1
copy_file VERSION 1
copy_file README.md 0
copy_file CATEGORIES.md 0
# 一页式使用说明(关键资产:面向"解压后下一步做什么",缺它组包必须失败)
if [ -f docs/quickstart.md ]; then
    cp docs/quickstart.md "$STAGE/QUICKSTART.md"
    echo "使用说明: docs/quickstart.md → QUICKSTART.md"
else
    fail "关键文件缺失: docs/quickstart.md(zip 必须随包携带使用说明)"
fi
# GUI 帮助页数据源(可选:教程目录缺失仅警告)
if [ -d docs/tutorials ]; then
    mkdir -p "$STAGE/docs"
    cp -r docs/tutorials "$STAGE/docs/"
    echo "教程: docs/tutorials → docs/tutorials(GUI 帮助页)"
else
    warn "docs/tutorials 缺失: GUI 帮助页将为空"
fi
# linux.conf include="~~/profiles.conf" 的 include 目标(与 mpv.conf 同目录;
# gen 已拷贝,此处兜底确保存在)
if [ -f profiles.conf ]; then
    cp profiles.conf "$STAGE/portable_config/"
fi

# user 层模板(升级契约:zip 只含 user.example.conf 模板,不含任何实体)
if [ -f user/user.example.conf ]; then
    mkdir -p "$STAGE/user"
    cp user/user.example.conf "$STAGE/user/"
    echo "user 层: 仅含模板 user/user.example.conf(实体不打包)"
else
    warn "user/user.example.conf 模板缺失(gen 将写入默认头部注释)"
fi

# ---- 4. 二进制 ----
CLI_SRC="target/release/mpv-config$EXE_SUFFIX"
if [ -x "$CLI_SRC" ]; then
    cp "$CLI_SRC" "$STAGE/mpv-config$EXE_SUFFIX"
    echo "CLI 二进制: $(basename "$CLI_SRC") → mpv-config$EXE_SUFFIX"
else
    # 本机模拟/本地开发常见:未 build --release。注明明细,继续(CI 必然存在)。
    warn "CLI 二进制 $CLI_SRC 未找到 —— zip 将不含可执行文件(仅配置+资产)。CI 中 cargo build --release 后必存在"
fi

GUI_SRC="ui/src-tauri/target/release/mpv-config$EXE_SUFFIX"
if [ -x "$GUI_SRC" ]; then
    cp "$GUI_SRC" "$STAGE/mpv-config-gui$EXE_SUFFIX"
    echo "GUI 二进制(可选): → mpv-config-gui$EXE_SUFFIX"
else
    warn "GUI 二进制 $GUI_SRC 未找到 —— 跳过(可选,zip 分发以 CLI/配置为主)"
fi

# ---- 4b. mpv 本体捆绑(仅 Windows:解压即播放器,定位决策 2026-08-16) ----
if [ "$PLATFORM" = "windows" ]; then
    if [ -n "${MPV_PACKAGE_URL:-}" ]; then
        MPV_URL="$MPV_PACKAGE_URL"
        MPV_TAG="custom(MPV_PACKAGE_URL 指定)"
    else
        MPV_LATEST="$(curl -fsSL "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest")" \
            || fail "查询 shinchiro mpv 最新版本失败(离线/受限环境可用 MPV_PACKAGE_URL 指定直链)"
        MPV_TAG="$(printf '%s' "$MPV_LATEST" | grep -m1 '"tag_name"' | sed 's/.*: "\(.*\)",*/\1/')"
        MPV_URL="$(printf '%s' "$MPV_LATEST" \
            | grep -oE 'https://github\.com/shinchiro/mpv-winbuild-cmake/releases/download/[^"]*/mpv-x86_64-[0-9]+-git-[0-9a-f]+\.7z' \
            | head -1)"
        [ -n "$MPV_URL" ] || fail "shinchiro 最新 release 中未找到 mpv-x86_64 主包资产"
    fi
    MPV_EXTRACTOR=""
    for candidate in 7z 7za; do
        if command -v "$candidate" >/dev/null 2>&1; then
            MPV_EXTRACTOR="$candidate"
            break
        fi
    done
    [ -n "$MPV_EXTRACTOR" ] || fail "解压 mpv 需要 7z(CI 自带;本地请安装 7-Zip 或 NanaZip)"

    MPV_WORK="$STAGE_ROOT/mpv-pkg"
    mkdir -p "$MPV_WORK"
    echo "mpv 本体: 下载 $MPV_URL"
    curl -fsSL -o "$MPV_WORK/mpv.7z" "$MPV_URL" || fail "下载 mpv 本体失败"
    # 只取播放器必需文件:shinchiro 包中的 doc/installer/updater 不随包分发
    # (updater.bat 与本工具的 upgrade 机制冲突,安装脚本对便携形态无意义)。
    "$MPV_EXTRACTOR" x -y -o"$MPV_WORK/extract" "$MPV_WORK/mpv.7z" \
        mpv.exe mpv.com d3dcompiler_43.dll >/dev/null \
        || fail "解压 mpv 本体失败(资产格式可能已变化,请检查 shinchiro 包结构)"
    for f in mpv.exe mpv.com d3dcompiler_43.dll; do
        [ -f "$MPV_WORK/extract/$f" ] || fail "mpv 包中缺少 $f(shinchiro 包结构可能已变化)"
        cp "$MPV_WORK/extract/$f" "$STAGE/"
    done
    printf '%s\n' \
        "本包捆绑的 mpv 播放器本体" \
        "来源: $MPV_URL" \
        "版本标签: $MPV_TAG" \
        "许可: mpv 以 GPLv2+ 许可发布,源码 https://github.com/mpv-player/mpv ," \
        "构建工程 https://github.com/shinchiro/mpv-winbuild-cmake 。" \
        > "$STAGE/MPV-BUILD.txt"
    rm -rf "$MPV_WORK"
    echo "mpv 本体: mpv.exe / mpv.com / d3dcompiler_43.dll + MPV-BUILD.txt"
fi

# ---- 5. 密钥扫描(命中即失败,不产半成品 zip) ----
# 两层检查:
#   A. 高熵私有密钥模式(sk-/AIza/ghp_/AKIA/私钥块)—— 扫描全部文件,命中即失败。
#   B. 密钥赋值模式(api_token= 等)—— 只扫会"生效"的配置/代码文件(排除 .md 文档,
#      文档中的示例行是惰性文本);放行两个公开上游默认值(非个人密钥):
#         tNjXZUnOJWcHznHDyalNMYqqP6IdDdpQ  → AssrtOSS/mpv-assrt 公共 OSS token(脚本默认)
#         NmJmYjIxOTZkNzIyN2UyMTIzMGM3Y2YzZjQ4MDNkZGM= → uosc_danmaku 模板占位(base64)
echo "-- 密钥扫描 --"
SECRET_HITS="$(grep -rnIE \
    -e '(sk-[A-Za-z0-9_-]{20,})' \
    -e '(AIza[0-9A-Za-z_-]{30,})' \
    -e '(gh[pousr]_[A-Za-z0-9]{30,})' \
    -e '(AKIA[0-9A-Z]{16})' \
    -e 'BEGIN (RSA|OPENSSH|EC|PRIVATE) KEY' \
    "$STAGE" 2>/dev/null || true)"
ASSIGN_HITS="$(grep -rnIE \
    -e '^[[:space:]]*(api_token|tmdb_api_key|api_key|apikey|token)[[:space:]]*=[^[:space:]#].{16,}' \
    --include='*.conf' --include='*.lua' --include='*.js' --include='*.json' \
    --include='*.yaml' --include='*.yml' --include='*.toml' --include='*.txt' \
    "$STAGE" 2>/dev/null || true)"
ASSIGN_HITS="$(printf '%s\n' "$ASSIGN_HITS" \
    | grep -vE 'tNjXZUnOJWcHznHDyalNMYqqP6IdDdpQ|NmJmYjIxOTZkNzIyN2UyMTIzMGM3Y2YzZjQ4MDNkZGM=' || true)"
HITS="$(printf '%s\n%s\n' "$SECRET_HITS" "$ASSIGN_HITS" | grep -v '^$' || true)"
if [ -n "$HITS" ]; then
    printf '%s\n' "$HITS" >&2
    fail "检测到疑似真实密钥,拒绝打包"
fi
echo "扫描通过: 高熵私钥模式 0 命中;密钥赋值 0 命中(2 个公开上游默认值已放行)"

# ---- 6. 打包 zip(结构: zip 内顶层目录 mpv-config/;先打临时再原子落位) ----
echo "-- 打包 --"
TMP_ZIP="$STAGE_ROOT/$ZIP_NAME"
if command -v zip >/dev/null 2>&1; then
    (cd "$STAGE_ROOT" && zip -qr "$TMP_ZIP" mpv-config)
elif command -v 7z >/dev/null 2>&1; then
    (cd "$STAGE_ROOT" && 7z a -tzip "$TMP_ZIP" mpv-config >/dev/null)
elif command -v python3 >/dev/null 2>&1; then
    python3 - "$TMP_ZIP" "$STAGE_ROOT" <<'PY'
import shutil, sys
base, root = sys.argv[1][:-4], sys.argv[2]
shutil.make_archive(base, 'zip', root, 'mpv-config')
PY
else
    fail "无可用 zip 工具(zip/7z/python3 均缺失)"
fi

# ---- 7. 自检:解压 → 结构断言 ----
echo "-- 自检 --"
CHECK_DIR="$STAGE_ROOT/check"
mkdir -p "$CHECK_DIR"
if command -v unzip >/dev/null 2>&1; then
    unzip -q "$TMP_ZIP" -d "$CHECK_DIR"
elif command -v python3 >/dev/null 2>&1; then
    python3 - "$TMP_ZIP" "$CHECK_DIR" <<'PY'
import sys, zipfile
zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])
PY
else
    tar -xf "$TMP_ZIP" -C "$CHECK_DIR"
fi

C="$CHECK_DIR/mpv-config"
[ -d "$C" ] || fail "自检失败: zip 内无顶层 mpv-config/ 目录"
[ -d "$C/portable_config" ] || fail "自检失败: 缺少 portable_config/ 目录"
for f in portable_config/mpv.conf portable_config/input.conf VERSION LICENSE.MD QUICKSTART.md; do
    [ -f "$C/$f" ] || fail "自检失败: 缺少 $f"
done
if [ "$PLATFORM" = "windows" ]; then
    for f in mpv.exe mpv.com MPV-BUILD.txt; do
        [ -f "$C/$f" ] || fail "自检失败: Windows 包缺少 $f(必须捆绑 mpv 本体)"
    done
fi
for d in config scripts script-opts shaders fonts vs; do
    [ -d "$C/$d" ] || fail "自检失败: 缺少目录 $d"
done
grep -qx "$VERSION" "$C/VERSION" || fail "自检失败: zip 内 VERSION 与根 VERSION 不一致"

# 禁止物: .git/.omo/target/node_modules
FORBIDDEN="$(find "$C" -name .git -o -name .omo -o -name target -o -name node_modules | head -5)"
[ -z "$FORBIDDEN" ] || fail "自检失败: zip 内含禁止目录/文件: $FORBIDDEN"

# user 层: 只允许 user.example.conf 模板
if [ -d "$C/user" ]; then
    USER_EXTRA="$(find "$C/user" -mindepth 1 ! -name user.example.conf | head -5)"
    [ -z "$USER_EXTRA" ] || fail "自检失败: zip 的 user/ 含模板以外内容: $USER_EXTRA"
fi

# 顶层不得残留任何文件(全部内容在 mpv-config/ 下)
TOP_EXTRA="$(find "$CHECK_DIR" -mindepth 1 -maxdepth 1 ! -name mpv-config)"
[ -z "$TOP_EXTRA" ] || fail "自检失败: zip 顶层有 mpv-config/ 以外内容: $TOP_EXTRA"

echo "自检通过: 结构断言 OK(顶层 mpv-config/、portable_config/ 成品、app 层齐全$( [ "$PLATFORM" = "windows" ] && echo -n "、mpv 本体已捆绑" )、无 user 实体/无 .git/.omo/target/node_modules)"

# 自检通过后才原子落位(失败不覆盖旧 zip,不产半成品)
mv "$TMP_ZIP" "$ZIP_PATH"

SIZE="$(du -h "$ZIP_PATH" | cut -f1)"
echo ""
echo "完成: $ZIP_PATH ($SIZE)"
echo "  zip 内文件数: $(unzip -l "$ZIP_PATH" 2>/dev/null | tail -1 | awk '{print $2}' || echo '?')"
