from source_io import item


def extract(data, sources):
    result = {}
    for group, kind in [("characters", "character"), ("weapons", "weapon")]:
        rows = data[group]["data"]["ChineseSimplified"][group].values()
        result[kind] = [item("genshin", kind, r["id"], r["name"], r["rarity"], sources[group]["id"],
                             tags=[r["weaponType"]]) for r in rows]
    return result
