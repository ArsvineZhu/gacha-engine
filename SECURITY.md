# Security model

Rule Packs are data, not plugins. Loading a pack must not execute pack-provided code.

Current controls:

- maximum pack file/text size: 16 MiB;
- bounded top-level node counts;
- bounded actions/rules per banner;
- selector depth limit;
- selector cycle rejection;
- duplicate ID rejection;
- unknown-reference rejection;
- state-type validation;
- empty-pool rejection;
- probability range and sum checks;
- checked integer/rational arithmetic where practical;
- no filesystem/network/process/eval primitives in the Rule Pack language.

The V0.1 limits are defensive defaults rather than a hardened sandbox specification. Before accepting arbitrary remote community packs, add fuzzing, total nested-node budgets, canonical pack hashing, and explicit CPU/state-space budgets for analysis queries.
