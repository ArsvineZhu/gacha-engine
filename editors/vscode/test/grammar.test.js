const fs = require("fs");
const path = require("path");
const test = require("node:test");
const assert = require("node:assert/strict");
const { KEYWORDS, TYPES, SCOPES } = require("../src/hints");

const root = path.join(__dirname, "..");
const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const grammar = JSON.parse(
  fs.readFileSync(path.join(root, "syntaxes/gacharule.tmLanguage.json"), "utf8"),
);
const source = fs.readFileSync(path.join(root, "syntaxes/gacharule.tmLanguage.json"), "utf8");

test("三种扩展名注册为同一种语言", () => {
  const language = pkg.contributes.languages[0];
  assert.equal(language.id, "gacharule");
  assert.deepEqual(language.extensions, [".gmr", ".gml", ".gmb"]);
  assert.equal(pkg.contributes.grammars[0].language, "gacharule");
  assert.equal(pkg.contributes.grammars[0].scopeName, "source.gacharule");
});

test("语法按文件根区分顶层关键字，并分开各类 token", () => {
  assert.equal(grammar.scopeName, "source.gacharule");
  assert.equal(grammar.repository["rule-file"].name, "meta.file.rule.gacharule");
  assert.equal(grammar.repository["roster-file"].name, "meta.file.roster.gacharule");
  assert.equal(grammar.repository["banner-file"].name, "meta.file.banner.gacharule");
  for (const keyword of KEYWORDS) {
    assert.match(source, new RegExp(`\\b${keyword}\\b`));
  }
  assert.equal(KEYWORDS.length, 29);
  assert.equal(TYPES.length, 11);
  assert.deepEqual(SCOPES, ["@pity", "@progress", "@banner", "@action"]);
  for (const scope of [
    "comment.line.number-sign.gacharule",
    "string.quoted.double.gacharule",
    "keyword.control.gacharule",
    "keyword.control.rule.gacharule",
    "keyword.control.roster.gacharule",
    "keyword.control.banner.gacharule",
    "invalid.illegal.keyword.gacharule",
    "storage.type.gacharule",
    "constant.language.scope.gacharule",
    "constant.numeric.gacharule",
    "constant.language.boolean.gacharule",
    "keyword.operator.gacharule",
    "punctuation.delimiter.gacharule",
    "variable.other.gacharule",
  ]) {
    assert.ok(source.includes(scope), scope);
  }
  for (const typeName of TYPES) assert.match(source, new RegExp(`\\b${typeName}\\b`));
});
