//! Complete MBS-aligned PEB engineering, detailing, QA, costing, drafting, and IFC pipeline.
//! All physical quantities persist in canonical SI while supporting dual SI/US Imperial views.

use crate::{PEB_ENGINE, Project, Rates, Result, storage::atomic_write};
use openmbs_codes::{
    AiscDemand, AiscMemberResult, AiscSectionInput, ColdFormedCheckResult, GeneratedCombination,
    GeneratedLoadCase, SiteHazardProfile, aisc360_check_section, aisi_s100_check_secondary,
    generate_asce7_loads,
};
use openmbs_detailing::{DetailModel, PartCategory, build_detail_model};
use openmbs_drawings::{
    DrawingControl, anchor_plan_svg, export_ifc4, piece_fabrication_dxf, shop_assembly_svg,
    shop_sheet_marks,
};
use openmbs_geometry::{BuildingSpec, EndwallType, WallOpening, WallSide};
use openmbs_inventory::{
    AnchorSupplyScope, BuiltUpSectionSpec, CompanyDatabase, DbCheckIssue, DbSeverity,
    resolve_cost_rates,
};
use openmbs_solver::{
    FrameElement, Node, SectionStiffness, Solution, element_station_forces, solve_with_options,
};
use openmbs_units::UnitSystem;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringProject {
    pub schema_version: u32,
    pub name: String,
    pub customer: String,
    pub site: String,
    pub input_source: String,
    pub unit_system: UnitSystem,
    pub building: BuildingSpec,
    pub hazard: SiteHazardProfile,
    pub database: CompanyDatabase,
    pub column_section_id: String,
    pub rafter_section_id: String,
    pub purlin_profile_id: String,
    pub finish_code: String,
    pub anchor_scope: AnchorSupplyScope,
    pub second_order_pdelta: bool,
    pub drawing_control: DrawingControl,
    pub base_rates: Rates,
    pub engineer_approved_by: Option<String>,
}

impl Default for EngineeringProject {
    fn default() -> Self {
        let db = CompanyDatabase::default();
        Self {
            schema_version: 2,
            name: "OpenMBS Demonstration PEB Warehouse".into(),
            customer: "Example Client Corp".into(),
            site: "Standard Exposure C Site".into(),
            input_source: "ASCE 7-22 / AISC 360-16 illustrative project inputs".into(),
            unit_system: UnitSystem::ImperialUS,
            building: BuildingSpec::default(),
            hazard: SiteHazardProfile::default(),
            column_section_id: db.built_up_sections[2].id.clone(),
            rafter_section_id: db.built_up_sections[2].id.clone(),
            purlin_profile_id: db.cold_formed_profiles[1].id.clone(),
            finish_code: db.finishes[0].code.clone(),
            database: db,
            anchor_scope: AnchorSupplyScope::ByManufacturer,
            second_order_pdelta: true,
            drawing_control: DrawingControl {
                unit_system: UnitSystem::ImperialUS,
                ..DrawingControl::default()
            },
            base_rates: Rates {
                currency: "USD".into(),
                source: "Illustrative manufacturer base rates (cents)".into(),
                steel_minor_per_kg: 165,
                panel_minor_per_m2: 1250,
                fabrication_minor_per_kg: 80,
                finish_minor_per_kg: 25,
                markup_basis_points: 1500,
            },
            engineer_approved_by: None,
        }
    }
}

impl EngineeringProject {
    /// Upgrades a v1 preliminary project into a full v2 `EngineeringProject`.
    pub fn from_v1(v1: &Project) -> Self {
        let mut p = Self {
            name: v1.name.clone(),
            customer: v1.customer.clone(),
            site: v1.site.clone(),
            input_source: v1.input_source.clone(),
            unit_system: UnitSystem::MetricSI,
            base_rates: v1.rates.clone(),
            ..Self::default()
        };
        let b = &v1.building;
        let bay_len = b.bay_m();
        p.building = BuildingSpec {
            width_m: b.width_m,
            bay_lengths_m: vec![bay_len; b.bays],
            left_eave_m: b.eave_m,
            right_eave_m: b.eave_m,
            ridge_x_m: b.width_m / 2.0,
            ridge_height_m: b.ridge_m(),
            fixed_bases: b.fixed_bases,
            endwall_type: EndwallType::ExpandableRigidFrame,
            endwall_post_bays: 3,
            purlin_spacing_m: b.purlin_spacing_m,
            girt_spacing_m: b.girt_spacing_m,
            flange_brace_spacing_m: 3.0,
            braced_bays: if b.bays > 1 {
                vec![0, b.bays - 1]
            } else {
                vec![0]
            },
            secondary_kg_per_m: b.secondary_kg_per_m,
            panel_kg_per_m2: b.panel_kg_per_m2,
            purlin_lap_m: 0.30,
            openings: b
                .openings
                .iter()
                .map(|o| WallOpening {
                    id: o.id.clone(),
                    wall: if o.wall == "right" {
                        WallSide::RightSidewall
                    } else {
                        WallSide::LeftSidewall
                    },
                    offset_m: o.offset_m,
                    sill_m: o.sill_m,
                    width_m: o.width_m,
                    height_m: o.height_m,
                    shop_framed: true,
                })
                .collect(),
            ..BuildingSpec::default()
        };
        p.drawing_control.unit_system = UnitSystem::MetricSI;
        p
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StationCheck {
    pub member: String,
    pub governing_xi: f64,
    pub axial_n: f64,
    pub shear_n: f64,
    pub moment_nm: f64,
    pub aisc: AiscMemberResult,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameAnalysisResult {
    pub combination: String,
    pub clause: String,
    pub is_serviceability: bool,
    pub tributary_m: f64,
    pub frame_label: String,
    pub solution: Solution,
    pub member_checks: Vec<StationCheck>,
    pub max_drift_x_m: f64,
    pub max_vertical_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TracedCostLine {
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub rate_minor: u64,
    pub total_minor: u64,
    pub rule_trace: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BomSummaryLine {
    pub mark: String,
    pub category: String,
    pub description: String,
    pub finish_code: String,
    pub count: usize,
    pub length_each_m: f64,
    pub total_mass_kg: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EngineeringRun {
    pub engine: String,
    pub revision: String,
    pub load_cases: Vec<GeneratedLoadCase>,
    pub combinations: Vec<GeneratedCombination>,
    pub frames: Vec<FrameAnalysisResult>,
    pub purlin_check: ColdFormedCheckResult,
    pub detail: DetailModel,
    pub db_issues: Vec<DbCheckIssue>,
    pub bom: Vec<BomSummaryLine>,
    pub costs: Vec<TracedCostLine>,
    pub subtotal_minor: u64,
    pub markup_minor: u64,
    pub total_minor: u64,
    pub max_aisc_ratio: f64,
    pub max_drift_ratio: f64,
    pub max_connection_ratio: f64,
    pub qa_findings: Vec<String>,
    pub release_gates_passed: bool,
}

pub fn fingerprint_peb(p: &EngineeringProject) -> Result<String> {
    p.building.validate()?;
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(p).map_err(|e| e.to_string())?)
    ))
}

impl EngineeringRun {
    pub fn is_current(&self, p: &EngineeringProject) -> bool {
        self.engine == PEB_ENGINE && fingerprint_peb(p).is_ok_and(|h| h == self.revision)
    }

    pub fn steel_mass_kg(&self) -> f64 {
        self.detail.parts.iter().map(|p| p.mass_kg).sum()
    }
}

fn find_section<'a>(
    db: &'a CompanyDatabase,
    id: &str,
    role: &str,
) -> Result<&'a BuiltUpSectionSpec> {
    db.built_up_sections
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("Selected {role} section '{id}' not found in database"))
}

#[allow(clippy::too_many_arguments)]
fn make_frame_elements(
    b: &BuildingSpec,
    col_sec: &BuiltUpSectionSpec,
    raf_sec: &BuiltUpSectionSpec,
    db: &CompanyDatabase,
    left_roof_kpa: f64,
    right_roof_kpa: f64,
    left_wall_kpa: f64,
    right_wall_kpa: f64,
    tributary_m: f64,
    include_self_weight: bool,
    gravity_factor: f64,
) -> (Vec<Node>, Vec<FrameElement>) {
    let mat_col = db
        .materials
        .iter()
        .find(|m| m.id == col_sec.material_id)
        .cloned()
        .unwrap_or_default();
    let mat_raf = db
        .materials
        .iter()
        .find(|m| m.id == raf_sec.material_id)
        .cloned()
        .unwrap_or_default();

    let pts = b.frame_points_xz();
    let mut nodes = Vec::with_capacity(33);
    let mut elements = Vec::with_capacity(32);

    // 4 macro members: 0=Left Column, 1=Left Rafter, 2=Right Rafter, 3=Right Column.
    // Right Column runs top->bottom in pts[3..=4], so depth tapers from eave(pts[3]) -> base(pts[4]).
    for member in 0..4 {
        let (sec, e_pa, density, flip_taper) = match member {
            0 => (col_sec, mat_col.e_pa, mat_col.density_kg_m3, false), // base -> eave
            1 => (raf_sec, mat_raf.e_pa, mat_raf.density_kg_m3, true), // eave(deep) -> ridge(shallow)
            2 => (raf_sec, mat_raf.e_pa, mat_raf.density_kg_m3, false), // ridge(shallow) -> eave(deep)
            _ => (col_sec, mat_col.e_pa, mat_col.density_kg_m3, true), // eave(deep) -> base(shallow)
        };
        let (d_start, d_end) = if flip_taper {
            (sec.depth_end_m, sec.depth_start_m)
        } else {
            (sec.depth_start_m, sec.depth_end_m)
        };

        let dx_tot = pts[member + 1][0] - pts[member][0];
        let dy_tot = pts[member + 1][1] - pts[member][1];
        let len_tot = dx_tot.hypot(dy_tot);

        for j in 0..8 {
            let t0 = j as f64 / 8.0;
            let t1 = (j + 1) as f64 / 8.0;
            nodes.push(Node {
                x: pts[member][0] * (1.0 - t0) + pts[member + 1][0] * t0,
                y: pts[member][1] * (1.0 - t0) + pts[member + 1][1] * t0,
                restrained: if member == 0 && j == 0 {
                    [true, true, b.fixed_bases]
                } else {
                    [false; 3]
                },
            });

            let da = d_start * (1.0 - t0) + d_end * t0;
            let db_m = d_start * (1.0 - t1) + d_end * t1;
            let mid_xi = 0.5 * (t0 + t1);
            let self_wt_npm = if include_self_weight {
                gravity_factor * sec.area_at(mid_xi) * density * 9.80665
            } else {
                0.0
            };

            let qx = match member {
                0 => left_wall_kpa * 1000.0 * tributary_m,
                3 => right_wall_kpa * 1000.0 * tributary_m,
                _ => 0.0,
            };
            let qy = match member {
                1 => -left_roof_kpa * 1000.0 * tributary_m * (dx_tot.abs() / len_tot) - self_wt_npm,
                2 => {
                    -right_roof_kpa * 1000.0 * tributary_m * (dx_tot.abs() / len_tot) - self_wt_npm
                }
                _ => -self_wt_npm,
            };

            elements.push(FrameElement {
                a: member * 8 + j,
                b: member * 8 + j + 1,
                section: SectionStiffness::TaperedI {
                    e: e_pa,
                    depth_a: da,
                    depth_b: db_m,
                    flange_width_a: sec.flange_width_m,
                    flange_width_b: sec.flange_width_m,
                    flange_t_a: sec.flange_thickness_m,
                    flange_t_b: sec.flange_thickness_m,
                    web_t_a: sec.web_thickness_m,
                    web_t_b: sec.web_thickness_m,
                },
                qx,
                qy,
                release_a_mz: false,
                release_b_mz: false,
            });
        }
    }
    nodes.push(Node {
        x: b.width_m,
        y: 0.0,
        restrained: [true, true, b.fixed_bases],
    });
    (nodes, elements)
}

pub fn process_peb(p: &EngineeringProject) -> Result<EngineeringRun> {
    if p.schema_version != 2 {
        return Err("Unsupported EngineeringProject schema version (expected 2)".into());
    }
    p.building.validate()?;
    let col_sec = find_section(&p.database, &p.column_section_id, "column")?;
    let raf_sec = find_section(&p.database, &p.rafter_section_id, "rafter")?;
    let purlin_prof = p
        .database
        .cold_formed_profiles
        .iter()
        .find(|cp| cp.id == p.purlin_profile_id)
        .ok_or_else(|| format!("Selected purlin profile '{}' missing", p.purlin_profile_id))?;

    let db_issues = openmbs_inventory::dbcheck(&p.database, p.unit_system, p.anchor_scope);
    if db_issues.iter().any(|i| i.severity == DbSeverity::Error) {
        let msgs: Vec<String> = db_issues
            .iter()
            .filter(|i| i.severity == DbSeverity::Error)
            .map(|i| format!("{}: {} ({})", i.rule_id, i.record_id, i.actual))
            .collect();
        return Err(format!("DBCHECK failed with errors: {}", msgs.join("; ")));
    }

    let (load_cases, combinations) = generate_asce7_loads(&p.building, &p.hazard);
    let case_map: HashMap<&str, &GeneratedLoadCase> =
        load_cases.iter().map(|c| (c.id.as_str(), c)).collect();

    // Determine governing interior & end rigid-frame tributary widths
    let mut trib_targets: Vec<(String, f64)> = Vec::new();
    let n_frames = p.building.bay_lengths_m.len() + 1;
    let max_interior_idx = (1..n_frames.saturating_sub(1)).max_by(|&a, &b| {
        p.building
            .tributary_width_m(a)
            .total_cmp(&p.building.tributary_width_m(b))
    });
    if let Some(idx) = max_interior_idx {
        trib_targets.push((
            format!("Interior Frame #{}", idx + 1),
            p.building.tributary_width_m(idx),
        ));
    }
    let end_trib = p
        .building
        .tributary_width_m(0)
        .max(p.building.tributary_width_m(n_frames - 1));
    trib_targets.push(("End Rigid Frame".into(), end_trib));

    let col_mat = p
        .database
        .materials
        .iter()
        .find(|m| m.id == col_sec.material_id)
        .cloned()
        .unwrap_or_default();
    let raf_mat = p
        .database
        .materials
        .iter()
        .find(|m| m.id == raf_sec.material_id)
        .cloned()
        .unwrap_or_default();

    let mut frame_results = Vec::new();
    let mut max_aisc_ratio: f64 = 0.0;
    let mut max_drift_ratio: f64 = 0.0;
    let mut max_knee_moment_nm: f64 = 0.0;
    let mut max_ridge_moment_nm: f64 = 0.0;
    let mut max_base_comp_n: f64 = 0.0;
    let mut max_base_uplift_n: f64 = 0.0;
    let mut max_shear_n: f64 = 0.0;

    for (frame_label, trib) in &trib_targets {
        for combo in &combinations {
            let mut l_roof = 0.0;
            let mut r_roof = 0.0;
            let mut l_wall = 0.0;
            let mut r_wall = 0.0;
            let mut dead_factor = 0.0;

            for (cid, factor) in &combo.factors {
                if let Some(lc) = case_map.get(cid.as_str()) {
                    l_roof += lc.left_roof_down_kpa * factor;
                    r_roof += lc.right_roof_down_kpa * factor;
                    l_wall += lc.left_wall_right_kpa * factor;
                    r_wall += lc.right_wall_right_kpa * factor;
                    if lc.family == "D" {
                        dead_factor += factor;
                    }
                }
            }

            let (nodes, elements) = make_frame_elements(
                &p.building,
                col_sec,
                raf_sec,
                &p.database,
                l_roof,
                r_roof,
                l_wall,
                r_wall,
                *trib,
                p.hazard.auto_frame_self_weight,
                dead_factor,
            );
            let sol = solve_with_options(
                &nodes,
                &elements,
                &vec![[0.0; 3]; nodes.len()],
                p.second_order_pdelta && !combo.is_serviceability,
            )?;

            let mut checks = Vec::with_capacity(4);
            let eave_limit = p.building.left_eave_m.max(p.building.right_eave_m) / 200.0;
            let max_drift_x = sol
                .displacement
                .iter()
                .map(|d| d[0].abs())
                .fold(0.0, f64::max);
            let max_vert = sol
                .displacement
                .iter()
                .map(|d| d[1].abs())
                .fold(0.0, f64::max);

            if combo.is_serviceability {
                max_drift_ratio = max_drift_ratio.max(max_drift_x / eave_limit);
            }

            for member in 0..4 {
                let (sec, mat, name, member_len, flip_taper) = match member {
                    0 => (
                        col_sec,
                        &col_mat,
                        "Left Column",
                        p.building.left_eave_m,
                        false,
                    ),
                    1 => (
                        raf_sec,
                        &raf_mat,
                        "Left Rafter",
                        p.building.left_rafter_length_m(),
                        true,
                    ),
                    2 => (
                        raf_sec,
                        &raf_mat,
                        "Right Rafter",
                        p.building.right_rafter_length_m(),
                        false,
                    ),
                    _ => (
                        col_sec,
                        &col_mat,
                        "Right Column",
                        p.building.right_eave_m,
                        true,
                    ),
                };
                let (d0, d1) = if flip_taper {
                    (sec.depth_end_m, sec.depth_start_m)
                } else {
                    (sec.depth_start_m, sec.depth_end_m)
                };

                let mut worst: Option<StationCheck> = None;
                for sub in 0..8 {
                    let elem_idx = member * 8 + sub;
                    let elem = &elements[elem_idx];
                    let stations =
                        element_station_forces(&nodes, elem, &sol.end_forces[elem_idx], 3);
                    for st in stations {
                        let global_xi = (sub as f64 + st.xi) / 8.0;
                        let depth_here = d0 * (1.0 - global_xi) + d1 * global_xi;
                        let aisc = aisc360_check_section(
                            &AiscSectionInput {
                                depth_m: depth_here,
                                flange_width_m: sec.flange_width_m,
                                flange_t_m: sec.flange_thickness_m,
                                web_t_m: sec.web_thickness_m,
                                e_pa: mat.e_pa,
                                fy_pa: mat.fy_pa,
                                unbraced_lb_m: p.building.flange_brace_spacing_m.min(member_len),
                                kx_lx_m: member_len,
                                ky_ly_m: p.building.flange_brace_spacing_m.min(member_len),
                                cb: 1.0,
                                stiffener_spacing_m: None,
                            },
                            &AiscDemand {
                                axial_compression_n: -st.axial_n, // tension + in solver, compression + in AISC
                                moment_nm: st.moment_nm,
                                shear_n: st.shear_n,
                            },
                            p.hazard.design_method,
                        );
                        if !combo.is_serviceability {
                            max_aisc_ratio = max_aisc_ratio.max(aisc.interaction_ratio);
                            max_shear_n = max_shear_n.max(st.shear_n.abs());
                            if member == 0 && sub == 7 && st.xi > 0.99 {
                                max_knee_moment_nm = max_knee_moment_nm.max(st.moment_nm.abs());
                            }
                            if member == 1 && sub == 7 && st.xi > 0.99 {
                                max_ridge_moment_nm = max_ridge_moment_nm.max(st.moment_nm.abs());
                            }
                        }
                        if worst
                            .as_ref()
                            .is_none_or(|w| aisc.interaction_ratio > w.aisc.interaction_ratio)
                        {
                            worst = Some(StationCheck {
                                member: name.into(),
                                governing_xi: global_xi,
                                axial_n: -st.axial_n,
                                shear_n: st.shear_n,
                                moment_nm: st.moment_nm,
                                aisc,
                            });
                        }
                    }
                }
                if let Some(w) = worst {
                    checks.push(w);
                }
            }

            if !combo.is_serviceability {
                for base_idx in [0usize, nodes.len() - 1] {
                    let r_vec = sol.reaction[base_idx];
                    if r_vec[1] >= 0.0 {
                        max_base_comp_n = max_base_comp_n.max(r_vec[1]);
                    } else {
                        max_base_uplift_n = max_base_uplift_n.max(-r_vec[1]);
                    }
                    max_shear_n = max_shear_n.max(r_vec[0].abs());
                }
            }

            frame_results.push(FrameAnalysisResult {
                combination: combo.id.clone(),
                clause: combo.clause.clone(),
                is_serviceability: combo.is_serviceability,
                tributary_m: *trib,
                frame_label: frame_label.clone(),
                solution: sol,
                member_checks: checks,
                max_drift_x_m: max_drift_x,
                max_vertical_m: max_vert,
            });
        }
    }

    // Cold-formed purlin check on governing bay span (LRFD 1.2D+1.6S or 1.2D+1.6Lr, serviceability D+Lr)
    let max_bay = p.building.bay_lengths_m.iter().copied().fold(0.0, f64::max);
    let d_kpa = case_map["D"].left_roof_down_kpa;
    let lr_kpa = case_map["Lr"].left_roof_down_kpa;
    let s_kpa = case_map["S"].left_roof_down_kpa;
    let strength_roof_kpa = (1.2 * d_kpa + 1.6 * lr_kpa.max(s_kpa)).max(d_kpa + lr_kpa);
    let service_roof_kpa = d_kpa + lr_kpa;
    let purlin_check = aisi_s100_check_secondary(
        purlin_prof,
        max_bay,
        strength_roof_kpa * 1000.0 * p.building.purlin_spacing_m,
        service_roof_kpa * 1000.0 * p.building.purlin_spacing_m,
        p.hazard.design_method,
        180.0,
    );

    // Detailing: Connections, 3D parts, BOMCHECK, Clash detection
    let detail = build_detail_model(
        &p.building,
        col_sec,
        raf_sec,
        &p.database,
        &p.finish_code,
        p.anchor_scope,
        max_knee_moment_nm,
        max_ridge_moment_nm,
        max_base_comp_n,
        max_base_uplift_n,
        max_shear_n,
        p.hazard.design_method,
    );

    let max_conn_ratio = detail
        .end_plates
        .iter()
        .flat_map(|ep| [ep.bolt_utilization, ep.plate_utilization])
        .chain(
            detail
                .base_plates
                .iter()
                .flat_map(|bp| [bp.anchor_utilization, bp.plate_utilization]),
        )
        .fold(0.0, f64::max);

    // Group BOM
    let mut bom_map: BTreeMap<(String, String), BomSummaryLine> = BTreeMap::new();
    for part in &detail.parts {
        let cat = format!("{:?}", part.category);
        let key = (part.piece_mark.clone(), part.finish_code.clone());
        let entry = bom_map.entry(key).or_insert_with(|| BomSummaryLine {
            mark: part.piece_mark.clone(),
            category: cat,
            description: part.description.clone(),
            finish_code: part.finish_code.clone(),
            count: 0,
            length_each_m: part.length_m,
            total_mass_kg: 0.0,
        });
        entry.count += 1;
        entry.total_mass_kg += part.mass_kg;
    }
    let bom: Vec<BomSummaryLine> = bom_map.into_values().collect();

    // Finish-driven costing with DB_USER rule trace
    let primary_mass: f64 = detail
        .parts
        .iter()
        .filter(|pt| {
            matches!(
                pt.category,
                PartCategory::PrimaryColumn
                    | PartCategory::PrimaryRafter
                    | PartCategory::EndwallColumn
                    | PartCategory::ConnectionPlate
            )
        })
        .map(|pt| pt.mass_kg)
        .sum();
    let secondary_mass: f64 = detail
        .parts
        .iter()
        .filter(|pt| {
            !matches!(
                pt.category,
                PartCategory::PrimaryColumn
                    | PartCategory::PrimaryRafter
                    | PartCategory::EndwallColumn
                    | PartCategory::ConnectionPlate
            )
        })
        .map(|pt| pt.mass_kg)
        .sum();

    let (p_steel, p_fab, p_fin, trace_p) = resolve_cost_rates(
        &p.database,
        &p.finish_code,
        "RF",
        raf_sec.mass_per_m(raf_mat.density_kg_m3),
        p.base_rates.steel_minor_per_kg,
        p.base_rates.fabrication_minor_per_kg,
        p.base_rates.finish_minor_per_kg,
    );
    let panel_adder = p
        .database
        .finishes
        .iter()
        .find(|f| f.code == p.finish_code)
        .map(|f| f.panel_minor_per_m2_adder)
        .unwrap_or(0);

    let line =
        |desc: &str, qty: f64, unit: &str, rate: u64, trace: String| -> Result<TracedCostLine> {
            let milli = (qty * 1000.0).round() as u128;
            let tot = u64::try_from((milli * rate as u128 + 500) / 1000)
                .map_err(|_| "Cost line overflow")?;
            Ok(TracedCostLine {
                description: desc.into(),
                quantity: milli as f64 / 1000.0,
                unit: unit.into(),
                rate_minor: rate,
                total_minor: tot,
                rule_trace: trace,
            })
        };

    let env_area = p.building.gross_roof_area_m2() + p.building.net_wall_area_m2();
    let costs = vec![
        line(
            "Primary & Plate Steel Material",
            primary_mass,
            "kg",
            p_steel,
            trace_p.clone(),
        )?,
        line(
            "Primary Fabrication Allowance",
            primary_mass,
            "kg",
            p_fab,
            trace_p.clone(),
        )?,
        line("Primary Surface Finish", primary_mass, "kg", p_fin, trace_p)?,
        line(
            "Secondary, Bracing & Fasteners",
            secondary_mass,
            "kg",
            p.base_rates.steel_minor_per_kg,
            "base-secondary".into(),
        )?,
        line(
            "Envelope Roof & Wall Cladding",
            env_area,
            "m2",
            p.base_rates.panel_minor_per_m2 + panel_adder,
            format!("finish:{}", p.finish_code),
        )?,
    ];
    let subtotal_minor = costs.iter().try_fold(0u64, |acc, c| {
        acc.checked_add(c.total_minor)
            .ok_or("Cost subtotal overflow")
    })?;
    let markup_minor = u64::try_from(
        (subtotal_minor as u128 * p.base_rates.markup_basis_points as u128 + 5000) / 10000,
    )
    .map_err(|_| "Markup overflow")?;
    let total_minor = subtotal_minor
        .checked_add(markup_minor)
        .ok_or("Total overflow")?;

    let mut qa_findings = Vec::new();
    if max_aisc_ratio > 1.0 {
        qa_findings.push(format!(
            "AISC 360-16 primary frame unity ratio {:.3} exceeds 1.000",
            max_aisc_ratio
        ));
    }
    if max_drift_ratio > 1.0 {
        qa_findings.push(format!(
            "Serviceability lateral drift ratio {:.3} exceeds H/200 limit",
            max_drift_ratio
        ));
    }
    if purlin_check.moment_ratio > 1.0 || purlin_check.deflection_ratio > 1.0 {
        qa_findings.push(format!(
            "Cold-formed purlin {} check exceeded: M={:.3}, delta={:.3}",
            purlin_check.profile_id, purlin_check.moment_ratio, purlin_check.deflection_ratio
        ));
    }
    if max_conn_ratio > 1.0 {
        qa_findings.push(format!(
            "Connection bolt/plate/anchor utilization {:.3} exceeds 1.000",
            max_conn_ratio
        ));
    }
    if !detail.clashes.is_empty() {
        qa_findings.push(format!(
            "{} geometric clash(es) detected across model parts/openings",
            detail.clashes.len()
        ));
    }
    if !db_issues.is_empty() {
        qa_findings.push(format!("{} DBCHECK warning(s) logged", db_issues.len()));
    }
    if p.engineer_approved_by.is_none() {
        qa_findings.push("Production release blocked pending independent EOR sign-off".into());
    }

    let release_gates_passed = qa_findings.is_empty();

    Ok(EngineeringRun {
        engine: PEB_ENGINE.into(),
        revision: fingerprint_peb(p)?,
        load_cases,
        combinations,
        frames: frame_results,
        purlin_check,
        detail,
        db_issues,
        bom,
        costs,
        subtotal_minor,
        markup_minor,
        total_minor,
        max_aisc_ratio,
        max_drift_ratio,
        max_connection_ratio: max_conn_ratio,
        qa_findings,
        release_gates_passed,
    })
}

/// Searches built-up frame section pairs in the company database to select the lightest
/// column/rafter configuration passing AISC 360-16 (<= 1.0) and H/200 drift (<= 1.0).
pub fn optimize_peb_frame(p: &EngineeringProject) -> Result<(String, String)> {
    let mut cands = p.database.built_up_sections.clone();
    cands.sort_by(|a, b| a.area_at(0.5).total_cmp(&b.area_at(0.5)));
    for s in &cands {
        let mut trial = p.clone();
        trial.column_section_id = s.id.clone();
        trial.rafter_section_id = s.id.clone();
        if let Ok(run) = process_peb(&trial)
            && run.max_aisc_ratio <= 1.0
            && run.max_drift_ratio <= 1.0
        {
            return Ok((s.id.clone(), s.id.clone()));
        }
    }
    Err("No built-up frame section in database satisfies AISC 360-16 and H/200 drift".into())
}

/// Formats an MBS Verify-style plain-text AISC 360-16 engineering verification report.
pub fn format_verify_report(p: &EngineeringProject, r: &EngineeringRun) -> String {
    let mut s = String::with_capacity(4096);
    let _ = writeln!(
        s,
        "OPENMBS DESIGN VERIFICATION REPORT (AISC 360-16 / ASCE 7-22)"
    );
    let _ = writeln!(s, "Project: {} | Revision: {}", p.name, r.revision);
    let _ = writeln!(
        s,
        "Method: {:?} | P-Delta: {} | Max AISC Unity: {:.3} | Max Drift Ratio: {:.3}",
        p.hazard.design_method, p.second_order_pdelta, r.max_aisc_ratio, r.max_drift_ratio
    );
    let _ = writeln!(s, "{}", "=".repeat(88));
    for f in &r.frames {
        if f.is_serviceability {
            continue;
        }
        let _ = writeln!(
            s,
            "\n[{}] {} (trib = {:.2} m, P-Delta iters = {}, residual = {:.2e})",
            f.frame_label,
            f.combination,
            f.tributary_m,
            f.solution.second_order_iterations,
            f.solution.residual_relative
        );
        for c in &f.member_checks {
            let _ = writeln!(
                s,
                "  {:<13} @xi={:.2}: P={:>7.1} kN, V={:>6.1} kN, M={:>7.1} kNm | Unity={:.3} [{}]",
                c.member,
                c.governing_xi,
                c.axial_n / 1000.0,
                c.shear_n / 1000.0,
                c.moment_nm / 1000.0,
                c.aisc.interaction_ratio,
                c.aisc.governing_clause
            );
        }
    }
    let _ = writeln!(
        s,
        "\nPurlin {} Check (AISI S100-16): M_ratio={:.3}, Defl_ratio={:.3}",
        r.purlin_check.profile_id, r.purlin_check.moment_ratio, r.purlin_check.deflection_ratio
    );
    for ep in &r.detail.end_plates {
        let _ = writeln!(
            s,
            "Connection [{}]: {}x {}, tp={:.0}mm, bolt_util={:.3}, plate_util={:.3}",
            ep.location,
            ep.bolt_count,
            ep.bolt_id,
            ep.plate_thickness_m * 1000.0,
            ep.bolt_utilization,
            ep.plate_utilization
        );
    }
    s
}

/// Exports the full MBS-grade engineering, fabrication, drawing, QA, costing, and IFC4 bundle.
pub fn export_peb_bundle(p: &EngineeringProject, r: &EngineeringRun, path: &Path) -> Result<()> {
    if !r.is_current(p) {
        return Err("Engineering run is stale for the current inputs".into());
    }
    if path.exists() {
        return Err("Export directory exists; choose a new directory".into());
    }
    let parent = path
        .parent()
        .filter(|x| !x.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    let dir = temp.path();

    let mut bom_csv =
        "mark,category,finish,count,length_each_m,total_mass_kg,description\n".to_string();
    for line in &r.bom {
        let _ = writeln!(
            bom_csv,
            "{},{},{},{},{:.4},{:.3},\"{}\"",
            line.mark,
            line.category,
            line.finish_code,
            line.count,
            line.length_each_m,
            line.total_mass_kg,
            line.description.replace('"', "\"\"")
        );
    }

    let mut costs_csv =
        "description,quantity,unit,rate_minor,total_minor,currency,rule_trace\n".to_string();
    for c in &r.costs {
        let _ = writeln!(
            costs_csv,
            "\"{}\",{:.3},{},{},{},{},\"{}\"",
            c.description,
            c.quantity,
            c.unit,
            c.rate_minor,
            c.total_minor,
            p.base_rates.currency,
            c.rule_trace
        );
    }

    let mut bomcheck_csv =
        "category,mark,location,inventory,count,total_length_m,total_mass_kg,status\n".to_string();
    for bc in &r.detail.bomcheck {
        let _ = writeln!(
            bomcheck_csv,
            "{},{},{},{},{},{:.3},{:.3},{}",
            bc.category,
            bc.mark,
            bc.location_tag,
            bc.inventory_id,
            bc.count,
            bc.total_length_m,
            bc.total_mass_kg,
            bc.status
        );
    }

    let mut clash_csv = "clash_id,group_key,severity,part_a,part_b,description\n".to_string();
    for cl in &r.detail.clashes {
        let _ = writeln!(
            clash_csv,
            "{},{},{},{},{},\"{}\"",
            cl.clash_id,
            cl.group_key,
            cl.severity,
            cl.part_a,
            cl.part_b,
            cl.description.replace('"', "\"\"")
        );
    }

    let mut max_comp_kn: f64 = 0.0;
    let mut max_uplift_kn: f64 = 0.0;
    let mut max_shear_kn: f64 = 0.0;
    for f in r.frames.iter().filter(|fr| !fr.is_serviceability) {
        for base in [0, f.solution.reaction.len() - 1] {
            let rv = f.solution.reaction[base];
            max_comp_kn = max_comp_kn.max(rv[1] / 1000.0);
            max_uplift_kn = max_uplift_kn.max(-rv[1] / 1000.0);
            max_shear_kn = max_shear_kn.max(rv[0].abs() / 1000.0);
        }
    }

    let e1_svg = anchor_plan_svg(
        &p.name,
        &r.revision,
        &p.building,
        &r.detail,
        &p.drawing_control,
        max_comp_kn,
        max_uplift_kn,
        max_shear_kn,
    );
    let ifc = export_ifc4(&p.name, &r.detail);
    let verify_txt = format_verify_report(p, r);
    let cert_letter = openmbs_codes::generate_pe_certification_letter(
        &p.name,
        &r.revision,
        &p.building,
        &p.hazard,
        r.max_aisc_ratio,
        r.max_drift_ratio,
        "Structural Engineer of Record",
        "PE-54912-US",
    );

    let mut files: Vec<(String, Vec<u8>)> = vec![
        (
            "project.json".into(),
            serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?,
        ),
        (
            "engineering_run.json".into(),
            serde_json::to_vec_pretty(r).map_err(|e| e.to_string())?,
        ),
        ("bom.csv".into(), bom_csv.into_bytes()),
        ("costs.csv".into(), costs_csv.into_bytes()),
        ("bomcheck.csv".into(), bomcheck_csv.into_bytes()),
        ("clashes.csv".into(), clash_csv.into_bytes()),
        ("verify_report.txt".into(), verify_txt.into_bytes()),
        ("certification_letter.txt".into(), cert_letter.into_bytes()),
        ("sheet_E1_anchor_plan.svg".into(), e1_svg.into_bytes()),
        ("model.ifc".into(), ifc.into_bytes()),
    ];

    for mark in shop_sheet_marks(&r.detail) {
        let svg = shop_assembly_svg(&p.name, &mark, &r.detail, &p.drawing_control);
        files.push((format!("shop_{mark}.svg"), svg.into_bytes()));
        if let Some(sample) = r.detail.parts.iter().find(|pt| pt.piece_mark == mark) {
            files.push((
                format!("shop_{mark}.dxf"),
                piece_fabrication_dxf(sample).into_bytes(),
            ));
        }
    }

    let mut hashes = BTreeMap::new();
    for (fname, bytes) in files {
        hashes.insert(fname.clone(), format!("{:x}", Sha256::digest(&bytes)));
        atomic_write(&dir.join(&fname), &bytes)?;
    }
    atomic_write(
        &dir.join("manifest.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "engine": r.engine,
            "revision": r.revision,
            "release_gates_passed": r.release_gates_passed,
            "max_aisc_ratio": r.max_aisc_ratio,
            "sha256": hashes,
        }))
        .map_err(|e| e.to_string())?,
    )?;
    std::fs::rename(dir, path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_peb_pipeline_runs_and_optimizes_cleanly() {
        let mut proj = EngineeringProject::default();
        let (col, raf) = optimize_peb_frame(&proj).unwrap();
        proj.column_section_id = col;
        proj.rafter_section_id = raf;
        let run = process_peb(&proj).unwrap();
        assert!(run.max_aisc_ratio <= 1.0);
        assert!(run.max_drift_ratio <= 1.0);
        assert!(!run.bom.is_empty());
        assert!(!run.detail.parts.is_empty());
        assert!(run.costs.iter().all(|c| !c.rule_trace.is_empty()));
    }

    #[test]
    fn peb_bundle_writes_ifc_shop_dxf_and_verify() {
        let mut proj = EngineeringProject::default();
        let (col, raf) = optimize_peb_frame(&proj).unwrap();
        proj.column_section_id = col;
        proj.rafter_section_id = raf;
        let run = process_peb(&proj).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("peb-out");
        export_peb_bundle(&proj, &run, &out).unwrap();
        assert!(out.join("model.ifc").exists());
        assert!(out.join("sheet_E1_anchor_plan.svg").exists());
        assert!(out.join("verify_report.txt").exists());
        assert!(out.join("shop_RF1.svg").exists());
        assert!(out.join("shop_RF1.dxf").exists());
        assert!(out.join("bomcheck.csv").exists());
    }
}
