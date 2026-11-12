# OpenMBS

OpenMBS is an open-source, pure-Rust alternative to **Metal Building Software (MBS)** for Pre-Engineered Building (PEB) design, structural analysis, detailing, costing, and drafting.

Built with a 10-crate modular workspace architecture, OpenMBS mirrors the core MBS workflow—from architectural geometry and ASCE 7-22 hazard loads to 2nd-order tapered frame analysis, AISC 360-16 code design, connection detailing, BOMCHECK, 3D clash detection, 1:1 CNC DXF profiles, and IFC4 BIM models.

```text
                                  +-------------------+
                                  |    OpenMBS GUI    |
                                  | (3D Egui Desktop) |
                                  +---------+---------+
                                            |
+-------------------+             +---------v---------+             +--------------------+
|    OpenMBS CLI    | ----------> |   openmbs-core    | <---------- | Company Database   |
| (Batch / CI Tools)|             | (Pipeline Engine) |             | (Inventory/Rules)  |
+-------------------+             +----+----+----+----+             +--------------------+
                                       |    |    |
           +---------------------------+    |    +--------------------------+
           |                                |                               |
+----------v-----------+         +----------v-----------+        +----------v-----------+
|    openmbs-codes     |         |   openmbs-solver     |        |  openmbs-detailing   |
| ASCE 7-22 / AISC 360 |         | 2D Tapered I-Frames  |        | DG-4 / DG-1 / Clashes|
| AISI S100-16 Secondary|        | Gauss-5 / P-Delta    |        | BOMCHECK             |
+----------------------+         +----------------------+        +----------------------+
           |                                                                |
           +---------------------------+    +-------------------------------+
                                       |    |
                                 +-----v----v-----+
                                 |openmbs-drawings|
                                 | E1 / RF1 / DXF |
                                 |  IFC4 STEP BIM |
                                 +----------------+
```

## Key Capabilities (v0.2.0 PEB Vertical Slice)

- **Dual-Unit System (`openmbs-units`)**: Native support for US Imperial (`ft`, `kips`, `psf`, `ksi`, `mph`, fractional `ft-in`) and SI Metric (`m`, `kN`, `kPa`, `MPa`, `m/s`) with exact NIST conversion constants.
- **Parametric PEB Geometry (`openmbs-geometry`)**: Variable bay spacings, asymmetric gable ridges and eaves, braced bays, 4-wall openings, and automatic girt trimming around framed openings.
- **Code Load Generator (`openmbs-codes`)**: Automated ASCE 7-22 directional MWFRS wind pressure distribution ($K_z, q_z, G, C_p, \pm GC_{pi}$), flat/sloped roof snow, roof live reduction ($R_1, R_2$), equivalent lateral seismic force ($C_s$), and LRFD/ASD/serviceability load combinations.
- **Advanced Tapered Solver (`openmbs-solver`)**: Linearly tapered built-up 3-plate I-elements integrated via 5-point Gauss-Legendre numerical quadrature, moment releases/hinges, station force interpolation, and iterative 2nd-order $P$-$\Delta$ geometric stiffness.
- **AISC 360-16 & AISI S100-16 Member Design (`openmbs-codes`)**: Checks slender-web bending reductions ($R_{pg}$), lateral-torsional buckling (LTB), flange local buckling (FLB), web shear ($C_{v1}$), and combined axial-flexural interaction ($H1$-1a / $H1$-1b) at all stations, alongside cold-formed secondary checks.
- **Connections & Detailing (`openmbs-detailing`)**: AISC Design Guide 4 moment end-plate connections (extended/flush), AISC Design Guide 1 base plates, anchor rod layouts with DM81 supply scope toggles, and hole oversize schedules.
- **QA Audits (`dbcheck`, `bomcheck`, Clash Detection)**:
  - `dbcheck`: Manufacturer database integrity verification (material grades, thickness order, cost rules).
  - `bomcheck`: 3D part occurrence and piece-mark reconciliation (`bomcheck.csv`).
  - 3D Clash Detection: Geometric AABB bounding box interference testing (`clashes.csv`).
- **Professional Drafting & BIM (`openmbs-drawings`)**:
  - Sheet E1: Anchor Bolt & Reaction Plan SVG.
  - Sheet RF1: Rigid Frame Shop Assembly SVG with part callouts.
  - Shop DXF: 1:1 full-scale CNC profile and hole patterns for fabrication.
  - IFC4 BIM: Valid ISO-10303-21 STEP exchange model for Revit, Tekla, and Trimble Connect.
- **Interactive Desktop Workbench (`openmbs-desktop`)**: Orbitable 3D viewport, interactive building shape, loads, member catalogs, discrete auto-sizer/optimizer, MBS Verify report viewer, and bundle exporter.

---

## Installation & Requirements

OpenMBS is written in 100% pure Rust with no external native C/C++ or Python dependencies.

Install stable Rust via [rustup.rs](https://rustup.rs):
```sh
rustup update stable
```

---

## Desktop GUI Workbench

Launch the native desktop interface:
```sh
cargo run -p openmbs-desktop --release
```

- **3D Viewport**: Orbit, pan, and zoom around the structural frame model, secondary framing, and openings.
- **Building Shape**: Configure span, sidewall eave heights, roof slope, and bay lengths in US Imperial or SI Metric.
- **Loads & Codes**: Enter wind speed (mph or m/s), ground snow (psf or kPa), live load, and exposure categories.
- **Primary Members & Auto-Size**: Select tapered built-up sections from inventory or run the discrete auto-sizer to satisfy AISC 360 interaction ratios and deflection limits.
- **Verify & Audit**: View the plain-text MBS Verify report, DBCHECK rules, BOMCHECK piece list, and 3D clash logs directly inside the app.
- **Export Bundle**: Generate the complete project documentation set (IFC, DXF, SVGs, CSVs, Manifest) with one click.

---

## Command-Line Interface (CLI)

OpenMBS CLI provides fast, scriptable batch processing for engineering offices, automated estimation, and CI/CD pipelines.

### 1. Initialize a New PEB Project
Create an engineering project file in US Imperial (`--us`) or SI Metric (`--si`):
```sh
cargo run -p openmbs-cli -- peb-new warehouse.peb.json --us
```

### 2. Run Complete PEB Pipeline & Export Bundle
Execute the complete design, code check, detailing, and drawing generation:
```sh
cargo run -p openmbs-cli -- peb-run warehouse.peb.json output/warehouse-bundle
```

The exported bundle contains:
- `manifest.json`: SHA-256 fingerprint hashes for all generated deliverables.
- `verify_report.txt`: Comprehensive plain-text MBS Verify report (reactions, member checks, governing combinations, cost).
- `sheet_E1_anchor_plan.svg`: Scaled anchor bolt and foundation reaction plan.
- `sheet_RF1_assembly.svg`: Shop assembly drawing with member marks and connection callouts.
- `shop_RF1.dxf`: 1:1 fabrication DXF with outer profiles and bolt hole locations.
- `model.ifc`: ISO-10303-21 IFC4 STEP structural BIM model.
- `bomcheck.csv`: Itemized bill of materials and location reconciliation.
- `clashes.csv`: 3D geometric clash detection log.

### 3. Generate Complete Live Demo Bundle
Generate a turnkey demonstration building and inspect all output deliverables:
```sh
cargo run -p openmbs-cli -- peb-demo output/demo-peb
```

### 4. Format MBS Verify Report to Console
Print the engineering verification report directly to stdout:
```sh
cargo run -p openmbs-cli -- verify warehouse.peb.json
```

### 5. Audit Manufacturer Database with DBCHECK
Verify catalog integrity, rule conflicts, and thickness ordering:
```sh
cargo run -p openmbs-cli -- dbcheck examples/database.json
```

---

## Workspace Architecture

| Crate | Purpose |
| --- | --- |
| [`openmbs-units`](crates/units) | Dual-unit system (US / SI) with NIST constants and architectural formatting |
| [`openmbs-solver`](crates/solver) | 2D Euler-Bernoulli frame solver with Gauss-5 tapered I-elements, releases, and $P$-$\Delta$ |
| [`openmbs-inventory`](crates/inventory) | Manufacturer inventory, built-up catalog, cold-formed shapes, DM81 supply scope, and DBCHECK |
| [`openmbs-geometry`](crates/geometry) | 3D PEB geometry, unequal bays, framed openings, girt trimming, and AABB bounding volumes |
| [`openmbs-codes`](crates/codes) | ASCE 7-22 load combinations, AISC 360-16 tapered checks ($R_{pg}$, LTB, FLB, $H1$-1), AISI S100 |
| [`openmbs-detailing`](crates/detailing) | DG-4 moment end-plates, DG-1 base plates, 3D part occurrences, BOMCHECK, 3D clash detection |
| [`openmbs-drawings`](crates/drawings) | Sheet E1 Anchor Plan SVG, Sheet RF1 Shop Assembly SVG, 1:1 CNC DXF, ISO-10303-21 IFC4 STEP |
| [`openmbs-core`](crates/core) | EngineeringProject, EngineeringRun, vertical slice pipeline, auto-sizer, and bundle packaging |
| [`openmbs-cli`](crates/cli) | CLI binary (`openmbs peb-new`, `peb-run`, `peb-demo`, `verify`, `dbcheck`) |
| [`openmbs-desktop`](crates/desktop) | Interactive native eframe/egui 3D workbench application |

---

## Verification & Testing

OpenMBS includes a comprehensive automated test suite covering analytical formulas, numerical integrations, and code requirements:

```sh
# Run all 43 tests across all 10 workspace crates
cargo test --workspace

# Check formatting and linting
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Documentation

- [Architecture & Module Mapping](docs/ARCHITECTURE.md)
- [Engineering Calculations & Code Checks](docs/ENGINEERING.md)
- [Feature Status Ledger](docs/FEATURE_STATUS.csv)
- [Roadmap & Parity Gates](docs/ROADMAP.md)

---

## License

MIT License. OpenMBS is an independent open-source project and is not affiliated with, sponsored by, or endorsed by Metal Building Software, Inc. (MBS).
