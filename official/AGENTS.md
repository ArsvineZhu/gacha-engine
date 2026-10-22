# Official game data

`source/` is migration input. The engine does not read JSON.

`games/` is generated GachaRule text. `python3 tools/migrate_official.py` deletes `games/<game>/*.{gmr,gml,gmb}` and rewrites them. Update the generator when a durable change belongs in every regeneration.

`sources.json` records provenance for humans. It is not a schema the compiler loads.

After regeneration, run `cargo test --test official`.
