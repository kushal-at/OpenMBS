# Contributing

Use Rust stable and run formatting, workspace tests and Clippy before submission. Keep the core independent of GUI code. Add a numerical reference fixture for solver changes and a failure-path test for validation/persistence changes. Do not label preliminary stress checks as code design.

Every engineering feature must document units, input provenance, assumptions, applicability, governing clauses when relevant, and validation evidence. Every new supported capability should update `docs/FEATURE_STATUS.csv` and the module mapping. Unknown/unsupported cases must remain explicit. Do not remove release blockers until the implementation and independent validation gate exist.

Do not copy proprietary code, private manuals, license-bypassing mechanisms or undocumented vendor formats. Third-party research sources retain their own rights. New contributions are licensed under the repository's MIT software license.
