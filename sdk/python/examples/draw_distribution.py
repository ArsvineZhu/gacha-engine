#!/usr/bin/env python3
"""读取一组 GachaRule 源文件，打印一次抽取的精确分布。"""

import sys
from pathlib import Path

from gacha_engine import Engine


def main():
    directory = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[3] / "spec/gacharule/positive"
    banner = sys.argv[2] if len(sys.argv) > 2 else "endfield_event_a"
    action = sys.argv[3] if len(sys.argv) > 3 else "draw"
    engine = Engine([str(path) for path in sorted_paths(directory)])
    session = engine.session(banner)
    if banner == "endfield_event_a":
        session.set_count("pity6", "65")
    draw = session.draw(action)
    print(f"guarantee={draw.guarantee}")
    for branch in draw.branches:
        print(f"{branch.entry}\trarity={branch.rarity}\t{branch.numer}/{branch.denom}")


def sorted_paths(directory: Path):
    paths = [path for path in directory.iterdir() if path.suffix in {".gmr", ".gml", ".gmb"}]
    rank = {".gmr": 0, ".gml": 1, ".gmb": 2}
    return sorted(paths, key=lambda path: (rank[path.suffix], path.name))


if __name__ == "__main__":
    main()
