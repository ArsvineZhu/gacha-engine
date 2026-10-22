const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const { orderGroup, mergeDiagnostics, parseCheckPayload, rangeFrom } = require("./diagnostics");
const { findGacha, runGacha } = require("./gacha");
const { completeAt, hoverAt, prefixAt } = require("./hints");

const selector = { language: "gacharule" };
const timers = new Map();

function activate(context) {
  const collection = vscode.languages.createDiagnosticCollection("gacharule");
  context.subscriptions.push(collection);

  const lint = (document) => {
    if (!document || document.languageId !== "gacharule") return;
    const key = document.uri.toString();
    const pending = timers.get(key);
    if (pending) clearTimeout(pending);
    timers.delete(key);
    void checkDocument(document, collection);
  };
  const schedule = (document) => {
    if (!document || document.languageId !== "gacharule") return;
    const key = document.uri.toString();
    const pending = timers.get(key);
    if (pending) clearTimeout(pending);
    timers.set(
      key,
      setTimeout(() => {
        timers.delete(key);
        void checkDocument(document, collection);
      }, 200),
    );
  };

  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(lint),
    vscode.workspace.onDidSaveTextDocument(lint),
    vscode.workspace.onDidChangeTextDocument((event) => schedule(event.document)),
    vscode.workspace.onDidCloseTextDocument((document) => collection.delete(document.uri)),
    vscode.languages.registerDocumentFormattingEditProvider(selector, {
      provideDocumentFormattingEdits: (document) => formatDocument(document),
    }),
    vscode.languages.registerCompletionItemProvider(
      selector,
      { provideCompletionItems: (document, position) => completionItems(document, position) },
      "@",
      ".",
    ),
    vscode.languages.registerHoverProvider(selector, {
      provideHover: (document, position) => hoverItem(document, position),
    }),
  );

  for (const document of vscode.workspace.textDocuments) lint(document);
}

function deactivate() {}

function resolveBin(document) {
  const configured = vscode.workspace.getConfiguration("gacharule").get("gachaPath");
  const starts = [];
  if (document.uri.scheme === "file") starts.push(path.dirname(document.uri.fsPath));
  const folder = vscode.workspace.getWorkspaceFolder(document.uri);
  if (folder) starts.push(folder.uri.fsPath);
  for (const item of vscode.workspace.workspaceFolders || []) starts.push(item.uri.fsPath);
  return findGacha(configured, starts);
}

function stdinNameOf(document) {
  if (document.uri.scheme === "file") return document.uri.fsPath;
  const match = document.getText().match(/(?:^|\n)\s*(rule|roster|banner)\b/);
  const ext = match ? { rule: "gmr", roster: "gml", banner: "gmb" }[match[1]] : "gmr";
  return `untitled.${ext}`;
}

function groupFiles(filePath) {
  const dir = path.dirname(filePath);
  let names = [];
  try {
    names = fs.readdirSync(dir);
  } catch {
    names = [];
  }
  const files = names
    .filter((name) => name.endsWith(".gmr") || name.endsWith(".gml") || name.endsWith(".gmb"))
    .map((name) => path.join(dir, name));
  if (!files.includes(filePath)) files.push(filePath);
  return orderGroup(files);
}

async function checkDocument(document, collection) {
  const version = document.version;
  const bin = resolveBin(document);
  const text = document.getText();
  const stdinName = stdinNameOf(document);
  const group = document.uri.scheme === "file" ? groupFiles(document.uri.fsPath) : [stdinName];
  const soloResult = await runGacha(
    bin,
    ["check", "--format=json", "--stdin", stdinName, stdinName],
    text,
  );
  if (document.version !== version) return;
  if (soloResult.error && soloResult.error.code === "ENOENT") {
    collection.set(document.uri, [missingBinary(document)]);
    return;
  }
  const solo = itemsFrom(soloResult, stdinName);
  let chosen = solo;
  let paths = document.uri.scheme === "file" ? group : [stdinName];
  if (group.length > 1) {
    const groupResult = await runGacha(
      bin,
      ["check", "--format=json", "--stdin", document.uri.fsPath, ...group],
      text,
    );
    if (document.version !== version) return;
    const grouped = itemsFrom(groupResult, document.uri.fsPath);
    chosen = mergeDiagnostics(solo[0], grouped[0]).map((item) => ({
      ...item,
      file: item.file || document.uri.fsPath,
    }));
    paths = group;
  }
  applyDiagnostics(collection, paths, chosen, document);
}

function itemsFrom(result, fallbackFile) {
  const stdout = result.stdout.trim();
  if (stdout.startsWith("{")) {
    try {
      return parseCheckPayload(stdout).map((item) => ({
        ...item,
        file: item.file || fallbackFile,
      }));
    } catch {
      /* 下面的文本回退处理非 JSON 输出。 */
    }
  }
  if (result.code === 0) return [];
  return [
    {
      code: "E_IO",
      file: fallbackFile,
      line: 1,
      column: 1,
      expected: "gacha check",
      actual: (result.stderr || stdout || "检查失败").trim().slice(0, 500),
    },
  ];
}

function applyDiagnostics(collection, paths, diagnostics, document) {
  const buckets = new Map(paths.map((file) => [file, []]));
  for (const item of diagnostics) {
    const key = buckets.has(item.file)
      ? item.file
      : paths.find((file) => path.basename(file) === item.file);
    buckets.get(key && buckets.has(key) ? key : paths[0]).push(item);
  }
  for (const [file, items] of buckets) {
    const uri = !path.isAbsolute(file) || file === document.uri.fsPath ? document.uri : vscode.Uri.file(file);
    const text = textOf(uri, document);
    const lines = text.split(/\r?\n/);
    collection.set(
      uri,
      items.map((item) => toDiagnostic(item, lines[Math.max(0, item.line - 1)] || "")),
    );
  }
}

function textOf(uri, document) {
  const open = vscode.workspace.textDocuments.find((item) => item.uri.toString() === uri.toString());
  if (open) return open.getText();
  if (document.uri.toString() === uri.toString()) return document.getText();
  if (uri.scheme === "file" && fs.existsSync(uri.fsPath)) return fs.readFileSync(uri.fsPath, "utf8");
  return "";
}

function toDiagnostic(item, lineText) {
  const span = rangeFrom(lineText, item.line, item.column);
  const diagnostic = new vscode.Diagnostic(
    new vscode.Range(span.startLine, span.startChar, span.endLine, span.endChar),
    `${item.code}: 期望 ${item.expected}，实际 ${item.actual}`,
    vscode.DiagnosticSeverity.Error,
  );
  diagnostic.source = "gacha";
  diagnostic.code = item.code;
  return diagnostic;
}

function missingBinary(document) {
  const end = Math.min(1, document.lineAt(0).text.length || 1);
  const diagnostic = new vscode.Diagnostic(
    new vscode.Range(0, 0, 0, end),
    "未找到 gacha。在仓库根目录执行 cargo build，或设置 gacharule.gachaPath。",
    vscode.DiagnosticSeverity.Warning,
  );
  diagnostic.source = "gacha";
  return diagnostic;
}

async function formatDocument(document) {
  const bin = resolveBin(document);
  const result = await runGacha(bin, ["fmt", "--stdin", stdinNameOf(document)], document.getText());
  if (result.error && result.error.code === "ENOENT") {
    throw new Error("未找到 gacha。在仓库根目录执行 cargo build，或设置 gacharule.gachaPath。");
  }
  if (result.code !== 0) {
    throw new Error((result.stderr || "gacha fmt 失败").trim());
  }
  if (result.stdout === document.getText()) return [];
  const last = document.lineAt(document.lineCount - 1).range.end;
  return [vscode.TextEdit.replace(new vscode.Range(0, 0, last.line, last.character), result.stdout)];
}

function completionItems(document, position) {
  const text = document.getText();
  const line = document.lineAt(position.line).text;
  const prefix = prefixAt(line, position.character);
  const items = completeAt(text, position.line, position.character);
  const range = new vscode.Range(position.line, position.character - prefix.length, position.line, position.character);
  const order = { name: "0", type: "1", keyword: "2", scope: "3" };
  const kinds = {
    name: vscode.CompletionItemKind.Variable,
    type: vscode.CompletionItemKind.Class,
    keyword: vscode.CompletionItemKind.Keyword,
    scope: vscode.CompletionItemKind.EnumMember,
  };
  return items.map((item) => {
    const completion = new vscode.CompletionItem(item.label, kinds[item.kind]);
    completion.detail = item.detail;
    completion.range = range;
    completion.sortText = `${order[item.kind] || "9"}_${item.label}`;
    return completion;
  });
}

function hoverItem(document, position) {
  const hover = hoverAt(document.getText(), position.line, position.character);
  if (!hover) return null;
  const markdown = new vscode.MarkdownString();
  markdown.appendCodeblock(hover.title, "gacharule");
  markdown.appendMarkdown(hover.body);
  return new vscode.Hover(markdown);
}

module.exports = { activate, deactivate };
