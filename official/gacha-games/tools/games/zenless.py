from source_io import item


def extract(data, sources):
    # 固定混淆字段映射，来源结构变化时立即失败。
    items = {r["PFOAJKNPCHL"]: r for r in next(iter(data["items"].values()))}
    text = data["text"]
    result = {"character": [], "w_engine": []}
    for group, kind, id_field in [("characters", "character", "PFOAJKNPCHL"),
                                  ("weapons", "w_engine", "HJOEBMMFOMB")]:
        for row in next(iter(data[group].values())):
            metadata = items.get(row[id_field])
            if not metadata:
                continue
            name = text.get(metadata["HAKHHHCAENA"])
            if not name:
                continue
            grade = metadata["CECLEKPMEJB"]
            if grade not in (2, 3, 4):
                raise ValueError("unexpected ZZZ rarity")
            record = item("zzz", kind, row[id_field], name, grade + 1, sources[group]["id"],
                          native={2: "B", 3: "A", 4: "S"}[grade])
            record["source_ids"] += [sources["items"]["id"], sources["text"]["id"]]
            result[kind].append(record)
    return result
