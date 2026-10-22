const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const g = require("..");

const root = path.resolve(__dirname, "../../..");
const positive = path.join(root, "spec/gacharule/positive");
const order = [
  "endfield.gmr",
  "genshin.gmr",
  "weapon_ten.gmr",
  "sample.gml",
  "banners.gmb",
];

function rarity(draw, grade) {
  let numer = 0n;
  let denom = 1n;
  for (const branch of draw.branches) {
    if (BigInt(branch.rarity) !== BigInt(grade)) continue;
    const left = BigInt(branch.numer);
    const right = BigInt(branch.denom);
    numer = numer * right + left * denom;
    denom *= right;
  }
  return [numer, denom];
}

function gcd(a, b) {
  while (b !== 0n) {
    const next = a % b;
    a = b;
    b = next;
  }
  return a;
}

test("positive rates and analysis", () => {
  const sources = order.map((name) => ({
    name,
    text: fs.readFileSync(path.join(positive, name), "utf8"),
  }));
  const engine = g.gacha_engine_compile(sources);
  const session = g.gacha_engine_session(engine);
  g.gacha_session_set_count(session, "endfield_event_a", "pity6", "65");
  const draw = g.gacha_session_draw(session, "endfield_event_a", "draw");
  const six = rarity(draw, 6);
  const sixGcd = gcd(six[0], six[1]);
  assert.deepEqual([six[0] / sixGcd, six[1] / sixGcd], [29n, 500n]);
  const report = g.gacha_session_analyze(session, {
    banner: "endfield_event_a",
    action: "draw",
    times: 1,
    actions: 1,
    copies: 1,
    targetKind: "rarity_at_least",
    targetRarity: "6",
    quantiles: [29 / 500],
  });
  assert.ok(Math.abs(report.probabilityWithin - 29 / 500) < 1e-9);

  const genshin = g.gacha_session_draw(session, "genshin_event_a", "draw");
  const five = rarity(genshin, 5);
  const fiveGcd = gcd(five[0], five[1]);
  assert.deepEqual([five[0] / fiveGcd, five[1] / fiveGcd], [3n, 500n]);
  const four = rarity(genshin, 4);
  const fourGcd = gcd(four[0], four[1]);
  assert.deepEqual([four[0] / fourGcd, four[1] / fourGcd], [51n, 1000n]);
});
