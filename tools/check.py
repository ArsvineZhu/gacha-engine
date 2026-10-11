"""本地与 CI 共用的质量检查入口。"""
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    commands = [
        [sys.executable, "tools/check_layout.py"],
        [sys.executable, "tools/check_docs.py"],
        ["cargo", "fmt-check"],
        ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
        ["cargo", "deny-project"],
        ["cargo", "machete"],
        ["jscpd", "--config", ".config/duplication.json", "crates", "official", "examples", "tools"],
        ["cargo", "test", "--workspace", "--locked"],
        ["cargo", "build", "--workspace", "--release", "--locked"],
        [sys.executable, "tests/cli/smoke.py"],
        [sys.executable, "tools/check_examples.py"],
    ]
    for manifest in ["examples/sdk/endfield-danqingdu/Cargo.toml", "examples/performance/Cargo.toml"]:
        commands.insert(3, ["cargo", "fmt", "--manifest-path", manifest, "--", "--check"])
        commands.insert(4, ["cargo", "clippy", "--manifest-path", manifest, "--all-targets", "--locked", "--", "-D", "warnings"])
    for command in commands:
        print("执行：" + " ".join(command), flush=True)
        subprocess.run(command, cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
