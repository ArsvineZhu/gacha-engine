# 拒绝测试清单

每个目录是独立的完整编译单元。按 `*.gmr`、`*.gml`、`*.gmb`（在相同类别下按文件名）顺序输入。`expected` 表中的位置以文件内部 1 基行、Unicode 标量列计数。

| 编号 / 程序目录 | 必须拒绝为 | 错误位置 | 最小反例中的关键源码 | 判定原因 |
|---|---|---|---|---|
| `01_indent_tab/` | `E_INDENT` | `r.gmr:2:1` | `\t` | 缩进使用制表符 |
| `02_indent_width/` | `E_INDENT` | `r.gmr:2:1` | `  p:` | 每级缩进不是四空格 |
| `03_percent_literal/` | `E_LEX` | `r.gmr:2:17` | `0.8%` | 百分号和小数不是源数值形式 |
| `04_noncanonical_rational/` | `E_NONCANONICAL_NUMBER` | `r.gmr:2:17` | `8/1000p` | 概率分数未约成既约分数 |
| `05_bad_rational_denominator/` | `E_NONCANONICAL_NUMBER` | `r.gmr:2:17` | `1/0p` | 有理数的分母必须大于一 |
| `06_probability_over_one/` | `E_LITERAL_RANGE` | `r.gmr:2:17` | `2p` | Chance 字面量超出一 |
| `07_reserved_name/` | `E_PARSE` | `r.gmr:1:6` | `rule` | 关键字不能成为根声明名字 |
| `08_name_unknown/` | `E_NAME_UNKNOWN` | `r.gmr:3:28` | `absent` | 引用不存在的符号 |
| `09_name_before_decl/` | `E_NAME_BEFORE_DECL` | `r.gmr:2:28` | `p` | 首次使用早于声明 |
| `10_name_duplicate/` | `E_DUP_NAME` | `r.gmr:3:5` | `p: Pool[3r]` | 同一作用域的第二次声明 |
| `11_selector_self_cycle/` | `E_SELECTOR_CYCLE` | `r.gmr:2:23` | `s` | 选择器自指形成一节点环 |
| `12_selector_forward_cycle/` | `E_NAME_BEFORE_DECL` | `r.gmr:2:24` | `s2` | 试图用前向引用形成双节点环 |
| `13_bool_as_counter/` | `E_TYPE` | `r.gmr:3:18` | `b >= 1c` | 期望 Counter 状态，实际 Bool |
| `14_counter_as_bool_target/` | `E_TYPE` | `r.gmr:4:9` | `c = false` | 期望 Bool 状态赋值，实际 Counter |
| `15_item_as_chance/` | `E_TYPE` | `r.gmr:6:13` | `i -> s` | 概率位置意外收到 Item |
| `16_selector_rarity_type/` | `E_TYPE` | `r.gmr:3:28` | `p` | 期望 Pool[5r] 实际 Pool[4r] |
| `17_result_in_guarantee/` | `E_RESULT_PHASE` | `r.gmr:3:18` | `result` | 保底条件访问本次结果 |
| `18_result_in_selector/` | `E_RESULT_PHASE` | `r.gmr:3:30` | `result` | 选择器条件访问本次结果 |
| `19_modulo_zero/` | `E_MODULUS` | `r.gmr:3:22` | `0c == 0c` | 模数为 0 |
| `20_modulo_out_of_range/` | `E_MODULUS` | `r.gmr:3:28` | `3c` | 余数必须小于模数 |
| `21_counter_initial_out_of_bounds/` | `E_COUNTER_INIT` | `r.gmr:2:30` | `3c` | 初值大于有界计数器上界 |
| `22_always_saturated_add/` | `E_ALWAYS_SATURATED` | `r.gmr:4:14` | `3c` | 增量超过所有可能可增长空间 |
| `23_zero_increment/` | `E_COUNTER_INIT` | `r.gmr:4:14` | `0c` | 零增量不合法 |
| `24_linear_invalid_cap/` | `E_LINEAR_ARGS` | `r.gmr:3:43` | `1/4p` | 起始概率超过曲线上限 |
| `25_distribution_duplicate_rarity/` | `E_DISTRIBUTION` | `r.gmr:6:9` | `3r: 1p` | 同一稀有度行重复 |
| `26_distribution_fixed_over_one/` | `E_DISTRIBUTION` | `r.gmr:8:13` | `3/4p -> s4` | 固定概率总和大于一 |
| `27_distribution_fixed_under_one/` | `E_DISTRIBUTION` | `r.gmr:5:13` | `1/2p` | 无剩余档时固定概率未恰为一 |
| `28_rest_and_weight/` | `E_REST_MIX` | `r.gmr:8:13` | `1w` | 剩余档混用了 rest 和权重 |
| `29_rest_single_weight/` | `E_REST_FORM` | `r.gmr:8:13` | `1w` | 只有一个加权剩余档，应使用 rest |
| `30_weight_zero/` | `E_LITERAL_RANGE` | `r.gmr:4:9` | `0w` | 权重必须严格大于零 |
| `31_weight_empty/` | `E_WEIGHT_EMPTY` | `r.gmr:3:31` | `:` | 加权选择器没有任何分支 |
| `32_action_zero/` | `E_ACTION_RANGE` | `r.gmr:6:15` | `0t` | 一次动作不得含零抽 |
| `33_action_too_many/` | `E_ACTION_RANGE` | `r.gmr:6:19` | `1001t` | 一次动作不得超过一千抽 |
| `34_action_wrong_target/` | `E_TYPE` | `r.gmr:6:25` | `s` | Action 只允许引用 Distribution |
| `35_rule_no_action/` | `E_RULE_SHAPE` | `r.gmr:1:1` | `rule r` | 缺少 Action 声明 |
| `36_guarantee_rarity_missing/` | `E_GUARANTEE_EFFECT` | `r.gmr:7:33` | `5r` | 强制稀有度缺少对应分布行 |
| `37_floor_promote_no_threshold/` | `E_GUARANTEE_EFFECT` | `r.gmr:7:26` | `floor 4r` | 保留更高策略缺少门槛行 |
| `38_floor_rescale_zero_mass/` | `E_FLOOR_ZERO` | `r.gmr:7:26` | `floor 5r` | 重抽门槛及以上没有任何档 |
| `39_event_wrong_arguments/` | `E_EVENT_SIGNATURE` | `r.gmr:9:21` | `true` | 声明 Count 事件却发出 Bool 参数 |
| `40_reset_bool/` | `E_TYPE` | `r.gmr:4:15` | `f` | Bool 状态不能用 Counter 重置原语 |
| `41_missing_block/` | `E_EMPTY_BLOCK` | `r.gmr:1:7` | `:` | 冒号后未提供缩进块 |
| `42_illegal_keyword_alias/` | `E_PARSE` | `r.gmr:1:1` | `pool` | 不存在 pool 前缀声明语法 |
| `43_bare_numeric_literal/` | `E_LEX` | `r.gmr:5:9` | `6:` | 没有裸整数型，必须写 6r/6c 等 |
| `58_empty_weight_branch/` | `E_PARSE` | `r.gmr:4:11` | `:` | 权重分支的冒号后没有子选择器 |
| `44_unknown_rule/` | `E_BIND_REF` | `b.gmb:2:17` | `missing_rule` | 规则来源根不存在 |
| `45_unknown_roster/` | `E_BIND_REF` | `b.gmb:3:19` | `missing_roster` | 名册来源根不存在 |
| `46_unknown_bind_symbol/` | `E_BIND_REF` | `b.gmb:4:10` | `lost` | 绑定的 Pool 符号未声明 |
| `47_missing_bind/` | `E_BIND_REF` | `b.gmb:1:1` | `banner b` | 已声明 Pool p 未绑定 |
| `48_empty_pool/` | `E_SELECTOR_EMPTY` | `b.gmb:4:24` | `[]` | 池不能是空列表 |
| `49_missing_roster_entry/` | `E_ENTRY_UNKNOWN` | `b.gmb:4:25` | `missing` | 绑定了名册中没有的条目 |
| `50_rarity_mismatch/` | `E_RARITY_MISMATCH` | `b.gmb:4:25` | `x` | 条目稀有度和 Pool[3r] 不同 |
| `51_wrong_bind_annotation/` | `E_BIND_TYPE` | `b.gmb:4:13` | `Pool[4r]` | bind 类型必须精确匹配规则符号 |
| `52_duplicate_bind/` | `E_BIND_REF` | `b.gmb:5:10` | `p` | 同一符号重复绑定 |
| `53_duplicate_pool_member/` | `E_BIND_TYPE` | `b.gmb:4:28` | `x` | 同一个池重复列出条目 |
| `54_undrawable_forced_item/` | `E_ENTRY_DISABLED` | `b.gmb:5:29` | `closed` | 保证槽不得绑定到不可抽取条目 |
| `55_slot_unbound/` | `E_BIND_REF` | `b.gmb:1:1` | `banner b` | 声明了 Item 槽却未绑定 |
| `56_missing_pity_group/` | `E_BIND_REF` | `b.gmb:1:1` | `banner b` | 有 @pity 状态却缺少 pity_key |
| `57_shared_group_schema/` | `E_SHARED_SLOT_SCHEMA` | `b.gmb:9:5` | `pity_key "same"` | 同组同名状态类型/上界不一致 |
