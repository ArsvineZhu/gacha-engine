#!/usr/bin/env python3
"""把 origin/phase2 的官方规则和名册写成 GachaRule。

输入在 official/source/<游戏>/，输出在 official/games/<游戏>/。
JSON 只作为迁移输入，引擎不读取它。
"""

from __future__ import annotations

import json
import math
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "official" / "source"
OUTPUT = ROOT / "official" / "games"

KEYWORDS = {
    "and",
    "banner",
    "bind",
    "choose",
    "emit",
    "entry",
    "false",
    "floor",
    "from",
    "grade",
    "guarantee",
    "in",
    "item",
    "linear",
    "not",
    "or",
    "pity_key",
    "progress_key",
    "promote",
    "rarity",
    "reset",
    "rest",
    "result",
    "roster",
    "rule",
    "rescale",
    "source_roster",
    "source_rule",
    "true",
    "update",
    "weighted",
}

SCOPES = {
    "pity_group": ("pity", "@pity"),
    "progress_group": ("progress", "@progress"),
    "banner": ("banner", "@banner"),
    "batch": ("action", "@action"),
}


def encode(raw: str) -> str:
    parts: list[str] = []
    for char in str(raw).lower():
        if char.isascii() and char.isalnum():
            parts.append(char)
        elif char in "._-":
            parts.append("_")
        else:
            parts.append(f"_u{ord(char):x}_")
    name = re.sub(r"_+", "_", "".join(parts)).strip("_")
    if not name or not name[0].isalpha():
        name = f"id_{name}" if name else "id"
    if name in KEYWORDS:
        name = f"{name}_name"
    return name


def esc(text: str) -> str:
    return (
        text.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
    )


def gcd(a: int, b: int) -> int:
    return math.gcd(a, b)


def parse_number(value) -> tuple[int, int]:
    if isinstance(value, bool):
        raise ValueError(f"不是数值: {value!r}")
    if isinstance(value, int):
        return value, 1
    text = str(value).strip()
    if "/" in text:
        numer, denom = text.split("/", 1)
        return int(numer), int(denom)
    if "." in text:
        whole, frac = text.split(".", 1)
        if not frac or not (whole or "0").lstrip("-").isdigit() or not frac.isdigit():
            raise ValueError(f"不是十进制数: {text}")
        denom = 10 ** len(frac)
        numer = int(whole or "0") * denom + int(frac)
        return numer, denom
    return int(text), 1


def token(value, suffix: str) -> str:
    numer, denom = parse_number(value)
    if denom <= 0 or numer < 0:
        raise ValueError(f"非法数值 {value}")
    common = gcd(numer, denom)
    numer //= common
    denom //= common
    if suffix == "p" and numer > denom:
        raise ValueError(f"概率超过 1: {value}")
    if suffix == "w" and numer == 0:
        raise ValueError("权重不能为 0")
    if denom == 1:
        return f"{numer}{suffix}"
    return f"{numer}/{denom}{suffix}"


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def display_of(item: dict) -> str:
    names = item.get("names") or {}
    return str(names.get("zh-CN") or item.get("display_name") or item["id"])


def tags_of(item: dict) -> list[str]:
    tags = item.get("tags") or []
    return [str(tag) for tag in tags]


class Game:
    def __init__(self, directory: Path):
        self.directory = directory
        self.rules = {path.name: load_json(path) for path in sorted(directory.glob("rule.*.json"))}
        self.banners = [load_json(path) for path in sorted(directory.glob("banner.*.json"))]
        self.items: dict[str, dict] = {}
        for path in sorted(directory.glob("catalog.*.json")):
            for item in load_json(path).get("items", []):
                ident = encode(item["id"])
                previous = self.items.get(ident)
                rarity = int(item["rarity"])
                if rarity < 1:
                    raise SystemExit(f"{path}: {item['id']} 稀有度 {rarity}")
                if previous and previous["id"] != item["id"]:
                    raise SystemExit(f"{item['id']} 与 {previous['id']} 编码冲突")
                if previous and previous["rarity"] != rarity:
                    raise SystemExit(f"{item['id']} 稀有度冲突")
                tags = tags_of(item)
                if previous:
                    merged = list(dict.fromkeys(previous["tags"] + tags))
                    previous["tags"] = merged
                else:
                    self.items[ident] = {
                        "id": item["id"],
                        "ident": ident,
                        "rarity": rarity,
                        "display": display_of(item),
                        "tags": tags,
                        "drawable": bool(item.get("drawable", True)),
                    }
        self.by_raw = {item["id"]: item for item in self.items.values()}

    def entry(self, raw: str) -> dict:
        item = self.by_raw.get(raw)
        if item is None:
            raise SystemExit(f"{self.directory.name}: 名册缺少 {raw}")
        return item


def referenced_states(action: dict, selectors: dict[str, dict]) -> set[str]:
    found: set[str] = set()

    def walk_cond(node):
        if not isinstance(node, dict):
            return
        op = node.get("op")
        if op in {"state_counter_gte", "state_counter_eq", "state_bool_eq", "state_modulo_eq"}:
            found.add(node["state"])
        for key in ("condition",):
            if key in node:
                walk_cond(node[key])
        for key in ("conditions",):
            for child in node.get(key, []):
                walk_cond(child)

    def walk_selector(name: str, seen: set[str]):
        if name in seen:
            return
        seen.add(name)
        selector = selectors[name]
        kind = selector["type"]
        if kind == "weighted":
            for branch in selector["branches"]:
                walk_selector(branch["selector"], seen)
        elif kind == "conditional":
            walk_cond(selector["when"])
            walk_selector(selector["then_selector"], seen)
            walk_selector(selector["else_selector"], seen)

    for entry in action_distribution(action)["entries"]:
        if entry["type"] == "linear_after":
            found.add(entry["state"])
        walk_selector(entry_selector(action, entry), set())
    for guarantee in action.get("guarantees", []):
        walk_cond(guarantee["when"])
    for transition in action.get("transitions", []):
        walk_cond(transition.get("when"))
        for update in transition.get("updates") or []:
            found.add(update["state"])
    return found


def action_distribution(action: dict) -> dict:
    return action["_distribution"]


def entry_selector(action: dict, entry: dict) -> str:
    return action["selectors"][str(entry["rarity"])]


def prepare(rule: dict) -> None:
    distributions = {item["id"]: item for item in rule["distributions"]}
    for banner in rule["banners"]:
        for action in banner["actions"]:
            action["_distribution"] = distributions[action["distribution"]]


def condition(node, state_name) -> str:
    op = node["op"]
    if op == "always":
        return "true"
    if op == "not":
        return f"not ({condition(node['condition'], state_name)})"
    if op == "all":
        parts = [f"({condition(child, state_name)})" for child in node["conditions"]]
        if not parts:
            raise ValueError("空 and")
        return " and ".join(parts)
    if op == "any":
        parts = [f"({condition(child, state_name)})" for child in node["conditions"]]
        if not parts:
            raise ValueError("空 or")
        return " or ".join(parts)
    if op == "state_counter_gte":
        return f"{state_name(node['state'])} >= {int(node['value'])}c"
    if op == "state_counter_eq":
        return f"{state_name(node['state'])} == {int(node['value'])}c"
    if op == "state_bool_eq":
        value = "true" if node["value"] else "false"
        return f"{state_name(node['state'])} == {value}"
    if op == "state_modulo_eq":
        return (
            f"{state_name(node['state'])} % {int(node['modulus'])}c "
            f"== {int(node['value'])}c"
        )
    if op == "outcome_rarity_eq":
        return f"result.grade == {int(node['rarity'])}r"
    if op == "outcome_rarity_gte":
        return f"result.grade >= {int(node['rarity'])}r"
    if op == "outcome_item_in_pool":
        return f"result in {state_name.pool(node['pool'])}"
    raise ValueError(f"未知条件 {op}")


class Symbols:
    def __init__(self):
        self.used: set[str] = set()
        self.states: dict[str, str] = {}
        self.pools: dict[str, str] = {}
        self.selectors: dict[str, str] = {}
        self.slots: dict[str, str] = {}
        self.events: dict[str, str] = {}

    def claim(self, table: dict[str, str], raw: str, prefix: str) -> str:
        if raw in table:
            return table[raw]
        base = encode(raw) if not prefix else f"{prefix}_{encode(raw)}"
        name = base
        index = 2
        while name in self.used:
            name = f"{base}_{index}"
            index += 1
        self.used.add(name)
        table[raw] = name
        return name

    def state(self, raw: str) -> str:
        return self.claim(self.states, raw, "")

    def pool(self, raw: str) -> str:
        return self.claim(self.pools, raw, "pool")

    def selector(self, raw: str) -> str:
        return self.claim(self.selectors, raw, "sel")

    def slot(self, raw: str) -> str:
        return self.claim(self.slots, raw, "slot")

    def event(self, raw: str) -> str:
        return self.claim(self.events, raw, "ev")


def selector_order(selectors: dict[str, dict], roots: list[str]) -> list[str]:
    needed: set[str] = set()

    def visit(name: str):
        if name in needed:
            return
        needed.add(name)
        selector = selectors[name]
        if selector["type"] == "weighted":
            for branch in selector["branches"]:
                visit(branch["selector"])
        elif selector["type"] == "conditional":
            visit(selector["then_selector"])
            visit(selector["else_selector"])

    for root in roots:
        visit(root)
    incoming = {name: 0 for name in needed}
    children: dict[str, list[str]] = {name: [] for name in needed}

    def link(parent: str, child: str):
        children[child].append(parent)
        incoming[parent] += 1

    for name in needed:
        selector = selectors[name]
        if selector["type"] == "weighted":
            for branch in selector["branches"]:
                link(name, branch["selector"])
        elif selector["type"] == "conditional":
            link(name, selector["then_selector"])
            link(name, selector["else_selector"])
    ready = sorted(name for name, count in incoming.items() if count == 0)
    ordered = []
    while ready:
        name = ready.pop(0)
        ordered.append(name)
        for parent in children[name]:
            incoming[parent] -= 1
            if incoming[parent] == 0:
                ready.append(parent)
                ready.sort()
    if len(ordered) != len(needed):
        raise SystemExit(f"选择器有环: {sorted(needed - set(ordered))}")
    return ordered


def assign_rarity(selectors: dict[str, dict], roots: list[tuple[str, int]]) -> dict[str, int]:
    rarity: dict[str, int] = {}

    def give(name: str, value: int):
        previous = rarity.get(name)
        if previous is not None and previous != value:
            raise SystemExit(f"选择器 {name} 同时用于 {previous}r 和 {value}r")
        if previous == value:
            return
        rarity[name] = value
        selector = selectors[name]
        if selector["type"] == "weighted":
            for branch in selector["branches"]:
                give(branch["selector"], value)
        elif selector["type"] == "conditional":
            give(selector["then_selector"], value)
            give(selector["else_selector"], value)

    for name, value in roots:
        give(name, value)
    return rarity


def effect_text(effect: dict, symbols: Symbols) -> str:
    kind = effect["type"]
    if kind == "force_item":
        return f"item {symbols.slot(effect['item'])}"
    if kind == "force_rarity":
        return f"rarity {int(effect['rarity'])}r"
    if kind == "min_rarity":
        word = {
            "preserve_higher_fill_floor": "promote",
            "conditional_current": "rescale",
        }[effect["strategy"]]
        return f"floor {int(effect['rarity'])}r {word}"
    raise SystemExit(f"未知保底效果 {kind}")


def emit_rule(game: Game, rule: dict, action: dict, symbols: Symbols) -> str:
    states = {item["id"]: item for item in rule["state"]}
    selectors = {item["id"]: item for item in rule["selectors"]}
    distribution = action["_distribution"]
    roots = [(entry_selector(action, entry), int(entry["rarity"])) for entry in distribution["entries"]]
    order = selector_order(selectors, [name for name, _ in roots])
    rarities = assign_rarity(selectors, roots)
    used_states = referenced_states(action, selectors)
    lines = [
        f"# 源规则 {rule['id']} 动作 {action['id']}",
        f"rule {symbols.rule_name}:",
    ]
    has_pity = False
    has_progress = False
    has_action = False
    for state_id in sorted(used_states, key=lambda item: symbols.state(item)):
        state = states[state_id]
        scope_key, scope = SCOPES[state["scope"]]
        has_pity = has_pity or scope_key == "pity"
        has_progress = has_progress or scope_key == "progress"
        has_action = has_action or scope_key == "action"
        name = symbols.state(state_id)
        initial = int(state.get("initial", 0))
        if state["kind"] == "counter":
            upper = state.get("max")
            bounds = "[]" if upper is None else f"[{int(upper)}c]"
            lines.append(f"    {name}: Counter{bounds} {scope} = {initial}c")
        elif state["kind"] == "boolean":
            value = "true" if state.get("initial") else "false"
            lines.append(f"    {name}: Bool {scope} = {value}")
        else:
            raise SystemExit(f"未知状态类型 {state['kind']}")
    symbols.has_pity = has_pity
    symbols.has_progress = has_progress
    symbols.has_action = has_action

    pools: list[tuple[str, int]] = []
    for name in order:
        selector = selectors[name]
        if selector["type"] == "pool":
            pools.append((selector["pool"], rarities[name]))
    for raw in pools_in(action):
        if any(name == raw for name, _ in pools):
            continue
        pools.append((raw, pool_rarity_from_banners(game, rule, raw)))
    seen_pools = set()
    for raw, rarity in pools:
        if raw in seen_pools:
            continue
        seen_pools.add(raw)
        lines.append(f"    {symbols.pool(raw)}: Pool[{rarity}r]")

    for guarantee in action.get("guarantees", []):
        if guarantee["effect"]["type"] == "force_item":
            role = guarantee["effect"]["item"]
            lines.append(f"    {symbols.slot(role)}: Item[0r]")
    # Item rarity is filled after we know the bound item. Placeholder replaced per banner
    # is illegal: the rule declares the rarity. Use the first banner that binds it.
    slot_lines = []
    for guarantee in action.get("guarantees", []):
        if guarantee["effect"]["type"] == "force_item":
            role = guarantee["effect"]["item"]
            rarity = slot_rarity(game, rule, role)
            slot_lines.append((symbols.slot(role), rarity))
    lines = [line for line in lines if not line.endswith(": Item[0r]")]
    for name, rarity in slot_lines:
        lines.append(f"    {name}: Item[{rarity}r]")
        symbols.slot_rarity[name] = rarity

    event_names = []
    for transition in action.get("transitions", []):
        for event in transition.get("events") or []:
            key = event_key(event, None)
            event_names.append(key)
    for key in dict.fromkeys(event_names):
        lines.append(f"    {symbols.event(key)}: Event[Count]")

    def cond(node):
        return condition(node, symbols.state) if False else render_cond(node, symbols)

    for name in order:
        selector = selectors[name]
        rarity = rarities[name]
        ident = symbols.selector(name)
        kind = selector["type"]
        if kind == "pool":
            lines.append(f"    {ident}: Selector[{rarity}r] = from {symbols.pool(selector['pool'])}")
        elif kind == "weighted":
            lines.append(f"    {ident}: Selector[{rarity}r] = weighted:")
            for branch in selector["branches"]:
                lines.append(
                    f"        {token(branch['weight'], 'w')}: {symbols.selector(branch['selector'])}"
                )
        elif kind == "conditional":
            lines.append(
                f"    {ident}: Selector[{rarity}r] = choose({render_cond(selector['when'], symbols)}, "
                f"{symbols.selector(selector['then_selector'])}, "
                f"{symbols.selector(selector['else_selector'])})"
            )
        else:
            raise SystemExit(f"未知选择器 {kind}")

    lines.append(f"    {symbols.distribution}: Distribution:")
    for entry in distribution["entries"]:
        rarity = int(entry["rarity"])
        amount = amount_text(entry, symbols)
        lines.append(f"        {rarity}r: {amount} -> {symbols.selector(entry_selector(action, entry))}")

    draws = int(action.get("draws") or 1)
    if has_action:
        span = f"{draws}t..{draws}t"
    else:
        span = "1t..1000t"
    lines.append(f"    {symbols.action}: Action[{span}] = {symbols.distribution}")

    guarantees = sorted(
        enumerate(action.get("guarantees", [])),
        key=lambda item: (-int(item[1].get("priority", 0)), item[0]),
    )
    for _, guarantee in guarantees:
        lines.append(
            f"    guarantee {encode(guarantee['id'])}: {render_cond(guarantee['when'], symbols)} "
            f"-> {effect_text(guarantee['effect'], symbols)}"
        )

    for transition in action.get("transitions", []):
        mutations = mutation_lines(transition, symbols)
        if not mutations:
            continue
        lines.append(f"    update {render_cond(transition['when'], symbols)}:")
        lines.extend(f"        {line}" for line in mutations)
    lines.append("")
    return "\n".join(lines)


def render_cond(node, symbols: Symbols) -> str:
    op = node["op"]
    if op == "always":
        return "true"
    if op == "not":
        return f"not ({render_cond(node['condition'], symbols)})"
    if op == "all":
        return " and ".join(f"({render_cond(child, symbols)})" for child in node["conditions"])
    if op == "any":
        return " or ".join(f"({render_cond(child, symbols)})" for child in node["conditions"])
    if op == "state_counter_gte":
        return f"{symbols.state(node['state'])} >= {int(node['value'])}c"
    if op == "state_counter_eq":
        return f"{symbols.state(node['state'])} == {int(node['value'])}c"
    if op == "state_bool_eq":
        value = "true" if node["value"] else "false"
        return f"{symbols.state(node['state'])} == {value}"
    if op == "state_modulo_eq":
        return (
            f"{symbols.state(node['state'])} % {int(node['modulus'])}c == {int(node['value'])}c"
        )
    if op == "outcome_rarity_eq":
        return f"result.grade == {int(node['rarity'])}r"
    if op == "outcome_rarity_gte":
        return f"result.grade >= {int(node['rarity'])}r"
    if op == "outcome_item_in_pool":
        return f"result in {symbols.pool(node['pool'])}"
    raise SystemExit(f"未知条件 {op}")


def amount_text(entry: dict, symbols: Symbols) -> str:
    kind = entry["type"]
    if kind == "constant":
        return token(entry["value"], "p")
    if kind == "remainder":
        return "rest"
    if kind == "share_of_remainder":
        return token(entry["weight"], "w")
    if kind == "linear_after":
        return (
            f"linear({symbols.state(entry['state'])}, {int(entry['after'])}c, "
            f"{token(entry['base'], 'p')}, {token(entry['increment'], 'p')}, "
            f"{token(entry['cap'], 'p')})"
        )
    raise SystemExit(f"未知概率 {kind}")


def event_key(event: dict, bindings: dict | None) -> str:
    key = str(event.get("key") or event.get("kind") or "event")
    if bindings and key in bindings:
        key = bindings[key]
    return key


def mutation_lines(transition: dict, symbols: Symbols, bindings: dict | None = None) -> list[str]:
    lines = []
    for update in transition.get("updates") or []:
        kind = update["type"]
        if kind == "increment":
            lines.append(f"{symbols.state(update['state'])} += {int(update.get('by', 1))}c")
        elif kind == "reset":
            lines.append(f"reset {symbols.state(update['state'])}")
        elif kind == "set_bool":
            value = "true" if update["value"] else "false"
            lines.append(f"{symbols.state(update['state'])} = {value}")
        else:
            raise SystemExit(f"未知更新 {kind}")
    for event in transition.get("events") or []:
        amount = int(event.get("amount", 1))
        lines.append(f"emit {symbols.event(event_key(event, bindings))}({amount}c)")
    return lines


def pools_in(action: dict) -> list[str]:
    found: list[str] = []

    def walk(node):
        if not isinstance(node, dict):
            return
        if node.get("op") == "outcome_item_in_pool":
            found.append(node["pool"])
        if "condition" in node:
            walk(node["condition"])
        for child in node.get("conditions") or []:
            walk(child)

    for guarantee in action.get("guarantees", []):
        walk(guarantee.get("when"))
    for transition in action.get("transitions", []):
        walk(transition.get("when"))
    return found


def pool_rarity_from_banners(game: Game, rule: dict, raw: str) -> int:
    for banner in game.banners:
        if banner["rules"] != rule["_file"]:
            continue
        for pool in banner["pools"]:
            if pool["id"] == raw and pool["items"]:
                return game.entry(pool["items"][0])["rarity"]
    raise SystemExit(f"{rule['id']} 的池 {raw} 没有可推断的稀有度")


def slot_rarity(game: Game, rule: dict, role: str) -> int:
    for path in sorted(game.directory.glob("banner.*.json")):
        banner = load_json(path)
        if banner["rules"] != rule["_file"]:
            continue
        binding = (banner.get("item_bindings") or {}).get(role)
        if binding:
            return game.entry(binding)["rarity"]
    raise SystemExit(f"{rule['id']} 的条目槽 {role} 没有绑定")


def emit_roster(game: Game) -> str:
    lines = [
        f"# 源名册 {game.directory.name}",
        f"roster roster_{encode(game.directory.name)}:",
    ]
    for item in sorted(game.items.values(), key=lambda entry: entry["ident"]):
        tags = ", ".join(f'"{esc(tag)}"' for tag in item["tags"])
        drawable = "true" if item["drawable"] else "false"
        lines.append(
            f'    entry {item["ident"]}: {item["rarity"]}r, "{esc(item["display"])}", [{tags}], {drawable}'
        )
    lines.append("")
    return "\n".join(lines)


def emit_banner(game: Game, banner: dict, rules: dict[tuple[str, str], Symbols]) -> str:
    lines = [f"# 源卡池 {banner['id']}"]
    rule = game.rules[banner["rules"]]
    actions = rule["banners"][0]["actions"]
    roster = f"roster_{encode(game.directory.name)}"
    for action in actions:
        symbols = rules[(banner["rules"], action["id"])]
        if len(actions) == 1:
            banner_name = f"banner_{encode(banner['id'])}"
        else:
            banner_name = f"banner_{encode(banner['id'])}__{encode(action['id'])}"
        lines.append(f"banner {banner_name}:")
        lines.append(f"    source_rule {symbols.rule_name}")
        lines.append(f"    source_roster {roster}")
        if symbols.has_pity:
            lines.append(f'    pity_key "{esc(banner["id"])}"')
        if symbols.has_progress:
            lines.append(f'    progress_key "{esc(banner["id"])}#progress"')
        pools = {pool["id"]: pool for pool in banner["pools"]}
        for raw, ident in symbols.pools.items():
            pool = pools.get(raw)
            if pool is None:
                raise SystemExit(f"{banner['id']} 缺少池 {raw}")
            members = []
            rarity = None
            for item_id in pool["items"]:
                item = game.entry(item_id)
                if rarity is None:
                    rarity = item["rarity"]
                elif item["rarity"] != rarity:
                    raise SystemExit(f"{banner['id']} 池 {raw} 稀有度不一致: {item_id}")
                members.append(item["ident"])
            if not members:
                raise SystemExit(f"{banner['id']} 池 {raw} 为空")
            declared = symbols.pool_rarity[ident]
            if declared != rarity:
                raise SystemExit(
                    f"{banner['id']} 池 {raw} 是 {rarity}r，规则要求 {declared}r"
                )
            lines.append(f"    bind {ident}: Pool[{rarity}r] = [{', '.join(members)}]")
        bindings = banner.get("item_bindings") or {}
        for raw, ident in symbols.slots.items():
            item_id = bindings.get(raw)
            if not item_id:
                raise SystemExit(f"{banner['id']} 缺少条目绑定 {raw}")
            item = game.entry(item_id)
            rarity = symbols.slot_rarity[ident]
            if item["rarity"] != rarity:
                raise SystemExit(f"{banner['id']} 条目 {item_id} 不是 {rarity}r")
            lines.append(f"    bind {ident}: Item[{rarity}r] = {item['ident']}")
    lines.append("")
    return "\n".join(lines)


def build_symbols(rule: dict, action: dict) -> Symbols:
    symbols = Symbols()
    symbols.rule_name = f"rule_{encode(rule['id'])}__{encode(action['id'])}"
    symbols.action = encode(action["id"])
    symbols.distribution = f"dist_{encode(action['distribution'])}"
    symbols.used.add(symbols.rule_name)
    symbols.used.add(symbols.action)
    symbols.used.add(symbols.distribution)
    symbols.pool_rarity = {}
    symbols.slot_rarity = {}
    symbols.has_pity = False
    symbols.has_progress = False
    symbols.has_action = False
    return symbols


def write_game(game: Game) -> None:
    out = OUTPUT / game.directory.name
    out.mkdir(parents=True, exist_ok=True)
    for old in out.glob("*"):
        if old.suffix in {".gmr", ".gml", ".gmb"}:
            old.unlink()
    rules = {}
    for filename, rule in game.rules.items():
        rule["_file"] = filename
        prepare(rule)
        for action in rule["banners"][0]["actions"]:
            symbols = build_symbols(rule, action)
            text = emit_rule(game, rule, action, symbols)
            # pool rarity from the generated pool declarations is stored while emitting
            (out / f"{symbols.rule_name}.gmr").write_text(text, encoding="utf-8")
            rules[(filename, action["id"])] = symbols
    (out / f"roster_{encode(game.directory.name)}.gml").write_text(emit_roster(game), encoding="utf-8")
    for banner in game.banners:
        text = emit_banner(game, banner, rules)
        (out / f"banner_{encode(banner['id'])}.gmb").write_text(text, encoding="utf-8")
    print(
        f"{game.directory.name}: {len(game.items)} entries, "
        f"{len(list(out.glob('*.gmr')))} rules, {len(game.banners)} banners"
    )


def remember_pool_rarity(symbols: Symbols, text: str) -> None:
    for raw, ident in symbols.pools.items():
        match = re.search(rf"{re.escape(ident)}: Pool\[(\d+)r\]", text)
        if not match:
            raise SystemExit(f"规则没有声明池 {ident}")
        symbols.pool_rarity[ident] = int(match.group(1))


def main() -> None:
    if not SOURCE.is_dir():
        raise SystemExit(f"缺少迁移输入 {SOURCE}")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    for directory in sorted(path for path in SOURCE.iterdir() if path.is_dir()):
        game = Game(directory)
        # emit_rule fills pool rarity only indirectly. Rebuild symbols by parsing output.
        out = OUTPUT / game.directory.name
        out.mkdir(parents=True, exist_ok=True)
        for old in out.glob("*"):
            if old.suffix in {".gmr", ".gml", ".gmb"}:
                old.unlink()
        compiled = {}
        for filename, rule in game.rules.items():
            rule["_file"] = filename
            prepare(rule)
            if len(rule["banners"]) != 1:
                raise SystemExit(f"{filename} 含有多个规则模板")
            for action in rule["banners"][0]["actions"]:
                symbols = build_symbols(rule, action)
                text = emit_rule(game, rule, action, symbols)
                remember_pool_rarity(symbols, text)
                (out / f"{symbols.rule_name}.gmr").write_text(text, encoding="utf-8")
                compiled[(filename, action["id"])] = symbols
        (out / f"roster_{encode(game.directory.name)}.gml").write_text(
            emit_roster(game), encoding="utf-8"
        )
        for banner in game.banners:
            (out / f"banner_{encode(banner['id'])}.gmb").write_text(
                emit_banner(game, banner, compiled), encoding="utf-8"
            )
        print(
            f"{game.directory.name}: {len(game.items)} entries, "
            f"{len(list(out.glob('*.gmr')))} rules, {len(game.banners)} banners"
        )


if __name__ == "__main__":
    main()
