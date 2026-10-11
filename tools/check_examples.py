"""在仓库外构建独立接入项目，并通过 SDK 与 CLI 复核结果。"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target/release" / ("gacha.exe" if os.name == "nt" else "gacha")


def source_hashes():
    return {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
            for base in [ROOT / "crates", ROOT / "official/gacha-games"]
            for p in base.rglob("*") if p.suffix in {".rs", ".toml", ".json"} and "target" not in p.parts}


def main():
    before = source_hashes()
    with tempfile.TemporaryDirectory(prefix="gacha-external-consumer-") as directory:
        consumer = Path(directory) / "danqingdu"
        shutil.copytree(ROOT / "examples/sdk/endfield-danqingdu", consumer,
                        ignore=shutil.ignore_patterns("target", "output", "reports"))
        manifest = consumer / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        for crate, local in [("gacha-sdk", "crates/gacha-sdk"), ("gacha-games", "official/gacha-games")]:
            text = text.replace(f'path = "../../../{local}"',
                                'path = ' + json.dumps(str(ROOT / local).replace("\\", "/")))
        manifest.write_text(text, encoding="utf-8", newline="\n")
        dependencies = tomllib.loads(text)["dependencies"]
        assert set(dependencies) == {"gacha-sdk", "gacha-games", "serde_json"}
        assert "[workspace]" in text
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target/external-consumer")
        env["PYTHONUTF8"] = "1"
        subprocess.run(["cargo", "+1.99.0", "run", "--release", "--locked", "--", "10000"],
                       cwd=consumer, env=env, check=True, stdout=subprocess.DEVNULL)
        cases = json.loads((consumer / "cases.json").read_text(encoding="utf-8"))
        baseline = json.loads((consumer / "baseline.json").read_text(encoding="utf-8"))
        for case in cases:
            name = case["case"]
            saved = json.loads((consumer / f"output/{name}.json").read_text(encoding="utf-8"))
            expected = baseline["models"][case["model"]]
            actual = saved["analysis"]["draw_distribution"]["distribution"]
            assert len(expected) == len(actual)
            assert all(abs(a - b["cumulative_probability"]) < 1e-12 for a,b in zip(expected,actual))
            counts = [p["count"] for p in saved["simulation"]["distribution"]]
            digest = hashlib.sha256(json.dumps(counts,separators=(",", ":")).encode()).hexdigest()
            assert digest == baseline["histograms"][name]
            bundle = f"output/{name}.bundle.json"
            result = subprocess.run([str(CLI), "study", bundle, f"output/{name}.request.json",
                                     "--trials", "100", "--seed", "42"], cwd=consumer,
                                    capture_output=True, encoding="utf-8", check=True)
            live = json.loads(result.stdout)
            assert live["analysis"]["fingerprint"] == saved["analysis"]["fingerprint"]
            for a, b in zip(live["analysis"]["draw_distribution"]["distribution"],
                            saved["analysis"]["draw_distribution"]["distribution"]):
                assert abs(a["cumulative_probability"] - b["cumulative_probability"]) < 1e-12
            snapshot = consumer / "output/cli-session.json"
            action = saved["analysis"]["request"]
            subprocess.run([str(CLI), "trace", bundle, action["banner"], action["action"],
                            "--save-session", str(snapshot)], cwd=consumer, check=True, stdout=subprocess.DEVNULL)
            resumed = subprocess.run([str(CLI), "trace", bundle, action["banner"], action["action"],
                                      "--session", str(snapshot)], cwd=consumer, check=True, capture_output=True, encoding="utf-8")
            assert json.loads(resumed.stdout)["results"][0]["trace"]["candidates"]
        destination = ROOT / "examples/sdk/endfield-danqingdu/reports"
        assert before == source_hashes(), "独立接入运行修改了引擎源文件"
        destination.mkdir(parents=True, exist_ok=True)
        for file in (consumer / "output").iterdir():
            if not file.name.endswith(".bundle.json"):
                shutil.copy2(file, destination / file.name)
        evidence = {"consumer_outside_provider_workspace": True, "public_sdk_and_official_library_only": True,
                    "canonical_official_data": True, "temporary_cli_bundles": True, "source_files_unchanged": True,
                    "cases": len(cases), "trials_per_case": 10000, "cli_matches_sdk": True,
                    "cli_trace_and_session_resume": True}
        evidence["golden_cdf_and_seeded_histograms_match"] = True
        (destination / "verification.json").write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
                                                       encoding="utf-8", newline="\n")
        print(json.dumps(evidence, ensure_ascii=False))


if __name__ == "__main__":
    main()
