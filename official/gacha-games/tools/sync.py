"""导入固定来源事实，仅更新内容目录。"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import importlib
import json
from pathlib import Path
import tempfile

from source_io import fetch, write_catalog


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache", type=Path, default=Path(tempfile.gettempdir()) / "gacha-engine-content")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    registry_path = Path(__file__).with_name("sources.json")
    registry = json.loads(registry_path.read_text(encoding="utf-8"))
    for game, config in registry.items():
        sources = {s["id"]: s for s in json.loads((root / "pools" / game / "sources.json").read_text(encoding="utf-8"))}
        config["files"] = {key: sources[id] for key, id in config["files"].items()}
    tasks = [(game, key, source) for game, config in registry.items() if not config.get("manual")
             for key, source in config["files"].items()]
    with ThreadPoolExecutor(max_workers=8) as pool:
        downloaded = list(pool.map(lambda task: fetch(task[2], args.cache), tasks))
    data = {game: {} for game in registry}
    for (game, key, _), payload in zip(tasks, downloaded):
        data[game][key] = payload
    for game, config in registry.items():
        if config.get("manual"):
            print(game, "来源与客户端字段人工交叉核对，保留规范内容目录")
            continue
        module = importlib.import_module("games." + config["adapter"])
        sources = config["files"]
        extracted = module.extract(data[game], sources)
        for kind, rows in extracted.items():
            if not rows:
                raise ValueError(f"empty extraction: {game}/{kind}")
            write_catalog(root, game, kind, rows)
        print(game, {kind: len(rows) for kind, rows in extracted.items()})


if __name__ == "__main__":
    main()
