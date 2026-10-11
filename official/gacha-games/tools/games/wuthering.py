from source_io import item


def extract(data, sources):
    result = {}
    for group, kind, id_field, name_field in [("characters", "character", "Id", "Name"),
                                             ("weapons", "weapon", "ItemId", "WeaponName")]:
        rows = []
        for row in data[group]:
            if row.get("IsTrial") or row.get("RoleType", 1) != 1 or (kind == "weapon" and row.get("IsShow") is False):
                continue
            key = row[name_field]
            name = data["text_zh"].get(key)
            if not name or name == "敬请期待":
                continue
            names = {"zh-CN": name}
            if data["text_en"].get(key):
                names["en"] = data["text_en"][key]
            rows.append(item("wuthering-waves", kind, row[id_field], name, row["QualityId"],
                             sources[group]["id"], names=names))
            rows[-1]["source_ids"] += [sources["text_zh"]["id"], sources["text_en"]["id"]]
        result[kind] = rows
    return result
