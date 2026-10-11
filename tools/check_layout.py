"""按集中配置检查文件长度及测试代码归属。"""
from pathlib import Path
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CONFIG = tomllib.loads((ROOT / ".config/scripts/code-length.toml").read_text(encoding="utf-8"))
TEXT = set(CONFIG["extensions"])
SKIP = set(CONFIG["ignored_directories"])


def main():
    failures = []
    checked = 0
    maximum = (0, "")
    for path in ROOT.rglob("*"):
        relative = path.relative_to(ROOT)
        if not path.is_file() or SKIP.intersection(relative.parts):
            continue
        if path.suffix not in TEXT and path.name not in CONFIG["names"]:
            continue
        text = path.read_text(encoding="utf-8-sig")
        count = len(text.splitlines())
        checked += 1
        maximum = max(maximum, (count, str(relative)))
        if count > CONFIG["max_lines"]:
            failures.append(f"{relative}: {count} 行，限制 {CONFIG['max_lines']}")
        if CONFIG["reject_tests_in_src"] and path.suffix == ".rs" and "crates" in relative.parts and "src" in relative.parts:
            if "#[test]" in text or "#[cfg(test)]" in text:
                failures.append(f"{relative}: 测试代码应放在 tests/ 中")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"目录检查通过：{checked} 个文本文件；最长 {maximum[0]} 行（{maximum[1]}）")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
