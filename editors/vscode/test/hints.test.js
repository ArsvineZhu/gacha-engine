const test = require("node:test");
const assert = require("node:assert/strict");
const { declarations, completeAt, hoverAt } = require("../src/hints");

const source = `\
rule demo:
    pity6: Counter[79c] @pity = 0c
    featured: Pool[6r]
    pick: Selector[6r] = from featured
    # pity6 在注释里
    rates: Distribution:
        6r: 1p -> pick
    guarantee hard: pity6 >= 79c -> rarity 6r
`;

test("当前文件的声明类型来自声明行", () => {
  const decls = Object.fromEntries(declarations(source).map((item) => [item.name, item.type]));
  assert.equal(decls.demo, "Rule");
  assert.equal(decls.pity6, "Counter");
  assert.equal(decls.featured, "Pool");
  assert.equal(decls.pick, "Selector");
  assert.equal(decls.rates, "Distribution");
  assert.equal(decls.hard, "Guarantee");
});

test("补全包含已声明名字、关键字和类型，注释里不补全", () => {
  const labels = completeAt(source, 2, 4).map((item) => item.label);
  assert.ok(labels.includes("featured"));
  assert.ok(labels.includes("Pool"));
  assert.ok(labels.includes("from"));
  assert.deepEqual(completeAt(source, 4, 10), []);
  assert.deepEqual(completeAt(source, 1, "    pity6: Counter".length).find((item) => item.label === "Counter").detail, "类型名");
});

test("悬停显示声明类型，关键字悬停显示说明", () => {
  const counter = hoverAt(source, 6, "        6r: 1p -> ".length);
  assert.equal(counter.title, "pick: Selector");
  const keyword = hoverAt(source, 7, 6);
  assert.equal(keyword.title, "guarantee");
  assert.match(keyword.body, /保底/);
});
