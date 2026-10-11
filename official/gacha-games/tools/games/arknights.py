from source_io import item


def extract(data, sources):
    rows = []
    for id, row in data["characters"].items():
        if not id.startswith("char_") or row.get("isNotObtainable"):
            continue
        rarity = int(row["rarity"].split("_")[-1])
        rows.append(item("arknights", "character", id, row["name"], rarity, sources["characters"]["id"],
                         tags=["operator", row["profession"]]))
    # 明日方舟导入干员，不建立独立抽卡武器目录。
    return {"character": rows}
