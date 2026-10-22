# Documentation agent rules

Human docs in this directory are Simplified Chinese. Keep this file in English.

- The language specification at `spec/gacharule/GachaRule-Language-v1.md` owns semantics. `language-reference.md` is a lookup. Do not silently diverge from the spec. Record a real compiler or executor gap in the lookup instead of rewriting the rule.
- `development.md` owns build, test, CLI, and directory facts. Other docs link there.
- `sdk.md` owns binding signatures and runtime error codes. Document only functions that exist.
- `official-games.md` owns the migration procedure.
- Tutorial fences in `language-tutorial.md` must match `examples/` byte for byte, including the trailing newline of each source file.
- Do not add a code of conduct, security policy, or contribution ceremony. Keep `LICENSE` at the repository root.
- After editing maintained Markdown, run `python3 ~/.codex/skills/documentation/scripts/check/governance_prose.py` on the changed files and follow the links from `README.md`.
