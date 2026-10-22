const test = require("node:test");
const assert = require("node:assert/strict");
const {
  orderGroup,
  mergeDiagnostics,
  parseCheckPayload,
  rangeFrom,
} = require("../src/diagnostics");

test("同目录文件按 gmr、gml、gmb 和文件名排序", () => {
  assert.deepEqual(
    orderGroup(["b.gmb", "a.gml", "z.gmr", "a.gmr"]),
    ["a.gmr", "z.gmr", "a.gml", "b.gmb"],
  );
});

test("JSON 诊断保留编译器字段", () => {
  const items = parseCheckPayload(
    '{"diagnostics":[{"code":"E_LEX","file":"r.gmr","line":2,"column":4,"expected":"合法 token","actual":"?" }]}',
  );
  assert.equal(items[0].code, "E_LEX");
  assert.equal(items[0].file, "r.gmr");
  assert.equal(items[0].line, 2);
  assert.equal(items[0].column, 4);
  assert.equal(items[0].expected, "合法 token");
  assert.equal(items[0].actual, "?");
});

test("单文件语法错误在跨文件诊断之外仍然保留", () => {
  const solo = { code: "E_PARSE", file: "/w/a.gmr", line: 3, column: 1, expected: ":", actual: "x" };
  const group = { code: "E_BIND_REF", file: "/w/b.gmb", line: 2, column: 5, expected: "rule", actual: "missing" };
  assert.deepEqual(mergeDiagnostics(solo, group), [group, solo]);
  assert.deepEqual(mergeDiagnostics(solo, solo), [solo]);
  assert.deepEqual(
    mergeDiagnostics(
      { code: "E_TYPE", file: "/w/a.gmr", line: 4, column: 8, expected: "Counter", actual: "Bool" },
      group,
    ),
    [group],
  );
  assert.deepEqual(mergeDiagnostics(solo, undefined), [solo]);
});

test("诊断范围从编译器的 1 基列号覆盖当前 token", () => {
  assert.deepEqual(rangeFrom("    pity6: Counter[79c] @pity = 0c", 4, 5), {
    startLine: 3,
    startChar: 4,
    endLine: 3,
    endChar: 9,
  });
  const entry = 'entry a: 6r, "名", [], true';
  const quote = entry.indexOf('"');
  assert.deepEqual(rangeFrom(entry, 1, quote + 1), {
    startLine: 0,
    startChar: quote,
    endLine: 0,
    endChar: quote + '"名"'.length,
  });
});
