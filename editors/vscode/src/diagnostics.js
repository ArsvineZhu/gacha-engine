const path = require("path");

const SYNTAX_CODES = new Set([
  "E_LEX",
  "E_INDENT",
  "E_PARSE",
  "E_EMPTY_BLOCK",
  "E_WEIGHT_EMPTY",
  "E_NONCANONICAL_NUMBER",
  "E_LITERAL_RANGE",
]);

function orderGroup(paths) {
  const rank = (file) => {
    if (file.endsWith(".gmr")) return 0;
    if (file.endsWith(".gml")) return 1;
    if (file.endsWith(".gmb")) return 2;
    return 3;
  };
  return [...paths].sort((a, b) => {
    const byRank = rank(a) - rank(b);
    if (byRank !== 0) return byRank;
    return path.basename(a).localeCompare(path.basename(b));
  });
}

function sameDiagnostic(left, right) {
  return (
    left &&
    right &&
    left.code === right.code &&
    left.file === right.file &&
    left.line === right.line &&
    left.column === right.column
  );
}

function mergeDiagnostics(solo, group) {
  if (!group) return solo ? [solo] : [];
  const out = [group];
  if (solo && SYNTAX_CODES.has(solo.code) && !sameDiagnostic(solo, group)) out.push(solo);
  return out;
}

function parseCheckPayload(stdout) {
  const parsed = JSON.parse(stdout);
  if (!parsed || !Array.isArray(parsed.diagnostics)) {
    throw new Error("gacha check 没有返回 diagnostics");
  }
  return parsed.diagnostics;
}

function rangeFrom(lineText, line, column) {
  const startLine = Math.max(0, (line || 1) - 1);
  const startChar = Math.max(0, (column || 1) - 1);
  let endChar = startChar + 1;
  if (typeof lineText === "string" && startChar < lineText.length) {
    const rest = lineText.slice(startChar);
    const matched = rest.match(
      /^(?:->|==|>=|\+=|\.\.|@(?:pity|progress|banner|action)\b|[A-Za-z_][A-Za-z0-9_]*|\d+(?:\/\d+)?[pwctr]\b|"(?:\\.|[^"\\])*"|.)/,
    );
    if (matched) endChar = startChar + matched[0].length;
  }
  return { startLine, startChar, endLine: startLine, endChar };
}

module.exports = {
  SYNTAX_CODES,
  orderGroup,
  mergeDiagnostics,
  parseCheckPayload,
  rangeFrom,
};
