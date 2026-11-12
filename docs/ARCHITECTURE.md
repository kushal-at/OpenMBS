# OpenMBS Architecture

OpenMBS is an open-source, pure-Rust alternative to Metal Building Software (MBS), structured as a modular 10-crate Cargo workspace designed for end-to-end Pre-Engineered Building (PEB) engineering, analysis, detailing, costing, and drafting.

```text
desktop / CLI
    -> EngineeringProject (dual-unit US/SI, BuildingSpec, CompanyDatabase, SiteHazardProfile)
    -> process_peb:
         1. Geometry & secondary layout (unequal bays, girt trimming, purlins)
         2. ASCE 7-22 load combinations (Wind MWFRS, Snow, Roof Live, Dead, Seismic)
         3. Solver: 2D frame with tapered I-elements (5-point Gauss quadrature), hinges, P-Delta
         4. AISC 360-16 member checks (Ch. B, D, E, F, G, H with slender-web R_pg)
         5. AISI S100-16 cold-formed secondary checks
         6. Detailing: DG-4 moment end-plates, DG-1 base plates, 3D PartOccurrences
         7. QA audits: DBCHECK database rules, BOMCHECK location reconciliation, 3D Clash Detection
         8. Costing: Manufacturer inventory pricing and conditional CostRule overrides
    -> EngineeringRun (deterministic, versioned, audited)
    -> export_peb_bundle:
         - manifest.json (SHA-256 integrity hashes)
         - verify_report.txt (plain-text MBS Verify report)
         - sheet_E1_anchor_plan.svg (reactions, base plates, anchor rods)
         - sheet_RF1_assembly.svg (rigid frame shop assembly with callouts)
         - shop_RF1.dxf (1:1 full-scale CNC/CAD plate & hole profile)
         - model.ifc (ISO-10303-21 IFC4 STEP structural BIM model)
         - bomcheck.csv & clashes.csv
```

## Workspace Crate Layout

| Crate | Purpose | Key Modules & Structs |
| --- | --- | --- |
| `openmbs-units` | Dual-unit system (US Imperial + SI Metric) with NIST exact constants | `UnitSystem`, `Length`, `Force`, `Pressure`, `Moment`, `Mass`, `format_ft_in` |
| `openmbs-solver` | 2D Euler-Bernoulli frame analysis with tapered members and $P$-$\Delta$ | `Frame2D`, `SectionStiffness::TaperedI` (Gauss-5 quadrature), `solve_frame`, moment releases |
| `openmbs-inventory` | Manufacturer database, catalogs, DM81 supply scope, and DBCHECK | `CompanyDatabase`, `BuiltUpSectionSpec`, `ColdFormedProfile`, `CostRule`, `resolve_cost_rates`, `dbcheck` |
| `openmbs-geometry` | 3D PEB geometry, unequal bays, openings, and girt trimming | `BuildingSpec`, `WallOpening`, `Aabb3`, `trimmed_girt_segments`, `EndwallType` |
| `openmbs-codes` | ASCE 7-22 load generator, AISC 360-16 & AISI S100-16 checks | `generate_asce7_loads`, `aisc360_check_section` ($R_{pg}$, LTB, FLB, $H1$-1), `aisi_s100_check_secondary` |
| `openmbs-detailing` | AISC DG-4 moment end-plates, DG-1 base plates, BOMCHECK, clash detection | `build_detail_model`, `PartOccurrence`, `run_bomcheck`, `run_clash_detection` |
| `openmbs-drawings` | Professional drafting (E1, RF1), 1:1 CNC DXF, and IFC4 STEP BIM | `anchor_plan_svg`, `shop_assembly_svg`, `piece_fabrication_dxf`, `export_ifc4` |
| `openmbs-core` | Vertical slice integration, sizing optimization, and bundle packaging | `EngineeringProject`, `EngineeringRun`, `process_peb`, `optimize_peb_frame`, `export_peb_bundle` |
| `openmbs-cli` | Command-line interface for batch processing and automated QA | CLI verbs: `peb-new`, `peb-run`, `peb-demo`, `verify`, `dbcheck` |
| `openmbs-desktop` | Interactive native GUI workbench (`eframe`/`egui`) | 3D orbitable viewport, design tabs, unit switcher, auto-sizer, export manager |

## Research Module Mapping

| Research Module | Primary Crate(s) | Implementation Status (v0.2.0) |
| --- | --- | --- |
| MOD-01 Projects & Revisions | `openmbs-core` (`storage.rs`, `peb.rs`) | Implemented: atomic file replacement, SHA-256 manifests, immutable snapshot history |
| MOD-02 Geometry & Openings | `openmbs-geometry` | Implemented: unequal bays, asymmetric gable/eave heights, 4-wall openings, girt trimming |
| MOD-03 Loads & Combinations | `openmbs-codes` | Implemented: ASCE 7-22 MWFRS wind directional, ground snow, live, seismic, LRFD/ASD combos |
| MOD-04 Structural Analysis | `openmbs-solver` | Implemented: tapered I-elements (5-point Gauss quadrature), end hinges, iterative $P$-$\Delta$ |
| MOD-05 Optimization | `openmbs-core` (`optimize_peb_frame`) | Implemented: inventory-constrained discrete auto-sizing against AISC 360 & deflection criteria |
| MOD-06 Detailing & Connections | `openmbs-detailing` | Implemented: AISC DG-4 moment end-plates, AISC DG-1 base plates, hole oversize catalogs |
| MOD-07 Inventory & DB Rules | `openmbs-inventory` | Implemented: plate/hot-rolled/cold-formed catalogs, DM81 supply scopes, `dbcheck` integrity tool |
| MOD-08 Costing & Pricing | `openmbs-inventory`, `openmbs-core` | Implemented: DB_USER-style conditional finish/member rules (`CostRule`), itemized takeoffs |
| MOD-09 Erection & Shop Drawings | `openmbs-drawings` | Implemented: Sheet E1 Anchor Plan SVG, Sheet RF1 Shop Assembly SVG, 1:1 CNC DXF |
| MOD-10 BOM & Shipping Lists | `openmbs-detailing` | Implemented: 3D PartOccurrences, piece marks, location-based `run_bomcheck` |
| MOD-11 QA & Audit Tools | `openmbs-inventory`, `openmbs-detailing`, `openmbs-core` | Implemented: DBCHECK rule validation, BOMCHECK piece reconciliation, 3D Clash Detection |
| MOD-12 BIM & Interoperability | `openmbs-drawings` | Implemented: ISO-10303-21 IFC4 STEP export, full-scale DXF profiles |
| MOD-13 GUI & Verify Reports | `openmbs-desktop`, `openmbs-core` | Implemented: native egui 3D workbench, plain-text MBS Verify reports (`verify_report.txt`) |

## Data Flow & Provenance

1. **Deterministic Processing**: Every project file tracks its unit system (`UnitSystem::US` or `UnitSystem::SI`), dimensions, site hazard parameters, and inventory catalogs.
2. **Analysis Pipeline**: The building geometry translates into multi-element 2D frames. Tapered members are integrated numerically using 5-point Gauss-Legendre quadrature across varying web depths. Iterative geometric stiffness accounts for 2nd-order $P$-$\Delta$ sway and $P$-$\delta$ member effects.
3. **Capacity & Verification**: Load cases are combined per ASCE 7-22 Section 2.3/2.4. AISC 360-16 Chapter H interactions ($H1$-1a / $H1$-1b) are evaluated at all critical stations with slender-web bending reductions ($R_{pg}$) and lateral-torsional buckling checks.
4. **Detailing & Output**: Physical 3D bounding boxes are generated for every part. Connections are sized per AISC design guides. Clashes are detected geometrically using Axis-Aligned Bounding Box (AABB) intersection tests. All drawings, BIM models, and reports are bundled atomically with SHA-256 verification hashes.
