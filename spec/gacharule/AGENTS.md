# GachaRule specification

This directory is the normative language and its corpus. Human explanation lives in `docs/language-tutorial.md` and `docs/language-reference.md`; do not move the specification's meaning into those files.

- `GachaRule-Language-v1.md` is the semantic authority.
- Positive sources compile together in this order: `endfield.gmr`, `genshin.gmr`, `weapon_ten.gmr`, `sample.gml`, `banners.gmb`.
- Each `rejection-tests/` directory is its own compilation unit. Input order inside a directory is `.gmr`, then `.gml`, then `.gmb`, and by filename within an extension.
- `rejection-tests/README.md` owns the expected code and `file:line:column`. `tests/corpus.rs` reads that table.
- Do not add syntax that the specification does not define.
