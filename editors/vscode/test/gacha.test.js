const path = require("path");
const test = require("node:test");
const assert = require("node:assert/strict");
const { findGacha } = require("../src/gacha");

test("从插件目录向上找到 cargo 产物", () => {
  const repo = path.resolve(__dirname, "../../..");
  const found = findGacha("", [path.join(repo, "editors", "vscode")]);
  assert.equal(found, path.join(repo, "target", "debug", "gacha"));
});
