# 🏗️ OpenMBS

<div align="center">

### **The Free, Open-Source 1:1 Alternative to Metal Building Software (MBS)**

[![GitHub Release](https://img.shields.io/github/v/release/kushal-at/OpenMBS?color=0284c7&style=for-the-badge&logo=github)](https://github.com/kushal-at/OpenMBS/releases)
[![Windows x64](https://img.shields.io/badge/Windows-x64%20%7C%20ARM64-0078d4?style=for-the-badge&logo=windows)](https://github.com/kushal-at/OpenMBS/releases)
[![Pure Rust](https://img.shields.io/badge/Pure-Rust-dea584?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-green?style=for-the-badge)](LICENSE)
[![CI Build](https://img.shields.io/github/actions/workflow/status/kushal-at/OpenMBS/ci.yml?branch=main&style=for-the-badge&logo=githubactions)](https://github.com/kushal-at/OpenMBS/actions)

<br/>

**Design, analyze, detail, cost, and draft Pre-Engineered Metal Buildings (PEB) in one unified native application.**  
*Replicating the complete, industry-standard MBS workflow—from building geometry and ASCE 7-22 wind/snow loads to 2nd-order tapered frame solver, AISC 360-16 design checks, 1:1 CNC DXF detailing, and IFC4 BIM models.*

<br/>

[⬇️ **Download Windows x64**](https://github.com/kushal-at/OpenMBS/releases/download/v0.2.0/openmbs-v0.2.0-windows-x64.zip) &nbsp;&bull;&nbsp;
[📦 **All Releases**](https://github.com/kushal-at/OpenMBS/releases) &nbsp;&bull;&nbsp;
[🌐 **Live Web Showcase**](https://kushal-at.github.io/OpenMBS/) &nbsp;&bull;&nbsp;
[📖 **User Guide**](#-the-10-step-mbs-design-workflow)

<br/>

```text
========================================================================================
 [ 1. Shape ] [ 2. Bay Spacing ] [ 3. Codes & Loads ] [ 4. Rigid Frames ] [ 5. Openings ]
 [ 6. Extensions ] [ 7. Inventory ] [ 8. Costing ] [ 9. Drawings ] [ 10. Verify Reports ]
========================================================================================
   [ 3D BIM Isometric ]         [ 2D Transverse Elevation ]         [ 2D Roof Plan ]
```

</div>

---

## 🌟 Why OpenMBS?

Metal Building Software (MBS) has been the trusted backbone of the pre-engineered metal building industry for decades. However, proprietary licensing, legacy interfaces, and closed file formats have restricted accessibility.

**OpenMBS is an open-source, modern reimagining of MBS** built in **pure Rust** from the ground up:

| Feature | Legacy MBS | OpenMBS |
|:---|:---:|:---:|
| **Cost & License** | $10,000+ per seat, recurring licenses | **100% Free & Open-Source (MIT / Apache-2.0)** |
| **Interface** | Legacy 90s-era dialogs | **Modern GPU-accelerated UI + 1:1 MBS 10-Tab Ribbon** |
| **3D & 2D Visualization** | Wireframe or external viewers | **Real-time 3D BIM Orbit + 2D Transverse & Roof Plans** |
| **Structural Solver** | Closed proprietary solver | **Open 2nd-order $P$-$\Delta$ 5-point Gauss tapered I-element solver** |
| **Building Codes** | ASCE 7, AISC 360, AISI S100 | **ASCE 7-22, AISC 360-16, AISI S100-16 with live ratio audit** |
| **BIM & CAD Export** | Proprietary DXF / ASCII reports | **1:1 CNC DXF, Anchor SVG, RF1 Assembly SVG, & ISO IFC4 BIM** |
| **Automation** | Complex macro scripts | **Native CLI for batch headless runs & CI/CD pipeline automation** |
| **Platforms** | Windows x86 only | **Windows x64, Windows ARM64, and portable cross-platform** |

---

## 🚀 Download & Quick Start

### 1. Download Native Windows Release (No Install Required)
1. Download the latest pre-compiled package:  
   👉 **[openmbs-v0.2.0-windows-x64.zip](https://github.com/kushal-at/OpenMBS/releases/download/v0.2.0/openmbs-v0.2.0-windows-x64.zip)** *(~7 MB)*
2. Extract the folder anywhere on your PC.
3. Double-click `openmbs-desktop.exe` to launch the full GUI application.

### 2. Run Headless Batch Pipeline via CLI
OpenMBS includes a standalone command-line executable (`openmbs.exe`) for automated estimation, engineering audits, and batch drawing export:

```powershell
# Create a new PEB warehouse project in US Imperial units
.\openmbs.exe peb-new warehouse.peb.json --us

# Solve frame, optimize sections, and generate complete drawings/BIM bundle
.\openmbs.exe peb-run warehouse.peb.json ./output/warehouse-bundle
```

The output bundle instantly produces:
- `E1_anchor_plan.svg` — 2D foundation anchor bolt layout and column reactions
- `RF1_shop_assembly.svg` — 2D rigid frame shop assembly sheet with piece marks
- `shop_RF1.dxf` — 1:1 CNC profile cutting & bolt hole patterns ready for fabrication
- `model.ifc` — ISO-10303-21 IFC4 STEP 3D model for Autodesk Revit & Tekla Structures
- `mbs_verify.txt` — Plain-text MBS `.VER` design verification and stress ratio audit
- `pe_certification_letter.txt` — Professional Engineer stamped design certificate

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

## 📋 The 10-Step MBS Design Workflow

OpenMBS replicates the authentic 10-tab Metal Building Software workflow:

```text
+--------------------------------------------------------------------------------------------------+
| OpenMBS Desktop  - [ 100' x 200' Industrial Warehouse - ASCE 7-22 / AISC 360-16 ]               |
+--------------------------------------------------------------------------------------------------+
| [1. Shape] [2. Bays] [3. Loads] [4. Frames] [5. Openings] [6. Ext] [7. DB] [8. Cost] [9. CAD] [10. Verify] |
+------------------------------------+-------------------------------------------------------------+
| PARAMETRIC INPUTS                  | REAL-TIME MULTI-VIEWPORT                                    |
| - Building Width:   100.0 ft       | Mode: (•) 3D BIM    ( ) 2D Elevation    ( ) 2D Roof Plan    |
| - Eave Height:       25.0 ft       |                                                             |
| - Building Length:  200.0 ft       |                       /\  Ridge: 29.17 ft                   |
| - Roof Slope:        1.0 : 12      |                     /    \                                  |
| - Frame Type: Clear Span Gable     |       Eave: 25.0 ft |      | Eave: 25.0 ft                  |
| - Wind Speed:        115 mph       |                    |      |                                |
| - Ground Snow:        25 psf       |                    +------+  Span: 100.0 ft                 |
|                                    |                                                             |
| [ Run Frame Solver ]  [ Auto-Size ]| Status: AISC 360-16 PASS  |  Max Ratio: 0.84 (Rafter Haunch)|
+------------------------------------+-------------------------------------------------------------+
```

### 1. 📐 Building Shape
- Set primary dimensions: Width (Span), Left/Right Eave Heights, Building Length, and Roof Slope.
- Choose between **Clear Span Gable**, **Single Slope**, or **Multi-Span Interior Columns**.
- Real-time dimensional callouts on 2D transverse cross-sections.

### 2. 📏 Bay Spacing & Grid Lines
- Set individual bay lengths along sidewalls (`Bay 1`, `Bay 2`, ..., `Bay N`).
- Automatic generation of rigid frame grid lines (`FL-1`, `FL-2`, `FL-3`, ...).
- Assign rod or cable **X-braced bays** on sidewalls and roof.

### 3. 🌪️ Codes & Hazard Loads
- **ASCE 7-22 Hazard Engine**: Wind speed, Exposure category ($B, C, D$), Elevation, Ground Snow, Roof Live Load, and Seismic $S_s / S_1$.
- Automatic calculation of velocity pressures ($q_z$), external coefficients ($C_p$), internal pressure ($\pm GC_{pi}$), and tributary wind loads.
- Full set of **LRFD & ASD load combinations** automatically assembled.

### 4. 🏛️ Rigid Frames & Primary Members
- Linearly tapered 3-plate welded I-sections (web depth, flange width, web/flange thicknesses).
- Distinct section assignments for Exterior Columns, Haunches, Rafters, and Interior Columns.
- Integrated **AISC 360-16 Auto-Sizer**: 1-click optimization of plate thicknesses to satisfy stress ratios and deflection limits without excess steel weight.

### 5. 🚪 4-Wall Framed Openings
- Place Overhead Doors, Personnel Walk Doors, and Windows on any of the 4 walls (Sidewall 1, Sidewall 2, Endwall 1, Endwall 2).
- Automatic jamb and header framing with **automatic cold-formed girt trimming** around openings.
- Built-in 3D clearance and interference checking.

### 6. 🏗️ Extensions, Cranes & Parapets
- **Lean-To Extensions**: Add left or right lean-to sheds with custom drop heights and roof slopes.
- **Canopies**: Add cantilevered or strutted canopies at sidewall eaves.
- **Bridge Crane Brackets**: Specify crane capacity, runway beam elevations, and wheel wheelbases with automatic bracket design.
- **Parapet Walls**: Model extended parapet facades for commercial architectural elevations.

### 7. 📚 Inventory & Manufacturer Database
- Complete catalog of hot-rolled plates, ASTM A572 Gr 50 / A36 steel, and ASTM A653 Gr 55 cold-formed coils.
- Standard cold-formed 8", 8.5", 9.5", 10", and 11.5" Z-purlins and C-girts with exact section properties.
- Configurable material pricing, shop fabrication labor rates, and paint finish surcharges.

### 8. 💰 Costing & Bill of Materials
- Complete steel takeoff: Primary tapered frame weight, secondary cold-formed weight, sheeting square footage, and fastener counts.
- Piece-mark reconciliation (`BOMCHECK`) ensuring zero orphan parts.
- Detailed cost summary broken down into Material, Shop Fabrication, Paint/Coating, and Freight.

### 9. 📐 CAD & BIM Deliverables
- **1:1 CNC Shop DXF (`shop_RF1.dxf`)**: Ready for direct import into CNC plasma and plate burning tables.
- **Sheet E1 (`E1_anchor_plan.svg`)**: Dimensioned anchor bolt layout plan with column reactions.
- **Sheet RF1 (`RF1_shop_assembly.svg`)**: Primary rigid frame shop assembly sheet with piece-mark bubbles and splice connection callouts.
- **IFC4 STEP (`model.ifc`)**: Industry-standard BIM exchange file compatible with Autodesk Revit, Tekla Structures, and Trimble Connect.

### 10. 📝 Verification & Engineering Reports
- **MBS `.VER` Audit Report**: Plain-text summary of governing load combinations, axial/flexural interaction ratios ($H1$-1a / $H1$-1b), and lateral sway deflections.
- **PE Stamped Certification Letter**: Official design certificate detailing project parameters, building codes, and structural engineer sign-off blocks.

---

## 🏗️ 10-Crate Modular Pure-Rust Architecture

OpenMBS is developed cleanly as a 10-crate modular workspace with zero unsafe code and zero external native dependencies:

```text
openmbs/
├── crates/
│   ├── units/       # Exact NIST unit conversions (ft, in, kips, psf, ksi <-> m, kN, kPa, MPa)
│   ├── solver/      # 2D tapered 3-plate I-element solver with 5-point Gauss & 2nd-order P-Delta
│   ├── inventory/   # Steel plate catalog, cold-formed shapes, costing rates & DBCHECK audit
│   ├── geometry/    # 3D PEB geometry, bay spacing, girt trimming, lean-tos, canopies, parapets
│   ├── codes/       # ASCE 7-22 wind/snow/seismic generator, AISC 360-16 & AISI S100-16 design
│   ├── detailing/   # DG-4 end-plates, DG-1 base plates, anchor rods, BOMCHECK, 3D clash audit
│   ├── drawings/    # Sheet E1 SVG, Sheet RF1 SVG, 1:1 CNC Shop DXF, and ISO-10303-21 IFC4 BIM
│   ├── core/        # Unified orchestration pipeline, project JSON serialization, and manifest
│   ├── cli/         # Command-line interface for batch execution and CI automation
│   └── desktop/     # GPU-accelerated native desktop GUI with 10-tab MBS ribbon and multi-view
├── web/             # Product showcase landing page with interactive HTML5 3D wireframe viewer
└── tests/           # 43+ comprehensive engineering verification and numerical sanity tests
```

---

## 🧪 Rigorous Engineering Verification

OpenMBS is backed by 43 automated engineering verification tests verifying:
- **Cantilever & Simply-Supported Benchmarks**: Deflections and bending moments within 0.1% of closed-form Euler-Bernoulli solutions.
- **Tapered I-Beam Quadrature**: 5-point Gauss-Legendre integration verified against exact analytical integrals.
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
