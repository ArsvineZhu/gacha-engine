# Python 绑定

模块名 `gacha_engine`。调用方式是先构造 `Engine`，再在一个卡池上打开 `Session`：

```python
from gacha_engine import Engine

engine = Engine(["endfield.gmr", "sample.gml", "banners.gmb"])
session = engine.session("endfield_event_a")
session.set_count("pity6", "65")
draw = session.draw("draw")
```

`draw.guarantee` 是这一抽生效的保底名，没有保底时为 `None`。每条 `draw.branches` 用 `numer` 和 `denom` 表示精确概率。悬停到类和方法上能看到参数和返回值的说明；编辑器读的是同目录的 `gacha_engine.pyi`。

运行这个模块要先装本地扩展。`pyproject.toml` 使用 maturin，要求 Python 3.9 或更新，并且系统里要有对应的 Python 开发头文件。在本目录：

```sh
maturin develop
python -m unittest tests/test_contract.py
```

Cargo 侧的测试和文档测试是关闭的。合同测试在 `tests/test_contract.py`。`examples/draw_distribution.py` 从目录加载源文件并打印一次抽取。
