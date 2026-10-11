"""使用真实清单检查发布模式 CLI 的完整流程。"""
from collections import Counter
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target" / "release" / ("gacha.exe" if sys.platform == "win32" else "gacha")


def run(*args):
    result = subprocess.run([str(EXE), *map(str, args)], cwd=ROOT, capture_output=True, encoding="utf-8")
    if result.returncode:
        raise AssertionError(result.stderr)
    return json.loads(result.stdout)


def main():
    catalogs = sorted((ROOT / "official/gacha-games/pools").rglob("catalog.*.json"))
    counts = Counter()
    for path in catalogs:
        catalog = run("catalog", path)
        for item in catalog["items"]:
            counts[(catalog["game"], item["kind"])] += 1
    manifests = sorted((ROOT / "official/gacha-games/pools").rglob("banner.*.json"))
    actions = 0
    for path in manifests:
        manifest = json.loads(path.read_text(encoding="utf-8"))
        assert run("validate", path)["valid"]
        rules = json.loads((path.parent / manifest["rules"]).read_text(encoding="utf-8"))
        eligible = {id for pool in manifest["pools"] for id in pool["items"]}
        for banner in rules["banners"]:
            effective = manifest.get("banner_bindings", {}).get(banner["id"], banner)
            for action in banner["actions"]:
                result = run("pull", path, effective["id"], action["id"], "--count", 2, "--seed", 42)
                assert len(result["results"]) == 2 * action.get("draws", 1)
                assert all(r["item"] in eligible and r["display_name"] and r["kind"] != "other" for r in result["results"])
                assert result == run("pull", path, effective["id"], action["id"], "--count", 2, "--seed", 42)
                actions += 1
    with tempfile.TemporaryDirectory(prefix="gacha-cli-test-") as temporary:
        query = Path(temporary) / "query.json"
        query.write_text(json.dumps(dict(banner="star-rail.character", action="single_pull",
                                        target=dict(kind="tag", tag="featured"), draws=10, quantiles=[])), encoding="utf-8")
        one = run("analyze", "official/gacha-games/pools/star-rail/banner.character-reference.json", query)
        two = run("analyze", "official/gacha-games/pools/star-rail/banner.character-reference.json", query, "--copies", 2)
        assert 0 < two["result"]["probability_within"] < one["result"]["probability_within"] < 1
    print(json.dumps(dict(manifests=len(manifests), catalogs=len(catalogs), contents=sum(counts.values()),
                          action_flows=actions, reproducible_replay=True, copy_analysis=True)))


if __name__ == "__main__":
    main()
