# AluVM Design Note

Related docs: [Blueprint](BLUEPRINT.md), [Architecture](ARCHITECTURE.md)

## Status

Exploratory. This is a design note, not a statement of shipped functionality.

## Why it matters

The core crate already contains VM-oriented modules, but the repo does not yet present a production-ready programmable validation story. AluVM is relevant because it could provide:

- deterministic off-chain execution
- bounded resource usage
- a stronger path toward RGB-style compatibility
- cleaner separation between transition logic and chain anchoring

## Where it would fit

The likely integration seam is inside `csv-adapter-core`, between schema and validation machinery:

```text
Schema -> Transition -> Validator -> VM backend
```

Any AluVM integration should preserve the existing division of responsibility:

- base layers enforce single-use
- proof logic verifies anchoring and finality
- VM logic validates transition rules

## Design goals

If this work proceeds, it should aim for:

- additive integration first, not a forced rewrite
- backend abstraction instead of AluVM-specific leakage through the whole codebase
- deterministic outputs that are easy to test and audit
- compatibility with existing schema and validation flows where practical

## Open questions

Before implementation, the project should answer:

1. Is the VM solving a real validation need or only adding architectural novelty?
2. Which current schema or transition features map cleanly to AluVM?
3. What is the minimum useful pilot workload?
4. How will the repo communicate maturity so experimental VM features do not look production-ready by accident?

## Suggested path

1. Introduce a backend abstraction in `csv-adapter-core`.
2. Prototype one narrow validation workload.
3. Compare determinism, ergonomics, and performance against the existing path.
4. Only then decide whether to expand into broader protocol use.
