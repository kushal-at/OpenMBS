# openmbs-core

Pipeline engine, auto-sizing optimizer, and bundle packaging for OpenMBS.

## Core Capabilities
- **PEB Pipeline Orchestrator (`process_peb`)**: Integrates geometry, ASCE 7-22 load combinations, 2D tapered $P$-$\Delta$ solver, AISC 360-16 checks, AISI S100-16 secondary screening, 3D detailing, BOMCHECK, clash detection, and costing.
- **Auto-Size Frame Optimizer (`optimize_peb_frame`)**: Discrete catalog search selecting optimal built-up sections satisfying AISC unity $\le 1.0$ and drift limits.
- **MBS Verify Report Formatter (`format_verify_report`)**: Plain-text engineering verification report.
- **Atomic Bundle Packager (`export_peb_bundle`)**: Generates deliverables with SHA-256 integrity verification (`manifest.json`).

