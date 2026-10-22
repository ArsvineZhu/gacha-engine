# gacha-engine

GachaRule 1.0 的编译器和精确执行器。机制、名册、卡池绑定分别写在 `.gmr`、`.gml`、`.gmb` 里。概率在编译期和单步执行中保持为有理数。

规范与语料在 `spec/gacharule/`。

```sh
cargo test
cargo run -- check spec/gacharule/positive/endfield.gmr spec/gacharule/positive/sample.gml spec/gacharule/positive/banners.gmb
```

`check` 的文件必须按依赖顺序给出：被引用的规则和名册在卡池绑定之前。
