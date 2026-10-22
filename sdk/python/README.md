# Python 绑定

模块名 `gacha_engine`。函数与 Rust 库是同一组操作，参数和字段见 [SDK](../../docs/sdk.md)。

`pyproject.toml` 使用 maturin，要求 Python 3.9 或更新。在本目录：

```sh
maturin develop
python -m unittest tests/test_contract.py
```

Cargo 侧的测试和文档测试是关闭的。合同测试在 `tests/test_contract.py`。`examples/draw_distribution.py` 从目录加载源文件并打印一次抽取。
