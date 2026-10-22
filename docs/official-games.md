# 官方游戏数据

七款游戏的 GachaRule 源在 `official/games/`。迁移输入在 `official/source/`。引擎只编译 `.gmr`、`.gml`、`.gmb`。

| 目录 | 内容 |
|---|---|
| `arknights` | 内核寻访参考规则、名册、一个卡池 |
| `endfield` | 多条规则（特许、武器、重构等）、一份名册、多个卡池 |
| `genshin` | 角色池参考规则、名册、一个卡池 |
| `nte` | 角色与弧光两条规则、一份名册、两个卡池 |
| `star-rail` | 角色池参考规则、名册、一个卡池 |
| `wuthering-waves` | 角色与武器两条规则、一份名册、两个卡池 |
| `zzz` | 角色池参考规则、名册、一个卡池 |

每个游戏目录里，`rule_*.gmr` 是机制，`roster_*.gml` 是名册，`banner_*.gmb` 是绑定。一条规则 JSON 里的每个动作写成一个独立的 `rule` 文件。文件名里的标识符由迁移脚本编码：目录名 `star-rail` 的名册是 `roster_star_rail.gml`，`wuthering-waves` 是 `roster_wuthering_waves.gml`。

`tests/official.rs` 把一个游戏目录里的全部源文件按 `.gmr`、`.gml`、`.gmb`，再按文件名排序后一次编译。`official_games_compile` 要求七个目录都成功。`official_base_rates_match_the_old_rules` 核对若干初始分布，例如原神角色参考池的五星 `3/500`、四星 `51/1000`，以及终末地特许固定五星在 `six_star_pity = 65` 时的六星 `29/500`。断言的完整列表在该测试函数里。

这些源是迁过来的规则文本。`official/source/<游戏>/sources.json` 记录目录和公告的来源注记，供人查阅，不参与编译。规范也写明：示例参数用于表达机制，不代替运营时另行维护的掉率资料。

## 重新生成

在仓库根执行，没有参数：

```sh
python3 tools/migrate_official.py
```

脚本对 `official/source/` 下每个游戏子目录：

1. 读取 `rule.*.json`、`banner.*.json` 和 `catalog.*.json`。
2. 删除 `official/games/<游戏>/` 里已有的 `.gmr`、`.gml`、`.gmb`。
3. 为每条规则的每个动作写一个 `.gmr`，为该游戏写一个 `roster_<游戏>.gml`，为每个 banner JSON 写一个 `.gmb`。
4. 在标准输出打印该游戏的条目数、规则文件数和卡池数。

名册条目来自 `catalog.*.json` 的 `items`。显示名优先取 `names["zh-CN"]`，否则取 `display_name`，再否则取原始 `id`。标识符压成 ASCII 小写：字母和数字保留，`.`、`_`、`-` 变成下划线，其他字符写成 `_u` 加码点十六进制；结果若与关键字冲突，后缀 `_name`。同一游戏的全部目录合成一份名册。

若某个 `rule.*.json` 的 `banners` 数组长度不是 1，脚本以非零状态退出，并指出该文件含有多个规则模板。

生成之后用 [开发](development.md) 里的 `cargo test --test official` 确认这些源仍能编译。
