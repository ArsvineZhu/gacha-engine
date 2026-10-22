const KEYWORDS = [
  "and",
  "banner",
  "bind",
  "choose",
  "emit",
  "entry",
  "floor",
  "from",
  "grade",
  "guarantee",
  "in",
  "item",
  "linear",
  "not",
  "or",
  "pity_key",
  "progress_key",
  "promote",
  "rarity",
  "reset",
  "rest",
  "result",
  "roster",
  "rule",
  "rescale",
  "source_roster",
  "source_rule",
  "update",
  "weighted",
];

const TYPES = [
  "Action",
  "Bool",
  "Chance",
  "Count",
  "Counter",
  "Distribution",
  "Event",
  "Item",
  "Pool",
  "Rarity",
  "Selector",
];

const SCOPES = ["@pity", "@progress", "@banner", "@action"];

const KEYWORD_DOCS = {
  rule: "机制文件根。一个 .gmr 恰好一个 rule。",
  roster: "名册文件根。一个 .gml 恰好一个 roster。",
  banner: "卡池绑定。一个 .gmb 可以有多个 banner。",
  entry: "名册条目：稀有度、显示名、标签、是否可抽取。",
  source_rule: "这个卡池使用的 rule 名称。",
  source_roster: "这个卡池使用的 roster 名称。",
  pity_key: "共享 @pity 状态的组键。",
  progress_key: "共享 @progress 状态的组键。",
  bind: "把规则里的 Pool 或 Item 绑到名册条目。",
  guarantee: "保底。同一规则里从上往下只应用第一条命中。",
  item: "保底效果：直接产出一个 Item 槽。",
  rarity: "保底效果：该稀有度概率为 1。",
  floor: "保底效果：最低稀有度，后接 rescale 或 promote。",
  rescale: "floor 的条件重抽。",
  promote: "floor 保留更高稀有度，并把门槛档填满。",
  update: "抽后更新块。条件可以读取本次 result。",
  reset: "把计数器写回声明初值。",
  emit: "按已声明的 Event 签名发出事件。",
  from: "由 Pool 构造均匀 Selector。",
  weighted: "带权重的 Selector 块。",
  choose: "按抽前条件在两个 Selector 中选一个。",
  linear: "软保底曲线 linear(counter, start, base, step, cap)。起点已经加一步。",
  rest: "分布里的剩余档。",
  and: "条件合取。",
  or: "条件析取。",
  not: "条件否定。",
  in: "result in pool。",
  result: "本次抽取结果。只在更新和事件参数里可读。",
  grade: "result.grade，本次结果的稀有度。",
};

const SCOPE_DOCS = {
  "@pity": "按 pity_key 共享，动作之间保留。",
  "@progress": "按 progress_key 共享，动作之间保留。",
  "@banner": "当前卡池实例内跨动作保留。",
  "@action": "每次动作开始新建，动作结束销毁。",
};

function codeOf(line) {
  let inString = false;
  let escaped = false;
  let out = "";
  for (const ch of line) {
    if (inString) {
      out += ch;
      if (escaped) {
        escaped = false;
        continue;
      }
      if (ch === "\\") {
        escaped = true;
        continue;
      }
      if (ch === '"') inString = false;
      continue;
    }
    if (ch === '"') {
      inString = true;
      out += ch;
      continue;
    }
    if (ch === "#") break;
    out += ch;
  }
  return out;
}

function inCommentOrString(line, character) {
  let inString = false;
  let escaped = false;
  const end = Math.min(character, line.length);
  for (let i = 0; i < end; i += 1) {
    const ch = line[i];
    if (inString) {
      if (escaped) {
        escaped = false;
        continue;
      }
      if (ch === "\\") {
        escaped = true;
        continue;
      }
      if (ch === '"') inString = false;
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === "#") return true;
  }
  return inString;
}

function declarations(text) {
  const found = new Map();
  text.split(/\r?\n/).forEach((line, index) => {
    const code = codeOf(line);
    const rules = [
      [/^\s*([a-z][a-z0-9_]*)\s*:\s*(Counter|Bool|Pool|Item|Event|Chance|Selector|Distribution|Action)\b/, 2],
      [/^\s*entry\s+([a-z][a-z0-9_]*)\b/, "Entry"],
      [/^\s*guarantee\s+([a-z][a-z0-9_]*)\b/, "Guarantee"],
      [/^\s*(rule|roster|banner)\s+([a-z][a-z0-9_]*)\b/, "root"],
    ];
    for (const [re, typeSpec] of rules) {
      const match = code.match(re);
      if (!match) continue;
      let name;
      let type;
      if (typeSpec === "root") {
        name = match[2];
        type = { rule: "Rule", roster: "Roster", banner: "Banner" }[match[1]];
      } else if (typeof typeSpec === "string") {
        name = match[1];
        type = typeSpec;
      } else {
        name = match[1];
        type = match[typeSpec];
      }
      found.set(name, {
        name,
        type,
        line: index,
        column: code.indexOf(name),
        signature: code.trim(),
      });
      break;
    }
  });
  return [...found.values()];
}

function prefixAt(line, character) {
  const match = line.slice(0, character).match(/(@?[A-Za-z0-9_]*)$/);
  return match ? match[1] : "";
}

function wordAt(line, character) {
  const re = /(@[A-Za-z]+|[A-Za-z_][A-Za-z0-9_]*)/g;
  let match;
  while ((match = re.exec(line))) {
    const start = match.index;
    const end = start + match[0].length;
    if (character >= start && character <= end) {
      return { text: match[0], start, end };
    }
  }
  return null;
}

function completeAt(text, line, character) {
  const source = text.split(/\r?\n/)[line] || "";
  if (inCommentOrString(source, character)) return [];
  const prefix = prefixAt(source, character);
  const before = source.slice(0, character - prefix.length);
  if (before.endsWith(".")) {
    return "grade".startsWith(prefix)
      ? [{ label: "grade", kind: "keyword", detail: KEYWORD_DOCS.grade }]
      : [];
  }
  const items = [];
  const add = (label, kind, detail) => {
    if (label.startsWith(prefix)) items.push({ label, kind, detail });
  };
  for (const decl of declarations(text)) add(decl.name, "name", decl.type);
  for (const typeName of TYPES) add(typeName, "type", "类型名");
  for (const keyword of KEYWORDS) add(keyword, "keyword", KEYWORD_DOCS[keyword]);
  for (const scope of SCOPES) add(scope, "scope", SCOPE_DOCS[scope]);
  add("true", "keyword", "布尔字面量");
  add("false", "keyword", "布尔字面量");
  return items;
}

function hoverAt(text, line, character) {
  const source = text.split(/\r?\n/)[line] || "";
  if (inCommentOrString(source, character)) return null;
  const word = wordAt(source, character);
  if (!word) return null;
  const decl = declarations(text).find((item) => item.name === word.text);
  if (decl) return { title: `${decl.name}: ${decl.type}`, body: decl.signature };
  if (KEYWORD_DOCS[word.text]) return { title: word.text, body: KEYWORD_DOCS[word.text] };
  if (TYPES.includes(word.text)) return { title: word.text, body: "类型名" };
  if (SCOPE_DOCS[word.text]) return { title: word.text, body: SCOPE_DOCS[word.text] };
  if (word.text === "true" || word.text === "false") return { title: word.text, body: "布尔字面量" };
  return null;
}

module.exports = {
  KEYWORDS,
  TYPES,
  SCOPES,
  declarations,
  completeAt,
  hoverAt,
  prefixAt,
  inCommentOrString,
};
