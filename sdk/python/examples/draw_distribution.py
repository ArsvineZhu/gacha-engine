#!/usr/bin/env python3
"""读取一组 GachaRule 源文件，打印一次抽取的精确分布。"""

import sys
from pathlib import Path

import gacha_engine as g


def main():
    directory = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parents[3] / "spec/gacharule/positive"
    banner = sys.argv[2] if len(sys.argv) > 2 else "endfield_event_a"
    action = sys.argv[3] if len(sys.argv) > 3 else "draw"
    engine = g.gacha_engine_load([str(path) for path in sorted_paths(directory)])
    session = g.gacha_engine_session(engine)
    if banner == "endfield_event_a":
        g.gacha_session_set_count(session, banner, "pity6", "65")
    draw = g.gacha_session_draw(session, banner, action)
    print(f"guarantee={draw.guarantee}")
    for branch in draw.branches:
        print(f"{branch.entry}\trarity={branch.rarity}\t{branch.numer}/{branch.denom}")


def sorted_paths(directory: Path):
    paths = [path for path in directory.iterdir() if path.suffix in {".gmr", ".gml", ".gmb"}]
    rank = {".gmr": 0, ".gml": 1, ".gmb": 2}
    return sorted(paths, key=lambda path: (rank[path.suffix], path.name))


if __name__ == "__main__":
    main()
