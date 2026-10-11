"""有大小限制的固定版本来源读取与规范化快照输出。"""
import gzip
import hashlib
import json
from pathlib import Path
import urllib.request

LIMIT = 64 * 1024 * 1024


def fetch(source, cache):
    key = hashlib.sha256(source["url"].encode()).hexdigest()
    path = Path(cache) / key
    if path.exists():
        data = path.read_bytes()
    else:
        request = urllib.request.Request(source["url"], headers={"User-Agent": "gacha-engine-content/0.1.0"})
        with urllib.request.urlopen(request, timeout=45) as response:
            data = response.read(LIMIT + 1)
        if len(data) > LIMIT:
            raise ValueError("source exceeds 64 MiB")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    digest = hashlib.sha256(data).hexdigest()
    if source.get("sha256") and source["sha256"] != digest:
        raise ValueError(f"source hash mismatch: {source['id']}")
    source["sha256"] = digest
    if source["url"].endswith(".gzip"):
        data = gzip.decompress(data)
    return json.loads(data)


def item(game, kind, id, name, rarity, source, tags=(), names=None, native=None):
    if not name or not name.strip() or rarity <= 0:
        raise ValueError(f"invalid content item: {game}/{id}")
    return dict(id=f"{game}.{kind}.{id}", upstream_id=str(id), kind=kind,
                display_name=name, names=names or {"zh-CN": name}, rarity=rarity,
                native_rarity=native or str(rarity), tags=[kind, *tags], source_ids=[source])


def write_catalog(root, game, kind, rows):
    rows = sorted(rows, key=lambda row: row["id"])
    if len({row["id"] for row in rows}) != len(rows):
        raise ValueError(f"duplicate item IDs: {game}/{kind}")
    paths = []
    # 目录按记录分片，每片不超过 600 行。
    for index in range(0, len(rows), 400):
        part = rows[index:index + 400]
        path = Path(root) / "pools" / game / f"catalog.{kind}-{index // 400 + 1:02}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        header = dict(schema_version=1, id=f"{game}.{kind}.{index // 400 + 1}", game=game,
                      version="0.1.0", description="已核对来源的内容快照；完整目录与具体卡池的可抽范围分别维护。")
        lines = ["{"]
        lines += [f"  {json.dumps(key)}: {json.dumps(value, ensure_ascii=False)}," for key, value in header.items()]
        lines += ['  "source_documents": ["sources.json"],', '  "items": [']
        lines += ["    " + json.dumps(row, ensure_ascii=False) + ("," if i < len(part) - 1 else "") for i, row in enumerate(part)]
        lines += ["  ]", "}"]
        path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        paths.append(str(path.relative_to(root)).replace("\\", "/"))
    directory = Path(root) / "pools" / game
    retained = {Path(root) / path for path in paths}
    for previous in directory.glob(f"catalog.{kind}-[0-9][0-9].json"):
        if previous not in retained:
            if not previous.resolve().is_relative_to(directory.resolve()):
                raise ValueError("catalog shard escapes content directory")
            previous.unlink()
    return paths
