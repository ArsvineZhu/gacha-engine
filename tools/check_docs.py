"""检查 Markdown 本地链接和标题锚点。"""
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"\[[^\]]*\]\(<?([^\s)>]+)>?(?:\s+\"[^\"]*\")?\)")


def anchors(text):
    result = set()
    counts = {}
    for heading in re.findall(r"^#{1,6}\s+(.+)$", text, re.MULTILINE):
        slug = re.sub(r"[^\w\-\s]", "", heading.strip().lower()).replace(" ", "-")
        suffix = counts.get(slug, 0)
        counts[slug] = suffix + 1
        result.add(slug + (f"-{suffix}" if suffix else ""))
    return result


def main():
    config = tomllib.loads((ROOT / ".config/scripts/doc-navigation.toml").read_text(encoding="utf-8"))
    files = set()
    for entry in config["roots"]:
        path = ROOT / entry
        files.update(path.rglob("*.md") if path.is_dir() else [path])
    failures = []
    checked = 0
    for path in sorted(files):
        text = re.sub(r"```.*?```", "", path.read_text(encoding="utf-8"), flags=re.DOTALL)
        for link in LINK.findall(text):
            url = urlsplit(link)
            if url.scheme in config["external_schemes"]:
                continue
            checked += 1
            target = (path.parent / unquote(url.path)).resolve() if url.path else path
            if config["check_local_targets"] and not target.exists():
                failures.append(f"{path.relative_to(ROOT)}: 链接目标不存在 {link}")
            elif config["check_anchors"] and url.fragment and target.suffix == ".md":
                if unquote(url.fragment) not in anchors(target.read_text(encoding="utf-8")):
                    failures.append(f"{path.relative_to(ROOT)}: 标题锚点不存在 {link}")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print(f"文档链接检查通过：{len(files)} 份文档，{checked} 个本地链接")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
