# GachaRule VS Code 插件

编辑 `.gmr`（机制）、`.gml`（名册）和 `.gmb`（卡池绑定）。三种扩展名注册为同一种语言 `GachaRule`。语法以 `spec/gacharule/GachaRule-Language-v1.md` 第 1–3 节为准。

诊断和格式化都调用仓库里的 `gacha`。类型错误来自 Rust 编译器；补全和悬停只看当前文件已经写下的名字，以及规范里的关键字和类型。

## 四项能力

- **高亮**：打开 `.gmr`、`.gml`、`.gmb` 即生效。关键字、类型、作用域、数值字面量、布尔值、注释、字符串、运算符和标点使用不同的 TextMate 作用域。文件以 `rule`、`roster` 或 `banner` 开头时，不属于该文件的顶层关键字会标成非法关键字。
- **诊断**：打开文件、修改文本或保存时运行 `gacha check --format=json`。问题面板里的 `code`、`file`、`line`、`column`、`expected`、`actual` 就是编译器的输出。
- **格式化**：在 GachaRule 文件里执行「格式化文档」（默认 `Shift+Alt+F`）。保存时格式化需要自行打开 `editor.formatOnSave`。格式化调用 `gacha fmt`，按规范重排换行、四空格缩进、冒号引块和词间空格，不改写 token 文本。
- **补全和悬停**：输入时补全当前文件已声明的名字、关键字、类型名、`true`/`false` 和作用域 `@pity`、`@progress`、`@banner`、`@action`。光标放在名字上时，悬停显示它在本文件中的声明类型，例如 `Counter`、`Pool`、`Selector`。

只检查当前文件时，词法和语法错误会画在这个文件上。同一目录里还有其他 `.gmr`、`.gml`、`.gmb` 时，插件按扩展名（机制、名册、绑定）再按文件名排序，整组交给编译器，从而得到跨文件绑定错误。当前文件使用编辑器里的文本，同目录的其他文件使用磁盘上的内容。

## 从源码运行

在仓库根目录编译：

```sh
cargo build
```

插件按下面的顺序寻找可执行文件：

1. 设置项 `gacharule.gachaPath`
2. 从当前文件向上找到的 `target/debug/gacha`，然后是 `target/release/gacha`
3. `PATH` 里的 `gacha`

`cargo build` 和 `cargo test` 都会生成 `target/debug/gacha`。用 VS Code 打开本目录 `editors/vscode`，按 F5，在弹出的扩展开发主机里打开一份 `.gmr`、`.gml` 或 `.gmb`。

要把插件装进当前窗口，打开命令面板，执行 “Extensions: Install Extension from Location...”，选择 `editors/vscode`。

## 插件测试

```sh
node --test editors/vscode/test/*.js
```

也可以在 `editors/vscode` 里执行 `npm test`。这两条命令都不需要安装 npm 依赖。
