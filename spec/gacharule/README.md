# GachaRule 1.0 规范包

本包是**语言规格和源语言测试语料**，不包含编译器、执行器、解释器、JSON 格式或旧格式转换器。

- `GachaRule-Language-v1.md`：中文完整语言规范、完整 EBNF、类型与名称规则、IR 降低、执行语义、58 个必须拒绝的实例索引、禁用特性与验收。
- `positive/`：三个 `.gmr` 机制文件、一个 `.gml` 名册、一个 `.gmb` 卡池绑定文件。
- `rejection-tests/`：58 个独立编译单元（有意非法），以及 `README.md` 中的错误类别和行列坐标。

## 正例的编译输入顺序

```text
positive/endfield.gmr
positive/genshin.gmr
positive/weapon_ten.gmr
positive/sample.gml
positive/banners.gmb
```

必须将这五个文件作为同一个、**按上述顺序**的编译单元。

## 反例的编译输入顺序

每个编号子目录单独编译。先依文件名顺序添加该目录中的 `.gmr` 文件，再添加 `.gml`，最后 `.gmb`。不得把不同编号目录合在一起编译。

断言：编译必须拒绝该程序，固定错误类别和首要错误源码位置必须与 `rejection-tests/README.md` 的对应行一致。

此包中的示例参数用于验证语言表达能力；有关游戏运营时实际公布或未公开的掉率，应另行维护数据来源与注记。
