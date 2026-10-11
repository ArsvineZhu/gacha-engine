from source_io import item


def extract(data, sources):
    result = {}
    for group, kind in [("characters", "character"), ("light_cones", "light_cone")]:
        result[kind] = [item("star-rail", kind, r["id"], r["name"], r["rarity"], sources[group]["id"],
                             tags=[r["path"]]) for r in data[group].values()]
    return result
