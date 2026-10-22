const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");

function findGacha(explicit, startDirs) {
  if (explicit && fs.existsSync(explicit)) return explicit;
  const seen = new Set();
  for (const start of startDirs.filter(Boolean)) {
    let dir = start;
    for (let depth = 0; depth < 8 && dir && !seen.has(dir); depth += 1) {
      seen.add(dir);
      for (const rel of ["target/debug/gacha", "target/release/gacha"]) {
        const candidate = path.join(dir, rel);
        if (fs.existsSync(candidate)) return candidate;
      }
      const parent = path.dirname(dir);
      if (parent === dir) break;
      dir = parent;
    }
  }
  return "gacha";
}

function runGacha(bin, args, stdin) {
  return new Promise((resolve) => {
    const child = spawn(bin, args, { stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => child.kill(), 15000);
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({ code: 127, stdout, stderr: error.message, error });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code: code ?? 1, stdout, stderr });
    });
    child.stdin.on("error", () => {});
    child.stdin.end(stdin ?? "");
  });
}

module.exports = { findGacha, runGacha };
