# 分层与文件归属

机制、内容、卡池组装、编译、执行、分析和命令行各有独立职责。`gacha-pack` 依赖 `gacha-schema` 和 `gacha-core`，`gacha-analysis` 使用核心转移接口，`gacha-sdk` 统一接入契约，`gacha-cli` 调用 SDK。核心执行层不读取文件、不下载数据、不决定游戏名单。官方库依赖通用 SDK 和文档解析器，依赖方向不逆转。

```text
.config/                    工具版本与检查策略
  scripts/                  文件长度、文档链接配置
official/gacha-games/       独立官方游戏资料库
  pools/<game>/            规则、内容、实际名单与唯一来源元数据
  tools/games/             各游戏内容导入适配器
  tests/                   官方资料组装与接入测试
crates/
  gacha-schema/src/         文档、内容、状态和规则的数据类型
  gacha-pack/src/
    io.rs                  有大小限制的本地文件读取
    composition.rs         内容验证与规则/目录/清单组装
    compile/               实体、概率、选择器、动作、条件编译
    validation/            引用、类型、资源上限和图结构校验
  gacha-core/src/
    model/                 索引化规则、状态、条目和结果
    engine/                概率计算、选择、更新与抽样
    prepared/              静态表、决策投影、有界计划缓存与 trace
    lifecycle.rs           作用域清理和完整动作的原子执行
    rational.rs, rng.rs    数值与随机数基础设施
  gacha-analysis/src/      查询、目标匹配、首个/第 N 个命中的动态规划
  gacha-sdk/src/           公开加载、会话、分析、模拟与导出
  gacha-cli/src/           SDK 命令适配
  */tests/                 对外契约和集成测试
  */tests/support/         测试辅助代码
tests/fixtures/             独立于真实游戏的数学测试数据
tests/cli/                  发布模式命令行流程检查
examples/
  sdk/endfield-danqingdu/   公开 SDK 与官方库接入；reports/ 保存报告
  performance/             算法对照；reports/ 保存性能数据
  cli/                     查询与状态输入
tools/check*.py             仓库检查与统一入口
```

规则包定义符号化池和角色槽位，清单列出具体可抽成员。内容目录只保存身份和稀有度。导入器不会修改清单；全目录中的新角色不会自动改变旧卡池。明确不可抽取的内容仍能出现在目录，但不能进入可抽池或强制保底。

测试统一放在 `tests/` 中，业务 `src/` 不含 `#[test]` 或 `#[cfg(test)]`。合成数据验证数学和机制，真实清单测试验证实际组装与内容约束。

所有维护的文本文件最多 600 行，目录每片最多 400 条内容记录。集中策略见 [.config](../.config/README.md)，`python tools/check.py` 是本地和 CI 共用入口。

相关契约见[文件格式](PACK_FORMAT.md)、[内容数据](CONTENT.md)和[接口](API.md)。

对外接口见 [SDK](SDK.md)，独立接入项目通过 `tools/check_examples.py` 在仓库外运行。
