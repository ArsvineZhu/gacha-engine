from source_io import item


def extract(data, sources):
    result = {}
    for group, kind in [("characters", "character"), ("arcs", "arc")]:
        rows = []
        seen = set()
        for id, row in data[group].items():
            # 合并同一身份的多个运行时标识。
            if (row["name"], row["rank"]) in seen:
                continue
            seen.add((row["name"], row["rank"]))
            rows.append(item("nte", kind, id, row["name"], {"S": 5, "A": 4, "B": 3}[row["rank"]],
                             sources[group]["id"], native=row["rank"], names={"en": row["name"]}))
        result[kind] = rows
    return result
