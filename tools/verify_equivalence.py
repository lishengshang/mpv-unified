#!/usr/bin/env python3
"""Semantic-equivalence verifier for the layered mpv-config split.

Merges `config/base.conf` (evaluating `#@if platform==...` conditionals) with
the platform layer, then compares the resulting ACTIVE option set (top level
and inside every profile) against a reference config. Every difference is
printed; differences matching the whitelist are tolerated.

Supported platforms: linux, windows, macos.
Reference defaults: linux -> ~/.config/mpv/mpv.conf (read-only, never touched),
windows/macos -> archive/mpv.conf.orig.

Exit codes:
  0  generated option set equals the reference (or only whitelisted diffs)
  1  real (non-whitelisted) differences found
  2  usage / I/O error

Whitelist files: docs/equivalence-whitelist-<platform>.txt — one token per
line, `#`-prefixed lines are comments. Tokens:
  `top:KEY`              tolerate any diff of top-level option KEY
  `profile:NAME`         tolerate the whole profile block NAME
  `profile:NAME:KEY`     tolerate one option inside profile NAME
  `bind:KEY`             (input.conf) tolerate a binding of input KEY
  `*`                    tolerate everything (not recommended)
  `line:<exact text>`    tolerate one exact report line
"""
import re
import sys
from collections import OrderedDict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DIRECTIVES = ("#@if", "#@else", "#@endif")


def log(msg=""):
    print(msg)


def parse_entries(lines, input_mode=False):
    """Yield (scope, key, value) for ACTIVE lines.
    input_mode: scope=None, key=input-key, value=command."""
    cur = None
    for raw in lines:
        s = raw.strip()
        if not s or s.startswith("#") or s.startswith("#@"):
            continue
        m = re.match(r"^\[([^\]]+)\]$", s)
        if m:
            cur = m.group(1)
            continue
        m = re.match(r"^([A-Za-z0-9_\-\.]+)(?:=(.*))?$", s)
        if not m:
            continue
        key = m.group(1)
        val = (m.group(2) or "").strip()
        val = re.split(r"\s+#", val)[0].strip()
        yield cur, key, val


def parse_bindings(lines):
    """input.conf active bindings: key -> command (command = rest of line)."""
    for raw in lines:
        s = raw.rstrip("\n")
        stripped = s.strip()
        if not stripped or stripped.startswith("#") or stripped.startswith("#@"):
            continue
        m = re.match(r"^(\S+)\s+(.*)$", stripped)
        if not m:
            continue
        key, cmd = m.group(1), m.group(2)
        cmd = re.split(r"\s+#", cmd)[0].rstrip()
        yield None, key, cmd


def eval_conds(lines, platform):
    out, stack = [], []
    for line in lines:
        s = line.strip()
        if s == "#@if platform==windows":
            stack.append(platform == "windows")
        elif s == "#@else":
            stack[-1] = not stack[-1]
        elif s == "#@endif":
            stack.pop()
        elif all(stack):
            out.append(line)
    return out


def load_layer(path, platform=None, input_mode=False, bindings=False):
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    if platform:
        lines = eval_conds(lines, platform)
    if bindings:
        return list(parse_bindings(lines))
    return list(parse_entries(lines, input_mode=input_mode))


def merge(layers):
    merged = OrderedDict()
    for layer in layers:
        for scope, key, val in layer:
            merged[(scope, key)] = val
    return merged


def extract_options(path, bindings=False):
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    if bindings:
        return merge([list(parse_bindings(lines))])
    return merge([list(parse_entries(lines))])


def read_whitelist(path):
    if not path.exists():
        return []
    tokens = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        tokens.append(line)
    return tokens


def render(scope, key):
    if scope is None:
        return f"top:{key}"
    return f"profile:{scope}:{key}"


def compare(gen, ref):
    """Return list of report lines (scope,key,kind,detail)."""
    diffs = []
    for (scope, key), val in ref.items():
        g = gen.get((scope, key))
        if g is None:
            diffs.append((scope, key, "MISSING", f"reference defines {render(scope,key)}={val} but generated does not"))
        elif g != val:
            diffs.append((scope, key, "VALUE", f"{render(scope,key)}: generated={g} reference={val}"))
    for (scope, key), val in gen.items():
        if (scope, key) not in ref:
            diffs.append((scope, key, "EXTRA", f"{render(scope,key)}={val} is generated but not in reference"))
    return diffs


def is_whitelisted(diff, tokens):
    scope, key, kind, detail = diff
    tk = render(scope, key)
    for t in tokens:
        if t == "*":
            return True
        if t == tk:
            return True
        if t.startswith("profile:") and t.count(":") == 1 and scope == t[8:]:
            return True
        if t.startswith("line:") and detail == t[5:]:
            return True
    return False


def comment_stats(path):
    """(total_lines, comment_lines, directive_lines)"""
    total = comments = directives = 0
    with open(path, encoding="utf-8") as f:
        for raw in f:
            total += 1
            s = raw.strip()
            if s.startswith(DIRECTIVES):
                directives += 1
            elif s.startswith("#"):
                comments += 1
    return total, comments, directives


def audit_comments():
    """Comment-preservation audit: every comment in the sources must exist in
    the products (per platform). Returns (ok, lines)."""
    src_win = REPO / "archive" / "mpv.conf.orig"
    src_lin = Path.home() / ".config" / "mpv" / "mpv.conf"
    out = []
    ok = True
    for src, products, label in (
        (src_win, ["base.conf", "windows.conf"], "windows-original"),
        (src_lin, ["base.conf", "linux.conf"], "linux-port"),
    ):
        if not src.exists():
            continue
        src_comments = set()
        for raw in src.read_text(encoding="utf-8").splitlines():
            s = raw.strip()
            if s.startswith("#") and not s.startswith(DIRECTIVES):
                src_comments.add(s)
        prod_text = ""
        for p in products:
            prod_text += (REPO / "config" / p).read_text(encoding="utf-8")
        prod_comments = set()
        for s in prod_text.splitlines():
            s = s.strip()
            if s.startswith("#") and not s.startswith(DIRECTIVES):
                prod_comments.add(s)
        missing = src_comments - prod_comments
        out.append(
            f"{label}: {len(src_comments)} unique comment lines in source, "
            f"{len(prod_comments)} in products, {len(missing)} missing"
        )
        if missing:
            ok = False
            for m in sorted(missing)[:10]:
                out.append(f"  MISSING COMMENT: {m}")
    return ok, out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        log("usage: verify-equivalence.sh <linux|windows|macos> [--ref PATH] [--input-ref PATH] [--whitelist PATH] [--self-test]")
        return 2
    platform = args[0]
    if platform not in ("linux", "windows", "macos"):
        log(f"unknown platform: {platform}")
        return 2

    ref_path = None
    input_ref = None
    whitelist_path = None
    self_test = False
    it = iter(sys.argv[1:])
    for a in it:
        if a == "--ref":
            ref_path = next(it)
        elif a == "--input-ref":
            input_ref = next(it)
        elif a == "--whitelist":
            whitelist_path = next(it)
        elif a == "--self-test":
            self_test = True

    base = REPO / "config" / "base.conf"
    plat = REPO / "config" / f"{platform}.conf"
    base_in = REPO / "config" / "input.conf"
    plat_in = REPO / "config" / f"{platform}.input.conf"

    if ref_path is None:
        if platform == "linux":
            ref_path = str(Path.home() / ".config" / "mpv" / "mpv.conf")
        else:
            ref_path = str(REPO / "archive" / "mpv.conf.orig")
    if input_ref is None:
        if platform == "linux":
            input_ref = str(Path.home() / ".config" / "mpv" / "input.conf")
        else:
            input_ref = str(REPO / "archive" / "input.conf.orig")
    if whitelist_path is None:
        whitelist_path = str(REPO / "docs" / f"equivalence-whitelist-{platform}.txt")

    if not base.exists() or not plat.exists():
        log(f"missing layer file: {base} or {plat}")
        return 2

    if self_test:
        return self_test_mode(platform, ref_path, input_ref)

    tokens = read_whitelist(Path(whitelist_path))

    gen = merge([load_layer(base, platform), load_layer(plat)])
    ref = extract_options(ref_path)
    diffs = compare(gen, ref)

    gen_in = merge([load_layer(base_in, platform, bindings=True), load_layer(plat_in, bindings=True)])
    ref_in = extract_options(input_ref, bindings=True)
    diffs_in = compare(gen_in, ref_in)

    log(f"== mpv.conf equivalence ({platform}) ==")
    log(f"   generated options: {len(gen)} | reference ({ref_path}): {len(ref)}")
    non_wl = []
    for d in diffs:
        flag = "" if is_whitelisted(d, tokens) else "  [NOT-WHITELISTED]"
        if flag:
            non_wl.append(d)
        log(f"   {d[2]:7s} {d[3]}{flag}")
    log(f"== input.conf equivalence ({platform}) ==")
    log(f"   generated bindings: {len(gen_in)} | reference ({input_ref}): {len(ref_in)}")
    for d in diffs_in:
        flag = "" if is_whitelisted(d, tokens) else "  [NOT-WHITELISTED]"
        if flag:
            non_wl.append(d)
        log(f"   {d[2]:7s} {d[3]}{flag}")

    ok, audit = audit_comments()
    log("== comment-preservation audit ==")
    for line in audit:
        log("   " + line)

    if non_wl:
        log(f"RESULT: FAIL ({len(non_wl)} non-whitelisted differences; whitelist: {whitelist_path})")
        return 1
    log(f"RESULT: PASS (differences: {len(diffs)} mpv.conf, {len(diffs_in)} input.conf — all whitelisted or none)")
    return 0


def self_test_mode(platform, ref_path, input_ref):
    """Prove the verifier catches option loss: remove one option from a
    throwaway copy of the layer set and expect a MISSING report + exit 1."""
    import tempfile

    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        base = REPO / "config" / "base.conf"
        plat = REPO / "config" / f"{platform}.conf"
        b = td / "base.conf"
        p = td / f"{platform}.conf"
        b.write_text(base.read_text(encoding="utf-8"), encoding="utf-8")
        p.write_text(plat.read_text(encoding="utf-8"), encoding="utf-8")
        # delete the first active option line in base (keep comments/headers)
        lines = b.read_text(encoding="utf-8").splitlines(keepends=True)
        removed = None
        for i, ln in enumerate(lines):
            s = ln.strip()
            if s and not s.startswith("#") and not s.startswith("#@") and not s.startswith("["):
                removed = s.split("=")[0].strip()
                del lines[i]
                break
        if removed is None:
            log("self-test: no removable option found")
            return 2
        b.write_text("".join(lines), encoding="utf-8")

        gen = merge([load_layer(b, platform), load_layer(p)])
        ref = extract_options(ref_path)
        diffs = compare(gen, ref)
        missing = [d for d in diffs if d[2] == "MISSING"]
        log(f"self-test: removed option `{removed}` from a throwaway base.conf copy")
        if missing:
            log(f"self-test: verifier correctly reports {len(missing)} MISSING option(s), e.g.:")
            log(f"   {missing[0][3]}")
            return 1 if missing else 0
        log("self-test FAILED: deleting an option produced no MISSING report")
        return 0


if __name__ == "__main__":
    sys.exit(main())
