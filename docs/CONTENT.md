# 内容数据与可抽范围

内容快照于 2026-10-11 核对，共 1,597 条规范化记录。完整内容目录、游戏规则和具体卡池名单分别维护，目录数量不能当作某期可抽数量。

| 游戏 | 角色 / 干员记录 | 装备同等概念 | 其他 |
| --- | ---: | ---: | ---: |
| 明日方舟：终末地 | 33 个客户端形态 + 2 名已公告干员 | 80 件客户端武器 + 2 件已公告武器 | 0 |
| 明日方舟 | 432 | 无独立武器抽卡 | 0 |
| 原神 | 124 | 255 件武器 | 0 |
| 崩坏：星穹铁道 | 98 | 170 件光锥 | 0 |
| 绝区零 | 65 | 100 件音擎 | 0 |
| 鸣潮 | 50 | 112 件武器 | 0 |
| 异环 | 23 | 50 件弧盘 | 1 类资源 |

明日方舟的武器属于干员设定，不单独建立抽卡武器目录。目录只保留身份、名称和稀有度等事实，不导入剧情、战斗数值或媒体资源。

## 终末地的更新依据

角色与武器目录使用客户端表字段摘要，保存真实 `chr_*` / `wpn_*` 标识，并与 [EnkaNetwork 数据](https://github.com/EnkaNetwork/API-docs/tree/master/store/ef)、[官方干员目录](https://endfield.hypergryph.com/operator)及版本公告交叉核对。

核对结果：官方目录列出 33 个形态，男女管理员各有一条；合并管理员形态后为 32 个角色身份。Enka 数据核对了 32 个形态及 79 件武器的身份/稀有度。缺少的噗切娜和点心时刻由 [9 月版本公告](https://endfield.hypergryph.com/news/2653)确认；公告同时确认提弗洛斯、寒夜幽影及苦难的尽头。梨诺、诀及对应更新由 [7 月版本公告](https://endfield.hypergryph.com/news/9335)补充核对。

目录排除客户端测试占位 `chr_9000_endmin`；男女管理员保留为角色资料，并标为 `gacha_eligible: false`。这两条属于角色资料，不属于抽卡角色。`gacha_eligible` 缺省表示尚未在目录层指定全局限制，仍需由清单明确列入对应卡池。

[3 月官方更新](https://endfield.hypergryph.com/news/6003)将武器「应急手段」更名为「显锋」，目录采用新名称，客户端标识为 `wpn_sword_0008`。[英文公告](https://endfield.gryphline.com/en-us/news/8907)也记录同次改名。

核对材料包括[核对摘要](../official/gacha-games/tools/reviews/endfield-20261011.json)、[字段核对摘要](../official/gacha-games/tools/reviews/endfield-20261011-audit.json)和[具体卡池名单摘要](../official/gacha-games/tools/reviews/endfield-rosters-20261011-audit.json)。冬猎、寒夜幽影和艺术暴君重构武器清单使用各自池表；活动获取内容噗切娜、点心时刻不因此进入这些抽卡池。这些清单记录 2026 年 9 月卡池，开放时间以公告为准。

## 其他游戏来源

固定来源 URL、完整提交号和 SHA-256 唯一维护于各游戏的 `pools/<游戏>/sources.json`；[导入清单](../official/gacha-games/tools/sources.json)仅引用来源标识。目录通过 `source_documents` 引用来源元数据。条目的 `source_ids` 指向名称与稀有度依据。

- 明日方舟：[ArknightsGameData](https://github.com/Kengxxiao/ArknightsGameData/tree/17759cc8cdf37388414145d16262a3a06f3ab25e)，导入可获得干员。
- 原神：[genshin-db-dist](https://github.com/theBowja/genshin-db-dist/tree/371c228cabc9e182995919e595d67409823a0bbe)，中文角色及武器资料。
- 星穹铁道：[StarRailRes](https://github.com/Mar-7th/StarRailRes/tree/dbe8cdfcfb0bf657b9fe1cf92d5537f118ccb487)，角色与光锥索引。
- 绝区零：[ZenlessData](https://github.com/malu-no996/ZenlessData/tree/1277ebca4b8a7a6c3bcbaac6d5708dc9f4a55f23)，身份、物品稀有度与本地化表关联。
- 鸣潮：[WutheringData](https://github.com/Dimbreath/WutheringData/tree/e9234ffe094b2d944d16b222d31102e8ab32d954)，非试用 RoleType 1 角色及可见武器；可见性不决定卡池资格。
- 异环：[nte-exporter](https://github.com/Golumpa/nte-exporter/tree/93c6626640299184392fdc85256e4ccb6145f2cf)，运行时身份映射去重；另由[官方 1.4 公告](https://nte.perfectworld.com/cn/article/news/gamenews/20260929/264407.html)补入黑羽、罪与罚及当期 A 级目标。

社区整理的客户端表与映射标为 `community`，官方公告标为 `official`。内容表核实身份与等级，不据此验证概率机制。

## 身份与稀有度

规范标识按游戏与概念划分命名空间，`upstream_id` 保留客户端原始标识。异环公告补充的内容在客户端标识核实前使用本地标识。星级保留数值；S/A/B 规范为 5/4/3，同时在 `native_rarity` 保留原等级。资源的等级 1 仅为引擎类别。

`light_cone`、`w_engine`、`arc` 分别保留装备概念。`featured` 来自具体清单的槽位绑定，仅在组装后的条目中添加，不写回通用目录。名称、概念、原生等级和原始标识在编译后仍可供展示使用。

## 复现与更新

```sh
python official/gacha-games/tools/sync.py
cargo run -p gacha-cli -- catalog official/gacha-games/pools/nte/catalog.announced-1.4.json
```

导入器按固定提交下载并验证哈希，再通过各游戏适配器生成每片最多 400 条的目录。终末地的规范目录直接维护人工交叉核对后的事实，导入器跳过自动覆盖；异环手工核对的公告补充文件单独保留。缓存默认位于系统临时目录，可用 `--cache PATH` 指定。

更新来源时，需修改对应游戏 `sources.json` 中的 URL、提交号和旧哈希，重新导入并审查差异；终末地更新需复核来源并同步规范目录与核对摘要。卡池成员另行审查，导入器不会将新角色自动加入历史池。机制依据见[规则研究](RULE_SOURCES.md)。

## 丹青渡公告内容

[独立公告目录](../official/gacha-games/pools/endfield/catalog.announced-1.6.json)补充祀、明河及对应武器玉中身、唤月。四条记录的身份与六星等级来自官方版本导览，客户端标识暂为空。它们不写入已有客户端快照，也不自动进入历史池。概率测试口径与结果见[测试报告](../examples/sdk/endfield-danqingdu/REPORT.md)。
