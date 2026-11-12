# 🏗️ OpenMBS v0.2.0

### **The Free, Open-Source 1:1 Alternative to Metal Building Software (MBS)**

OpenMBS is an authentic, modern open-source Pre-Engineered Metal Building (PEB) software built in pure Rust. It delivers the complete industry-standard MBS engineering workflow—from 3D parametric geometry and ASCE 7-22 wind/snow loads to 2nd-order tapered rigid frame analysis, AISC 360-16 design checks, 1:1 CNC DXF fabrication detailing, and IFC4 BIM models.

---

### 📦 Download Pre-Compiled Binaries

- 📥 **[openmbs-v0.2.0-windows-x64.zip](https://github.com/kushal-at/OpenMBS/releases/download/v0.2.0/openmbs-v0.2.0-windows-x64.zip)** *(Windows x64 Portable, ~7 MB)*
  - Contains `openmbs-desktop.exe` (Native GPU-accelerated Desktop GUI)
  - Contains `openmbs.exe` (Headless CLI batch pipeline runner)
  - Contains `web-showcase.html` (Interactive 3D model viewer)
  - Sample outputs: 1:1 CNC DXF, Anchor Bolt SVG, Assembly SVG, IFC4 BIM, and MBS Verify reports.
  - Zero installer needed: just extract and run!

- 🌐 **[Live Web Showcase & Interactive 3D Viewer](https://kushal-at.github.io/OpenMBS/)**

---

### 🌟 What's New in v0.2.0

#### 1. Authentic 10-Tab MBS Workflow Ribbon
Replicating the exact MBS tabs and design pipeline:
1. **Building Shape**: Width, Eave Height, Length, Roof Slope (Gable / Single Slope), Clear Span or Interior Columns.
2. **Bay Spacing & Grid Lines**: Individual bay lengths, frame lines (FL-1 to FL-N), and X-braced bays.
3. **Codes & Hazard Loads**: ASCE 7-22 directional MWFRS wind pressure distribution ($K_z, q_z, G, C_p, \pm GC_{pi}$), flat/sloped roof snow, roof live load reduction, and seismic forces with LRFD/ASD combinations.
4. **Rigid Frames**: Linearly tapered 3-plate welded I-beams (columns, haunches, rafters), moment releases, and 1-click AISC 360-16 auto-sizer.
5. **4-Wall Openings**: Overhead doors, walk doors, and windows with automatic jamb/header framing and cold-formed girt trimming.
6. **Extensions, Cranes & Parapets**: Lean-to sheds, canopies, parapet walls, and crane runway brackets.
7. **Inventory & Database**: Hot-rolled plates, ASTM A572 Gr 50 / A36 steel, cold-formed Z/C purlins and girts, and pricing rules.
8. **Costing & Takeoff**: Steel tonnage breakdown, fastener counts, fabrication labor, finish paint, and freight.
9. **CAD & BIM Drawings**: 1:1 CNC Shop DXF (`shop_RF1.dxf`), Sheet E1 Anchor Plan SVG, Sheet RF1 Shop Assembly SVG, and ISO IFC4 STEP BIM.
10. **Verify & Reports**: Plain-text MBS `.VER` design verification, AISC stress ratios, deflections, and PE Stamped Certification Letters.

#### 2. Multi-Mode Viewport
- **3D BIM Isometric**: Real-time GPU orbit, pan, and zoom around the structural frame, secondary purlins/girts, and openings.
- **2D Transverse Elevation**: MBS DESED style cross-section showing span, left/right eave heights, ridge elevation, and rafter tapers.
- **2D Roof Plan**: Top-down framing plan showing rigid frame lines (FL-1..FL-N), purlin layout, and roof X-bracing.

#### 3. Standalone Headless CLI
- `openmbs peb-new <project.json>` — Initialize new PEB warehouse projects in US Imperial or SI Metric.
- `openmbs peb-run <project.json> <output_dir>` — Solve, auto-size, and export complete fabrication/BIM bundle.
