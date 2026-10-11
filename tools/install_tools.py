"""按集中清单安装固定版本的 Rust 开发工具。"""
from pathlib import Path
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[1]


def main():
    config = tomllib.loads((ROOT / ".config/tools.toml").read_text(encoding="utf-8"))
    for name, version in config["cargo"].items():
        subprocess.run(["cargo", "install", name, "--version", version, "--locked"],
                       cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
