#!/usr/bin/env python3
"""Dependency-free structural checks for the bundled reference packs.

This is not a substitute for `cargo test`; it exists because the generation
sandbox may not have a Rust toolchain.
"""

from __future__ import annotations

import json
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def die(message: str) -> None:
    raise AssertionError(message)


def unique(records: list[dict], kind: str) -> dict[str, dict]:
    result = {}
    for record in records:
        ident = record["id"]
        if not ident or ident in result:
            die(f"invalid/duplicate {kind} id: {ident!r}")
        result[ident] = record
    return result


def walk_condition(node: dict, state: dict, pools: dict) -> None:
    op = node["op"]
    if op in {"all", "any"}:
        for child in node["conditions"]:
            walk_condition(child, state, pools)
    elif op == "not":
        walk_condition(node["condition"], state, pools)
    elif op.startswith("state_"):
        if node["state"] not in state:
            die(f"condition references unknown state {node['state']}")
        if op == "state_modulo_eq":
            if node["modulus"] <= 0 or not (0 <= node["value"] < node["modulus"]):
                die("invalid modulo condition")
    elif op == "outcome_item_in_pool" and node["pool"] not in pools:
        die(f"condition references unknown pool {node['pool']}")


def check_selector_cycles(selectors: dict[str, dict]) -> None:
    visiting: set[str] = set()
    done: set[str] = set()

    def visit(ident: str) -> None:
        if ident in done:
            return
        if ident in visiting:
            die(f"selector cycle at {ident}")
        visiting.add(ident)
        node = selectors[ident]
        if node["type"] == "weighted":
            for branch in node["branches"]:
                target = branch["selector"]
                if target not in selectors:
                    die(f"selector {ident} references unknown selector {target}")
                if Fraction(branch["weight"]) <= 0:
                    die(f"selector {ident} has non-positive weight")
                visit(target)
        visiting.remove(ident)
        done.add(ident)

    for ident in selectors:
        visit(ident)


def distribution_at(dist: dict, counters: dict[str, int]) -> dict[int, Fraction]:
    values: dict[int, Fraction] = {}
    shares: list[tuple[int, int]] = []
    remainder_rarity = None
    fixed = Fraction(0)
    for entry in dist["entries"]:
        rarity = entry["rarity"]
        typ = entry["type"]
        if typ == "constant":
            value = Fraction(entry["value"])
        elif typ == "linear_after":
            current = counters[entry["state"]]
            steps = current - entry["after"] + 1 if current >= entry["after"] else 0
            value = Fraction(entry["base"]) + steps * Fraction(entry["increment"])
            value = min(value, Fraction(entry.get("cap", "1")))
        elif typ == "remainder":
            remainder_rarity = rarity
            continue
        elif typ == "share_of_remainder":
            shares.append((rarity, entry["weight"]))
            continue
        else:
            continue
        values[rarity] = value
        fixed += value
    if fixed > 1:
        die(f"distribution fixed mass > 1: {fixed}")
    rem = 1 - fixed
    if remainder_rarity is not None:
        if shares:
            die("mixed remainder and share_of_remainder")
        values[remainder_rarity] = rem
    elif shares:
        total = sum(weight for _, weight in shares)
        for rarity, weight in shares:
            values[rarity] = rem * Fraction(weight, total)
    if sum(values.values()) != 1:
        die(f"distribution does not sum to one: {values}")
    return values


def check_pack(path: Path) -> None:
    pack = json.loads(path.read_text(encoding="utf-8"))
    assert pack["schema_version"] == 1
    state = unique(pack["state"], "state")
    items = unique(pack["items"], "item")
    pools = unique(pack["pools"], "pool")
    distributions = unique(pack["distributions"], "distribution")
    selectors = unique(pack["selectors"], "selector")
    unique(pack["banners"], "banner")

    for pool in pools.values():
        if not pool["items"]:
            die(f"empty pool {pool['id']}")
        for item in pool["items"]:
            if item not in items:
                die(f"pool {pool['id']} references unknown item {item}")

    for selector in selectors.values():
        if selector["type"] == "pool" and selector["pool"] not in pools:
            die(f"selector {selector['id']} references unknown pool")
    check_selector_cycles(selectors)

    for dist in distributions.values():
        rarities = [e["rarity"] for e in dist["entries"]]
        if len(rarities) != len(set(rarities)):
            die(f"duplicate rarity in distribution {dist['id']}")
        for entry in dist["entries"]:
            if entry["type"] in {"linear_after", "table"} and entry["state"] not in state:
                die(f"distribution references unknown state {entry['state']}")

    for banner in pack["banners"]:
        for action in banner["actions"]:
            if action["distribution"] not in distributions:
                die("unknown action distribution")
            for selector in action["selectors"].values():
                if selector not in selectors:
                    die("unknown action selector")
            for guarantee in action.get("guarantees", []):
                walk_condition(guarantee["when"], state, pools)
                effect = guarantee["effect"]
                if effect["type"] == "force_item" and effect["item"] not in items:
                    die("force_item references unknown item")
            for transition in action.get("transitions", []):
                walk_condition(transition["when"], state, pools)
                for update in transition.get("updates", []):
                    if update["state"] not in state:
                        die("update references unknown state")

    # Reference-pack mathematical checkpoints.
    dist = next(iter(distributions.values()))
    p0 = distribution_at(dist, {"six_star_pity": 0})
    assert p0[6] == Fraction(8, 1000)
    assert p0[5] == Fraction(8, 100)
    assert p0[4] == Fraction(912, 1000)
    p65 = distribution_at(dist, {"six_star_pity": 65})
    assert p65[6] == Fraction(58, 1000)
    p78 = distribution_at(dist, {"six_star_pity": 78})
    assert p78[6] == Fraction(708, 1000)

    print(f"OK {path.relative_to(ROOT)}")


def main() -> int:
    for path in sorted((ROOT / "packs" / "endfield").glob("*.json")):
        check_pack(path)
    print("All dependency-free sanity checks passed.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"FAILED: {exc}", file=sys.stderr)
        raise
