# Engineering Calculations & Verification

OpenMBS v0.2.0 implements an end-to-end Pre-Engineered Building (PEB) structural engineering, code compliance, detailing, and manufacturing verification pipeline in pure Rust. While all calculations follow published ASCE, AISC, and AISI specifications and are verified against automated unit and integration tests, final construction documents always require review and seal by a licensed Professional Engineer (PE) for project-specific site and fabrication requirements.

## 1. Dual-Unit Core (`openmbs-units`)

All internal engineering calculations run deterministically in consistent SI base units (meters, Newtons, Pascals, Newton-meters, kilograms). Inputs and displays transparently convert between US Imperial (`ft`, `ft-in`, `kips`, `psf`, `ksi`, `kip-ft`, `mph`) and SI Metric (`m`, `mm`, `kN`, `kPa`, `MPa`, `kN-m`, `m/s`) using exact NIST constants (`0.3048 m/ft`, `4.4482216152605 N/lbf`). Architectural fractional formatting (`format_ft_in`) handles feet-inches-sixteenths conversion.

## 2. Structural Frame Formulation (`openmbs-solver`)

- **Element Types**: 2D Euler-Bernoulli frame elements supporting both prismatic and **linearly tapered built-up I-sections** (`SectionStiffness::TaperedI`).
- **Numerical Quadrature**: Tapered web-depth stiffness matrices ($k_e$) and consistent nodal load vectors ($f_{eq}$) are integrated along the element axis using 5-point Gauss-Legendre quadrature across exact linear web height interpolation $h(x) = h_i + (h_j - h_i)(x/L)$.
- **End Releases & Hinges**: Pin-ended columns, pinned ridge joints, or single-end moment releases are applied cleanly via static condensation before global stiffness assembly.
- **Second-Order Analysis ($P$-$\Delta$)**: Iterative geometric stiffness matrices ($K_g$) assemble axial force effects ($P/L$) per iteration until relative nodal displacement convergence is achieved, capturing global frame sway stability and amplification.
- **Matrix Conditioning**: Assembled systems undergo diagonal scaling prior to partial-pivot Gaussian elimination. Mechanisms (pivots $< 10^{-12}$) and relative equilibrium residuals $> 10^{-7}$ are rejected deterministically.

## 3. Code Load Generation (`openmbs-codes`)

### ASCE 7-22 Directional MWFRS & Gravity Loads
- **Wind (`generate_asce7_loads`)**: Velocity pressure $q_z = 0.613 \cdot K_z \cdot K_{zt} \cdot K_d \cdot K_e \cdot V^2$ (SI) with Exposure Categories B, C, and D terrain coefficients ($K_z$), gust-effect factor $G$, external pressure coefficients ($C_p$) across windward wall, leeward wall, windward roof, and leeward roof slopes, combined with both positive and negative internal pressure cases ($\pm GC_{pi}$).
- **Snow**: Flat roof snow load $p_f = 0.7 \cdot C_e \cdot C_t \cdot I_s \cdot p_g$ and sloped snow reduction $p_s = C_s \cdot p_f$ per roof pitch.
- **Roof Live**: Tributary area and slope reduction factors ($R_1, R_2$) yielding $L_r = L_o \cdot R_1 \cdot R_2$.
- **Seismic**: Equivalent lateral force seismic response coefficient ($C_s$) distributed to frame eave/ridge heights.
- **Combinations**: Full LRFD (1.4D, 1.2D+1.6L_r+0.5W, 1.2D+1.0W+0.5L_r, 0.9D+1.0W, etc.), ASD, and unfactored Serviceability drift/deflection combinations.

## 4. Member Capacity & Stability Checks (`openmbs-codes`)

### AISC 360-16 Built-Up Tapered & Prismatic I-Sections
Evaluated at multiple station points along every frame member:
- **Section Classification (Ch. B)**: Flange and web compactness limits ($\lambda_p, \lambda_r$) differentiating compact, noncompact, and slender elements.
- **Compression (Ch. E)**: Flexural buckling ($F_{cr}$) with effective length factors and slender-element area reduction ($Q_a$).
- **Flexure (Ch. F)**:
  - Compact/noncompact yielding and flange local buckling (FLB).
  - Lateral-torsional buckling (LTB) checking unbraced length $L_b$ against $L_p$ and $L_r$ with moment gradient factor $C_b$.
  - **Slender-web bending reduction ($R_{pg}$)** per Chapter F5 for deep built-up PEB web plates where $h_c/t_w > 5.70\sqrt{E/F_y}$.
- **Shear (Ch. G)**: Web shear coefficient $C_{v1}$ including tension-field/post-buckling limits based on $h/t_w$.
- **Combined Axial & Flexure (Ch. H)**: Interaction equations **H1-1a** (when $P_r/P_c \ge 0.2$) and **H1-1b** (when $P_r/P_c < 0.2$).

### AISI S100-16 Cold-Formed Secondary Members
Cold-formed Z-purlins and C/Z-girts checked against flexural yielding, effective width local buckling, distortional screening, and serviceability deflection limits under wind uplift and gravity snow/live loads.

## 5. Detailing, Connections & QA (`openmbs-detailing` & `openmbs-inventory`)

- **AISC Design Guide 4 (Moment End-Plates)**: Sizes extended 4-bolt and 8-bolt moment end-plate connections and bolt groups at frame knees and ridge splices.
- **AISC Design Guide 1 (Base Plates)**: Determines base plate dimensions, plate thickness ($t_p$), and anchor rod layouts (with DM81 supply scope inclusion/exclusion toggles).
- **DBCHECK (`dbcheck`)**: Audits manufacturer `CompanyDatabase` integrity—checking monotonic plate thicknesses, duplicate marks, valid material grades, and conflicting `CostRule` overrides.
- **BOMCHECK (`run_bomcheck`)**: Reconciles all 3D `PartOccurrence` items against piece marks, weights, holes, and erection grid locations (`bomcheck.csv`).
- **3D Clash Detection (`run_clash_detection`)**: Tests physical 3D `Aabb3` bounding volumes across primary frames, purlins, girts, framed openings, and connection plates to report geometric interferences (`clashes.csv`).

## 6. Verification Test Suite

The pure-Rust workspace maintains 43 automated unit and integration tests across 10 crates:
- `openmbs-units`: NIST unit conversions, round-trip accuracy, and architectural fractional formatting.
- `openmbs-solver`: Prismatic/tapered cantilever deflections vs analytical exact integrals, end-hinge releases, and $P$-$\Delta$ sway amplification.
- `openmbs-geometry`: Unequal bay spacings, asymmetric gable elevations, and opening girt trimming.
- `openmbs-inventory`: Database rule priority resolution (`resolve_cost_rates`) and `dbcheck` integrity fault injection.
- `openmbs-codes`: ASCE 7-22 wind/snow coefficients, AISC 360-16 slender-web $R_{pg}$ and H1-1 interaction checks, AISI S100 cold-formed screening.
- `openmbs-detailing`: DG-4/DG-1 sizing, BOMCHECK reconciliation, and 3D AABB clash detection.
- `openmbs-drawings`: Valid IFC4 STEP entity references, SVG drawing sheets, and 1:1 DXF entities.
- `openmbs-core`: End-to-end `process_peb` pipeline, `optimize_peb_frame` auto-sizing, Verify report formatting, and bundle exports.
