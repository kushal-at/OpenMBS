# 🏗️ OpenMBS

<div align="center">

### **The Free, Open-Source 1:1 Alternative to Metal Building Software (MBS)**

[![GitHub Release](https://img.shields.io/github/v/release/kushal-at/OpenMBS?color=0284c7&style=for-the-badge&logo=github)](https://github.com/kushal-at/OpenMBS/releases)
[![Windows x64](https://img.shields.io/badge/Windows-x64%20%7C%20ARM64-0078d4?style=for-the-badge&logo=windows)](https://github.com/kushal-at/OpenMBS/releases)
[![Pure Rust](https://img.shields.io/badge/Pure-Rust-dea584?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-green?style=for-the-badge)](LICENSE)
[![CI Build](https://img.shields.io/github/actions/workflow/status/kushal-at/OpenMBS/ci.yml?branch=main&style=for-the-badge&logo=githubactions)](https://github.com/kushal-at/OpenMBS/actions)

<br/>

**Design, analyze, detail, cost, manufacture, and draft Pre-Engineered Metal Buildings (PEB) in one unified native enterprise platform.**  
*Replicating the complete, industry-standard Metal Building Software (MBS) workflow—from customer proposal and 3D parametric geometry to ASCE 7-22 wind/snow loads, 2nd-order tapered frame solver, AISC 360-16 design checks, mini-storage unit layouts, mezzanines, 1:1 CNC DXF fabrication detailing, 7-stage MES shop tracking, and IFC4 BIM models.*

<br/>

[⬇️ **Download Windows x64 Release**](https://github.com/kushal-at/OpenMBS/releases/download/v0.2.0/openmbs-v0.2.0-windows-x64.zip) &nbsp;&bull;&nbsp;
[📦 **All Releases**](https://github.com/kushal-at/OpenMBS/releases) &nbsp;&bull;&nbsp;
[🌐 **Live Web Showcase**](https://kushal-at.github.io/OpenMBS/) &nbsp;&bull;&nbsp;
[📖 **20-Module Enterprise Workspace Guide**](#-the-20-module-enterprise-mbs-workspace)

<br/>

```text
========================================================================================================================
 [ 1. Dashboard ] [ 2. Projects ] [ 3. Shape ] [ 4. Bays ] [ 5. Frames ] [ 6. Secondary ] [ 7. Features ] [ 8. Storage ]
 [ 9. Openings ] [ 10. Loads ] [ 11. Engineering ] [ 12. Detailing ] [ 13. Costing ] [ 14. BOM ] [ 15. Inventory ]
 [ 16. Manufacturing ] [ 17. Shipping ] [ 18. Quality QC ] [ 19. Drawing Sheets ] [ 20. Administration ]
========================================================================================================================
    [ 3D BIM Isometric ]    [ Frame-Only 3D ]    [ Roof Framing 3D ]    [ 2D Transverse Elevation ]    [ 2D Roof Plan ]
```

</div>

---

## 🌟 Why OpenMBS?

Metal Building Software (MBS) has been the trusted gold standard of the pre-engineered metal building industry for over 35 years. However, proprietary licensing fees ($10,000+ per workstation), closed file formats, and legacy 1990s user interfaces have kept metal building technology out of reach for modern fabricators and engineering firms.

**OpenMBS is a ground-up, 100% open-source alternative** engineered in **pure Rust** with zero legacy bloat:

| Capability | Legacy MBS | OpenMBS |
|:---|:---:|:---:|
| **Cost & License** | $10,000+ per seat + annual maintenance | **100% Free & Open-Source (MIT / Apache-2.0)** |
| **Architecture** | 32-bit legacy C/C++ & Borland BDE | **Pure Safe Rust, 64-bit Native, Zero C/C++ dependencies** |
| **Interface** | Legacy 90s dialogs & DOS-style inputs | **GPU-accelerated UI + Authentic 20-Module Enterprise Workspace** |
| **Real-time 3D & 2D** | Static wireframe or external viewers | **5 Real-time Viewports: 3D BIM, Frame-Only, Roof Framing, 2D Elev, 2D Plan** |
| **Structural Solver** | Black-box closed solver | **Open 2nd-order $P$-$\Delta$ 5-point Gauss tapered 3-plate I-element solver** |
| **Building Codes** | ASCE 7, AISC 360, AISI S100 | **ASCE 7-22, AISC 360-16, AISI S100-16 with live ratio audit** |
| **Mini-Storage & Lockers** | Separate expensive add-on module | **Built-in Mini-Storage Module: Corridors, Locker Grids, Roll-up Doors** |
| **Mezzanines & Features** | Complex manual modeling | **Parametric Mezzanines, Canopies, Lean-tos, Parapets, Bridge Cranes** |
| **BIM & CAD Deliverables** | Proprietary DXF / ASCII text | **1:1 CNC DXF, Anchor SVG, RF1 Assembly SVG, ER1 Erection Plan, ISO IFC4** |
| **Shop & Shipping** | Standalone paperwork | **7-Stage MES Work Order Tracking & Flatbed Trailer Logistics** |
| **Automation** | Proprietary macro scripts | **Native Headless CLI for CI/CD batch pipelines and automated runs** |
| **Footprint & Startup** | 5+ GB install size, slow startup | **~7 MB portable download, instant sub-second launch** |

---

## 💡 Why is OpenMBS ~7 MB while Legacy MBS is 5+ GB?

A common question from engineers familiar with legacy metal building packages is: *How can OpenMBS deliver the entire MBS workflow at ~7 MB when legacy MBS requires 5+ GB?*

1. **No Multimedia Bloat**: Legacy MBS installers bundle over 3.5 GB of uncompressed 1990s AVI/WMV tutorial videos and training media. OpenMBS hosts tutorials online and in interactive HTML5.
2. **Procedural Geometry vs. Static Block Libraries**: Legacy software includes gigabytes of static 2D DWG block files. OpenMBS computes all 3D tapered I-sections, purlin lap splices, base plates, stiffeners, framed openings, and mini-storage locker partitions **parametrically on-demand in microseconds**.
3. **Pure Rust Native Machine Code**: OpenMBS has zero heavy external runtimes—no 500 MB Chromium/Electron shell, no 800 MB Java VM, and no 2 GB Microsoft Access/Borland database runtime. It compiles directly to native 64-bit CPU instructions.
4. **Embedded Vector & BIM Emitters**: Native Rust SVG, 1:1 CNC DXF, and ISO-10303-21 IFC4 STEP generators produce production drawings mathematically without third-party CAD DLLs.

---

## 🚀 Download & Quick Start

### 1. Download Native Windows Release (No Install Required)
1. Download the latest pre-compiled package:  
   👉 **[openmbs-v0.2.0-windows-x64.zip](https://github.com/kushal-at/OpenMBS/releases/download/v0.2.0/openmbs-v0.2.0-windows-x64.zip)** *(~7 MB)*
2. Extract the folder anywhere on your computer.
3. Double-click `openmbs-desktop.exe` to launch the full GPU-accelerated application.

### 2. Run Headless Batch Pipeline via CLI
OpenMBS includes a standalone command-line executable (`openmbs.exe`) for automated estimation, engineering audits, and batch drawing export:

```powershell
# Create a new PEB warehouse project in US Imperial units
.\openmbs.exe peb-new warehouse.peb.json

# Solve frame, optimize sections, and generate complete drawings/BIM bundle
.\openmbs.exe peb-run warehouse.peb.json ./output/warehouse-bundle
```

The output bundle instantly generates:
- `sheet_E1_anchor_plan.svg` — 2D foundation anchor bolt layout and column reactions
- `sheet_RF1_shop_assembly.svg` — 2D rigid frame shop assembly sheet with piece-mark bubbles
- `sheet_ER1_erection_plan.svg` — 2D roof and wall erection framing plan with grid lines & schedules
- `shop_RF1.dxf` & `shop_C1.dxf` — 1:1 CNC profile cutting & bolt hole patterns ready for fabrication
- `model.ifc` — ISO-10303-21 IFC4 STEP 3D model for Autodesk Revit, Tekla Structures & Trimble Connect
- `verify_report.txt` — Plain-text MBS `.VER` design verification and stress ratio audit
- `certification_letter.txt` — Professional Engineer stamped design certificate
- `bom.csv` & `bomcheck.csv` — Reconciled fabrication bill of materials with piece marks
- `costs.csv` — Multi-tier cost estimation breakdown

### 3. Build From Source (Pure Rust)
OpenMBS has zero external C/C++ or system dependencies:

```bash
# Clone the repository
git clone https://github.com/kushal-at/OpenMBS.git
cd OpenMBS

# Run desktop application
cargo run -p openmbs-desktop --release
```

---

## 📋 The 20-Module Enterprise MBS Workspace

OpenMBS replicates the authentic, end-to-end Metal Building Software production pipeline across 20 dedicated enterprise modules:

| # | Module | Core Functionality |
|---|---|---|
| **1** | **Dashboard** | Business overview, active project status, pending approvals, total building area, steel tonnage, and quotation value |
| **2** | **Project Lifecycle** | Multi-building project management (e.g. BLD-001 Warehouse + BLD-002 Office), customer billing/delivery info, stage tracking |
| **3** | **Building Shape** | Parametric envelope: Span Width, Left/Right Eave Heights, Total Length, Roof Slope (Gable / Single Slope / Multi-Span) |
| **4** | **Bay Spacing** | Non-uniform bay lengths, frame lines (`FL-1` to `FL-N`), and sidewall/roof diagonal rod/cable X-bracing assignments |
| **5** | **Rigid Frames** | Linearly tapered 3-plate welded I-sections (web depth, flange width, plate thicknesses), moment releases, and 1-click AISC auto-sizer |
| **6** | **Secondary Framing** | Cold-formed 8"-12" Z-purlins, C-girts, eave struts, bypass vs. flush framing, flange braces, and sag rods |
| **7** | **Building Features** | Cantilevered & strutted canopies, lean-to sheds, extended parapets, bridge crane runway brackets, and 2nd-floor mezzanines |
| **8** | **Mini-Storage** | Dedicated self-storage module: unit locker layouts (5x10, 10x10, 10x20), interior climate corridors, and roll-up doors |
| **9** | **4-Wall Openings** | Overhead doors, walk doors, and windows with automatic jamb/header framing and **automatic cold-formed girt trimming** |
| **10** | **Codes & Hazard Loads** | ASCE 7-22 directional MWFRS wind pressure distribution ($K_z, q_z, G, C_p, \pm GC_{pi}$), ground snow, and LRFD/ASD combinations |
| **11** | **Engineering Review** | 2nd-order $P$-$\Delta$ sway analysis, governing load combinations, AISC 360-16 / AISI S100-16 interaction ratios, PE stamped sign-off |
| **12** | **Detailing** | DG-4 extended 4-bolt / 8-bolt moment end-plates, DG-1 column base plates, anchor rods, and stiffener plate detailing |
| **13** | **Cost Estimation** | 4-Tier quotation proposals (V1 Standard, V2 Architectural, V3 High-R Thermal, V4 Custom), material margins, and profit takeoff |
| **14** | **Bill of Materials** | Full itemized steel takeoff: plate weight, cold-formed footage, fasteners, and piece-mark reconciliation (`BOMCHECK`) |
| **15** | **Inventory & Database** | Hot-rolled steel catalog (ASTM A572 Gr 50 / A36), cold-formed coils (ASTM A653), shop labor rates, and `DBCHECK` audit |
| **16** | **Manufacturing MES** | 7-Stage shop tracking: Plate Cutting $\to$ Fitting $\to$ Welding $\to$ Drilling $\to$ Blast/Paint $\to$ QA Inspection $\to$ Shipping Staging |
| **17** | **Shipping Logistics** | 48ft flatbed trailer dispatch, axle load distribution, center-of-gravity balance, and multi-truck transport scheduling |
| **18** | **Quality Control** | Automated 3D geometric clash detection (cross-bracing vs. doors, crane clearances, and mezzanine column conflicts) |
| **19** | **Drawing Sheets** | Sheet E1 Anchor Plan, Sheet RF1 Shop Assembly, Sheet ER1 Erection Plan, 1:1 CNC Shop DXF profiles, and ISO IFC4 BIM models |
| **20** | **Administration** | Role-based permissions (Executive, PE Engineer, Detailer, Estimator, Shop Foreman, QC Inspector) and audit logging |

---

## 📐 Multi-Mode Viewport

OpenMBS provides five real-time hardware-accelerated viewport modes:

1. **3D BIM Isometric**: Complete 3D spatial orbit, pan, and zoom showing primary tapered frames, secondary purlins and girts, framed openings, mezzanine platforms, and diagonal cross-bracing.
2. **Frame-Only 3D**: Focus view isolating primary structural frames, haunches, splices, base plates, and moment connections.
3. **Roof Framing 3D**: Dedicated structural roof framing perspective displaying purlin lines, lap splices, sag rods, and roof X-bracing.
4. **2D Transverse Elevation**: Authentic MBS DESED-style cross-section showing clear span, left/right eave heights, roof slope, ridge elevation, and member depth profiles.
5. **2D Roof Plan**: Plan view showing longitudinal bay spacing, frame lines (`FL-1`..`FL-N`), purlin runs, and diagonal cable bracing.

---

## 🏗️ 10-Crate Modular Pure-Rust Architecture

OpenMBS is built as a 10-crate modular workspace with zero unsafe code and zero external native dependencies:

```text
openmbs/
├── crates/
│   ├── units/       # Exact NIST unit conversions (ft, in, kips, psf, ksi <-> m, kN, kPa, MPa)
│   ├── solver/      # 2D tapered 3-plate I-element solver with 5-point Gauss & 2nd-order P-Delta
│   ├── inventory/   # Steel plate catalog, cold-formed shapes, costing rates & DBCHECK audit
│   ├── geometry/    # 3D PEB geometry, bay spacing, girt trimming, lean-tos, canopies, mezzanines, mini-storage
│   ├── codes/       # ASCE 7-22 wind/snow/seismic generator, AISC 360-16 & AISI S100-16 design
│   ├── detailing/   # DG-4 end-plates, DG-1 base plates, anchor rods, BOMCHECK, 3D clash audit
│   ├── drawings/    # Sheet E1 SVG, Sheet RF1 SVG, Sheet ER1 SVG, 1:1 CNC Shop DXF, and ISO IFC4 BIM
│   ├── core/        # Unified orchestration pipeline, multi-building, MES tracking, shipping & manifest
│   ├── cli/         # Command-line interface for batch execution and CI automation
│   └── desktop/     # GPU-accelerated native desktop GUI with 20-module enterprise workspace & 5-mode viewport
├── web/             # Product showcase landing page with interactive HTML5 3D wireframe viewer
└── tests/           # 43+ comprehensive engineering verification and numerical sanity tests
```

---

## 🧪 Rigorous Engineering Verification

OpenMBS is backed by 43 automated engineering verification tests verifying:
- **Cantilever & Simply-Supported Benchmarks**: Deflections and bending moments within 0.1% of closed-form Euler-Bernoulli solutions.
- **Tapered I-Beam Quadrature**: 5-point Gauss-Legendre numerical integration verified against exact analytical integrals.
- **Second-Order $P$-$\Delta$**: Iterative geometric stiffness convergence and sway amplification.
- **Global Equilibrium**: Total horizontal shear and vertical reaction summation matches applied hazard loads exactly.
- **AISC 360-16 Code Capacity**: Slender-web flexural reductions ($R_{pg}$), lateral-torsional buckling (LTB), and web shear ($C_{v1}$).
- **BOMCHECK & 3D Clash Testing**: Geometric interference verification between structural cross-bracing and wall door openings.

Run the test suite at any time:
```bash
cargo test --workspace
```

---

## 🤝 Contributing & Community

OpenMBS is built by structural engineers, metal building detailers, and software craftspeople who believe engineering software should be transparent, accessible, and fast.

- **Found a bug or code discrepancy?** Open an [Issue](https://github.com/kushal-at/OpenMBS/issues).
- **Want to add a feature or manufacturer profile?** Fork the repository and open a [Pull Request](https://github.com/kushal-at/OpenMBS/pulls).
- **Suggestions for the GUI workflow?** Join the discussion in [Discussions](https://github.com/kushal-at/OpenMBS/discussions).

---

## 📄 License

OpenMBS is distributed under the dual **MIT / Apache-2.0** license. You are free to use it for personal, academic, or commercial projects.
