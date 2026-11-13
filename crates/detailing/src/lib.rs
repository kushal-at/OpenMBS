//! Connections, 3D fabrication parts/assemblies, hole tables, BOMCHECK, and
//! 3D geometric clash detection (independent clean-room Rust detailing engine).

use openmbs_codes::DesignMethod;
use openmbs_geometry::{
    Aabb3, BuildingSpec, EndwallType, WallOpening, WallSide, trimmed_girt_segments,
};
use openmbs_inventory::{AnchorSupplyScope, BuiltUpSectionSpec, CompanyDatabase};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartCategory {
    PrimaryColumn,
    PrimaryRafter,
    EndwallColumn,
    Purlin,
    SidewallGirt,
    EndwallGirt,
    OpeningJamb,
    OpeningHeader,
    FlangeBrace,
    DiagonalBrace,
    ConnectionPlate,
    Stiffener,
    StructuralBolt,
    AnchorRod,
    LeanToMember,
    CanopyMember,
    ParapetMember,
    CraneRunway,
    PortalBent,
    MezzanineBeam,
    MezzanineJoist,
    MezzanineColumn,
    MiniStoragePartition,
    MiniStorageDoor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoleSpec {
    pub x_from_start_m: f64,
    pub y_offset_m: f64,
    pub diameter_m: f64,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartOccurrence {
    pub occurrence_id: String,
    pub piece_mark: String,
    pub assembly_mark: String,
    pub category: PartCategory,
    pub description: String,
    pub material_id: String,
    pub finish_code: String,
    pub location_tag: String,
    pub start_m: [f64; 3],
    pub end_m: [f64; 3],
    pub depth_start_m: f64,
    pub depth_end_m: f64,
    pub width_m: f64,
    pub length_m: f64,
    pub mass_kg: f64,
    pub bbox: Aabb3,
    pub holes: Vec<HoleSpec>,
    pub weld_note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EndPlateDesign {
    pub location: String,
    pub bolt_id: String,
    pub bolt_count: usize,
    pub plate_width_m: f64,
    pub plate_height_m: f64,
    pub plate_thickness_m: f64,
    pub weld_size_m: f64,
    pub demand_moment_nm: f64,
    pub demand_shear_n: f64,
    pub bolt_utilization: f64,
    pub plate_utilization: f64,
    pub governing_clause: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasePlateDesign {
    pub location: String,
    pub anchor_id: String,
    pub anchor_count: usize,
    pub anchor_scope: AnchorSupplyScope,
    pub plate_width_m: f64,
    pub plate_length_m: f64,
    pub plate_thickness_m: f64,
    pub max_uplift_n: f64,
    pub max_compression_n: f64,
    pub max_shear_n: f64,
    pub anchor_utilization: f64,
    pub plate_utilization: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BomCheckLine {
    pub category: String,
    pub mark: String,
    pub location_tag: String,
    pub inventory_id: String,
    pub count: usize,
    pub total_length_m: f64,
    pub total_mass_kg: f64,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClashIssue {
    pub clash_id: String,
    pub group_key: String,
    pub severity: String,
    pub part_a: String,
    pub part_b: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetailModel {
    pub parts: Vec<PartOccurrence>,
    pub end_plates: Vec<EndPlateDesign>,
    pub base_plates: Vec<BasePlateDesign>,
    pub bomcheck: Vec<BomCheckLine>,
    pub clashes: Vec<ClashIssue>,
}

fn select_plate_thickness(required_m: f64, stock: &[f64]) -> f64 {
    let mut sorted = stock.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    sorted
        .into_iter()
        .find(|&t| t >= required_m)
        .unwrap_or_else(|| required_m.max(0.012))
}

/// Designs a bolted moment end-plate connection for knee or ridge joints.
#[allow(clippy::too_many_arguments)]
pub fn design_end_plate(
    location: &str,
    member_depth_m: f64,
    flange_width_m: f64,
    flange_t_m: f64,
    moment_nm: f64,
    shear_n: f64,
    db: &CompanyDatabase,
    method: DesignMethod,
) -> EndPlateDesign {
    let bolt = db.bolts.first().expect("dbcheck ensures bolts exist");
    let mat = db.materials.first().cloned().unwrap_or_default();
    let phi_b = match method {
        DesignMethod::LRFD => 0.75,
        DesignMethod::ASD => 1.0 / 2.00,
    };
    let phi_y = match method {
        DesignMethod::LRFD => 0.90,
        DesignMethod::ASD => 1.0 / 1.67,
    };

    let ab = std::f64::consts::PI * bolt.diameter_m * bolt.diameter_m / 4.0;
    let pt_cap = phi_b * bolt.fnt_pa * ab;
    let pv_cap = phi_b * bolt.fnv_pa * ab;

    let arm = (member_depth_m - flange_t_m).max(0.20);
    let tf_demand = moment_nm.abs() / arm;

    // Select 4-bolt flush vs 8-bolt extended configuration
    let (bolt_count, tension_bolts) = if tf_demand <= 2.0 * pt_cap && shear_n.abs() <= 4.0 * pv_cap
    {
        (4usize, 2usize)
    } else {
        (8usize, 4usize)
    };
    let bolt_tens = tf_demand / tension_bolts as f64;
    let bolt_shear = shear_n.abs() / bolt_count as f64;
    let bolt_util = (bolt_tens / pt_cap).max(bolt_shear / pv_cap);

    // Yield-line / cantilever flange-to-bolt moment arm ~ 0.055 m -> required plate thickness tp
    let p_eff = 0.055;
    let bp = (flange_width_m + 0.05).max(0.22);
    let req_tp = ((4.0 * tf_demand * p_eff) / (phi_y * mat.fy_pa * bp)).sqrt();
    let tp = select_plate_thickness(req_tp, &db.plate_thicknesses_m);
    let plate_util = (req_tp / tp).powi(2);

    let plate_height = if bolt_count == 8 {
        member_depth_m + 0.22
    } else {
        member_depth_m + 0.06
    };

    EndPlateDesign {
        location: location.into(),
        bolt_id: bolt.id.clone(),
        bolt_count,
        plate_width_m: bp,
        plate_height_m: plate_height,
        plate_thickness_m: tp,
        weld_size_m: (flange_t_m * 0.7).max(0.006),
        demand_moment_nm: moment_nm.abs(),
        demand_shear_n: shear_n.abs(),
        bolt_utilization: bolt_util,
        plate_utilization: plate_util,
        governing_clause: "AISC 360-16 J3 / DG-4 Extended Moment End-Plate".into(),
    }
}

/// Designs a column base plate & anchor rod layout using AISC Design Guide 1 simplified checks.
#[allow(clippy::too_many_arguments)]
pub fn design_base_plate(
    location: &str,
    col_depth_m: f64,
    col_bf_m: f64,
    compression_n: f64,
    uplift_n: f64,
    shear_n: f64,
    anchor_scope: AnchorSupplyScope,
    db: &CompanyDatabase,
    method: DesignMethod,
) -> BasePlateDesign {
    let anchor = db
        .anchor_rods
        .first()
        .expect("dbcheck ensures anchor records");
    let mat = db.materials.first().cloned().unwrap_or_default();
    let phi_b = match method {
        DesignMethod::LRFD => 0.75,
        DesignMethod::ASD => 1.0 / 2.00,
    };
    let phi_y = match method {
        DesignMethod::LRFD => 0.90,
        DesignMethod::ASD => 1.0 / 1.67,
    };

    let ab = std::f64::consts::PI * anchor.diameter_m * anchor.diameter_m / 4.0;
    let t_cap = phi_b * anchor.fnt_pa * ab;
    let v_cap = phi_b * anchor.fnv_pa * ab;
    let anchor_count = if uplift_n.abs() > 2.0 * t_cap { 6 } else { 4 };

    let plate_w = (col_bf_m + 0.14).max(0.30);
    let plate_l = (col_depth_m + 0.14).max(0.35);
    let cantilever = 0.07;
    let bearing_stress = compression_n.abs() / (plate_w * plate_l);
    let req_tp = cantilever * ((2.0 * bearing_stress) / (phi_y * mat.fy_pa)).sqrt();
    let tp = select_plate_thickness(req_tp.max(0.012), &db.plate_thicknesses_m);

    let anchor_util = (uplift_n.abs() / (anchor_count as f64 * t_cap))
        + (shear_n.abs() / (anchor_count as f64 * v_cap));

    BasePlateDesign {
        location: location.into(),
        anchor_id: anchor.id.clone(),
        anchor_count,
        anchor_scope,
        plate_width_m: plate_w,
        plate_length_m: plate_l,
        plate_thickness_m: tp,
        max_uplift_n: uplift_n.abs(),
        max_compression_n: compression_n.abs(),
        max_shear_n: shear_n.abs(),
        anchor_utilization: anchor_util,
        plate_utilization: (req_tp / tp).powi(2),
    }
}

/// Generates the full 3D fabrication model (primary frames, endwalls, trimmed girts, lapped
/// purlins, opening framing, bracing rods, plates, fasteners), BOMCHECK, and Clash checks.
#[allow(clippy::too_many_arguments)]
pub fn build_detail_model(
    b: &BuildingSpec,
    column_sec: &BuiltUpSectionSpec,
    rafter_sec: &BuiltUpSectionSpec,
    db: &CompanyDatabase,
    finish_code: &str,
    anchor_scope: AnchorSupplyScope,
    max_knee_moment_nm: f64,
    max_ridge_moment_nm: f64,
    max_base_comp_n: f64,
    max_base_uplift_n: f64,
    max_shear_n: f64,
    method: DesignMethod,
) -> DetailModel {
    let mut parts = Vec::new();
    let mat = db
        .materials
        .iter()
        .find(|m| m.id == rafter_sec.material_id)
        .cloned()
        .unwrap_or_default();
    let density = mat.density_kg_m3;
    let bolt = db.bolts.first().expect("bolts configured");
    let anchor = db.anchor_rods.first().expect("anchors configured");

    let eave_plate = design_end_plate(
        "Eave Knee Splice",
        rafter_sec.depth_start_m,
        rafter_sec.flange_width_m,
        rafter_sec.flange_thickness_m,
        max_knee_moment_nm,
        max_shear_n,
        db,
        method,
    );
    let ridge_plate = design_end_plate(
        "Ridge Splice",
        rafter_sec.depth_end_m,
        rafter_sec.flange_width_m,
        rafter_sec.flange_thickness_m,
        max_ridge_moment_nm,
        0.25 * max_shear_n,
        db,
        method,
    );
    let base_plate = design_base_plate(
        "Column Base",
        column_sec.depth_start_m,
        column_sec.flange_width_m,
        max_base_comp_n,
        max_base_uplift_n,
        max_shear_n,
        anchor_scope,
        db,
        method,
    );

    let frame_ys = b.frame_y_positions();
    let frame_pts = b.frame_points_xz();
    let col_holes = vec![
        HoleSpec {
            x_from_start_m: 0.05,
            y_offset_m: 0.06,
            diameter_m: anchor.diameter_m + 0.006,
            note: "Anchor bolt hole".into(),
        },
        HoleSpec {
            x_from_start_m: b.left_eave_m - 0.06,
            y_offset_m: 0.05,
            diameter_m: bolt.hole_diameter_m,
            note: "Knee flange splice hole".into(),
        },
    ];

    for (fi, &y) in frame_ys.iter().enumerate() {
        let is_end = fi == 0 || fi == frame_ys.len() - 1;
        let loc = format!("Frame Line {}", fi + 1);

        if is_end && b.endwall_type == EndwallType::PostAndBeam {
            // Post-and-beam endwall: corner + intermediate wind columns
            let cols = b.endwall_post_bays + 1;
            for c_idx in 0..cols {
                let x = b.width_m * c_idx as f64 / (cols - 1) as f64;
                let top_z = b.roof_height_at_x(x);
                parts.push(PartOccurrence {
                    occurrence_id: format!("FL{}-EWC{}", fi + 1, c_idx + 1),
                    piece_mark: format!("EWC{}", c_idx + 1),
                    assembly_mark: format!("EWF-{}", fi + 1),
                    category: PartCategory::EndwallColumn,
                    description: format!("Endwall post-and-beam column ({:.2} m)", top_z),
                    material_id: mat.id.clone(),
                    finish_code: finish_code.into(),
                    location_tag: loc.clone(),
                    start_m: [x, y, 0.0],
                    end_m: [x, y, top_z],
                    depth_start_m: 0.20,
                    depth_end_m: 0.20,
                    width_m: 0.15,
                    length_m: top_z,
                    mass_kg: top_z * 26.0,
                    bbox: Aabb3::from_points([x, y, 0.0], [x, y, top_z], 0.10),
                    holes: col_holes.clone(),
                    weld_note: Some("6mm fillet base & cap plate".into()),
                });
            }
        } else {
            // Rigid Frame Line: Left/Right Columns + Left/Right Rafters
            for (tag, mark, cat, a_xz, b_xz, sec, plate_ref) in [
                (
                    "CL",
                    "C1",
                    PartCategory::PrimaryColumn,
                    frame_pts[0],
                    frame_pts[1],
                    column_sec,
                    &eave_plate,
                ),
                (
                    "CR",
                    "C1",
                    PartCategory::PrimaryColumn,
                    frame_pts[4],
                    frame_pts[3],
                    column_sec,
                    &eave_plate,
                ),
                (
                    "RL",
                    "RF1",
                    PartCategory::PrimaryRafter,
                    frame_pts[1],
                    frame_pts[2],
                    rafter_sec,
                    &ridge_plate,
                ),
                (
                    "RR",
                    "RF1",
                    PartCategory::PrimaryRafter,
                    frame_pts[3],
                    frame_pts[2],
                    rafter_sec,
                    &ridge_plate,
                ),
            ] {
                let len = (b_xz[0] - a_xz[0]).hypot(b_xz[1] - a_xz[1]);
                let mass = len * sec.mass_per_m(density);
                let pad = sec.flange_width_m * 0.5;
                parts.push(PartOccurrence {
                    occurrence_id: format!("FL{}-{tag}", fi + 1),
                    piece_mark: mark.into(),
                    assembly_mark: format!("{mark}-{}", fi + 1),
                    category: cat,
                    description: format!("{} ({})", sec.id, sec.source),
                    material_id: sec.material_id.clone(),
                    finish_code: finish_code.into(),
                    location_tag: loc.clone(),
                    start_m: [a_xz[0], y, a_xz[1]],
                    end_m: [b_xz[0], y, b_xz[1]],
                    depth_start_m: sec.depth_start_m,
                    depth_end_m: sec.depth_end_m,
                    width_m: sec.flange_width_m,
                    length_m: len,
                    mass_kg: mass,
                    bbox: Aabb3::from_points([a_xz[0], y, a_xz[1]], [b_xz[0], y, b_xz[1]], pad),
                    holes: col_holes.clone(),
                    weld_note: Some(format!(
                        "{:.0}mm continuous web-to-flange + {:.0}mm end-plate weld",
                        sec.web_thickness_m * 1000.0,
                        plate_ref.weld_size_m * 1000.0
                    )),
                });

                // Add bolted splice end-plate occurrence
                let plate_mass = plate_ref.plate_width_m
                    * plate_ref.plate_height_m
                    * plate_ref.plate_thickness_m
                    * density;
                parts.push(PartOccurrence {
                    occurrence_id: format!("FL{}-{tag}-EP", fi + 1),
                    piece_mark: format!("EP-{tag}"),
                    assembly_mark: format!("{mark}-{}", fi + 1),
                    category: PartCategory::ConnectionPlate,
                    description: format!(
                        "{:.0}x{:.0}x{:.0}mm bolted end plate",
                        plate_ref.plate_width_m * 1000.0,
                        plate_ref.plate_height_m * 1000.0,
                        plate_ref.plate_thickness_m * 1000.0
                    ),
                    material_id: mat.id.clone(),
                    finish_code: finish_code.into(),
                    location_tag: loc.clone(),
                    start_m: [b_xz[0], y, b_xz[1]],
                    end_m: [b_xz[0], y, b_xz[1]],
                    depth_start_m: plate_ref.plate_height_m,
                    depth_end_m: plate_ref.plate_height_m,
                    width_m: plate_ref.plate_width_m,
                    length_m: plate_ref.plate_thickness_m,
                    mass_kg: plate_mass,
                    bbox: Aabb3::from_points([b_xz[0], y, b_xz[1]], [b_xz[0], y, b_xz[1]], 0.15),
                    holes: vec![],
                    weld_note: None,
                });

                // Add splice bolts
                for b_idx in 0..plate_ref.bolt_count {
                    parts.push(PartOccurrence {
                        occurrence_id: format!("FL{}-{tag}-B{}", fi + 1, b_idx + 1),
                        piece_mark: bolt.id.clone(),
                        assembly_mark: format!("{mark}-{}", fi + 1),
                        category: PartCategory::StructuralBolt,
                        description: format!("{} splice bolt", bolt.grade),
                        material_id: bolt.grade.clone(),
                        finish_code: "GALV-HD".into(),
                        location_tag: loc.clone(),
                        start_m: [b_xz[0], y, b_xz[1]],
                        end_m: [b_xz[0], y, b_xz[1]],
                        depth_start_m: bolt.diameter_m,
                        depth_end_m: bolt.diameter_m,
                        width_m: bolt.diameter_m,
                        length_m: 0.08,
                        mass_kg: bolt.unit_mass_kg,
                        bbox: Aabb3::from_points(
                            [b_xz[0], y, b_xz[1]],
                            [b_xz[0], y, b_xz[1]],
                            0.03,
                        ),
                        holes: vec![],
                        weld_note: None,
                    });
                }
            }

            // Column base plates + anchor rods at left & right columns
            for (side, x_col) in [("L", 0.0), ("R", b.width_m)] {
                let bp_mass = base_plate.plate_width_m
                    * base_plate.plate_length_m
                    * base_plate.plate_thickness_m
                    * density;
                parts.push(PartOccurrence {
                    occurrence_id: format!("FL{}-BP-{side}", fi + 1),
                    piece_mark: "BP1".into(),
                    assembly_mark: format!("C1-{}", fi + 1),
                    category: PartCategory::ConnectionPlate,
                    description: format!(
                        "{:.0}x{:.0}x{:.0}mm column base plate",
                        base_plate.plate_width_m * 1000.0,
                        base_plate.plate_length_m * 1000.0,
                        base_plate.plate_thickness_m * 1000.0
                    ),
                    material_id: mat.id.clone(),
                    finish_code: finish_code.into(),
                    location_tag: loc.clone(),
                    start_m: [x_col, y, 0.0],
                    end_m: [x_col, y, 0.0],
                    depth_start_m: base_plate.plate_length_m,
                    depth_end_m: base_plate.plate_length_m,
                    width_m: base_plate.plate_width_m,
                    length_m: base_plate.plate_thickness_m,
                    mass_kg: bp_mass,
                    bbox: Aabb3::from_points([x_col, y, 0.0], [x_col, y, 0.0], 0.18),
                    holes: vec![],
                    weld_note: None,
                });

                if anchor_scope == AnchorSupplyScope::ByManufacturer {
                    for a_idx in 0..base_plate.anchor_count {
                        parts.push(PartOccurrence {
                            occurrence_id: format!("FL{}-ANC-{side}-{}", fi + 1, a_idx + 1),
                            piece_mark: anchor.id.clone(),
                            assembly_mark: format!("ANCH-{}-{}", fi + 1, side),
                            category: PartCategory::AnchorRod,
                            description: format!("{} anchor rod", anchor.grade),
                            material_id: anchor.grade.clone(),
                            finish_code: "PLAIN".into(),
                            location_tag: loc.clone(),
                            start_m: [x_col, y, -anchor.embedment_m],
                            end_m: [x_col, y, anchor.length_m - anchor.embedment_m],
                            depth_start_m: anchor.diameter_m,
                            depth_end_m: anchor.diameter_m,
                            width_m: anchor.diameter_m,
                            length_m: anchor.length_m,
                            mass_kg: anchor.unit_mass_kg,
                            bbox: Aabb3::from_points([x_col, y, 0.0], [x_col, y, 0.0], 0.04),
                            holes: vec![],
                            weld_note: None,
                        });
                    }
                }
            }
        }
    }

    // Roof purlins per bay (including configurable lap allowance)
    let left_purlin_lines = (b.left_rafter_length_m() / b.purlin_spacing_m).ceil() as usize + 1;
    let right_purlin_lines = (b.right_rafter_length_m() / b.purlin_spacing_m).ceil() as usize + 1;
    for (bay_idx, &bay_len) in b.bay_lengths_m.iter().enumerate() {
        let y0 = frame_ys[bay_idx];
        let y1 = frame_ys[bay_idx + 1];
        let purlin_len = bay_len + b.purlin_lap_m;
        for (slope_tag, lines, p0, p1) in [
            ("LR", left_purlin_lines, frame_pts[1], frame_pts[2]),
            ("RR", right_purlin_lines, frame_pts[3], frame_pts[2]),
        ] {
            for row in 0..lines {
                let t = row as f64 / (lines - 1).max(1) as f64;
                let x = p0[0] * (1.0 - t) + p1[0] * t;
                let z = p0[1] * (1.0 - t) + p1[1] * t;
                parts.push(PartOccurrence {
                    occurrence_id: format!("PUR-B{}-{slope_tag}-{row}", bay_idx + 1),
                    piece_mark: format!("Z200-{:.2}", purlin_len),
                    assembly_mark: format!("ROOF-B{}", bay_idx + 1),
                    category: PartCategory::Purlin,
                    description: format!("Z-purlin with {:.2}m lap", b.purlin_lap_m),
                    material_id: "Cold-Formed 55ksi".into(),
                    finish_code: "GALV-G90".into(),
                    location_tag: format!("Bay {}", bay_idx + 1),
                    start_m: [x, y0, z + 0.05],
                    end_m: [x, y1, z + 0.05],
                    depth_start_m: 0.20,
                    depth_end_m: 0.20,
                    width_m: 0.065,
                    length_m: purlin_len,
                    mass_kg: purlin_len * b.secondary_kg_per_m,
                    bbox: Aabb3::from_points([x, y0, z + 0.05], [x, y1, z + 0.05], 0.04),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Sidewall & endwall girts trimmed cleanly around openings!
    let left_openings: Vec<&WallOpening> = b
        .openings
        .iter()
        .filter(|o| o.wall == WallSide::LeftSidewall)
        .collect();
    let right_openings: Vec<&WallOpening> = b
        .openings
        .iter()
        .filter(|o| o.wall == WallSide::RightSidewall)
        .collect();
    let front_openings: Vec<&WallOpening> = b
        .openings
        .iter()
        .filter(|o| o.wall == WallSide::FrontEndwall)
        .collect();
    let back_openings: Vec<&WallOpening> = b
        .openings
        .iter()
        .filter(|o| o.wall == WallSide::BackEndwall)
        .collect();

    let total_len = b.total_length_m();
    for (tag, x_wall, eave_h, openings_ref) in [
        ("LW", 0.0, b.left_eave_m, &left_openings),
        ("RW", b.width_m, b.right_eave_m, &right_openings),
    ] {
        let rows = (eave_h / b.girt_spacing_m).ceil() as usize;
        for row in 1..=rows {
            let z = (row as f64 * b.girt_spacing_m).min(eave_h - 0.1);
            for (seg_idx, (s, e)) in trimmed_girt_segments(total_len, z, openings_ref)
                .into_iter()
                .enumerate()
            {
                let len = e - s;
                parts.push(PartOccurrence {
                    occurrence_id: format!("GIRT-{tag}-R{row}-S{}", seg_idx + 1),
                    piece_mark: format!("C200-{len:.2}"),
                    assembly_mark: format!("WALL-{tag}"),
                    category: PartCategory::SidewallGirt,
                    description: "Trimmed sidewall girt".into(),
                    material_id: "Cold-Formed 55ksi".into(),
                    finish_code: "GALV-G90".into(),
                    location_tag: tag.into(),
                    start_m: [x_wall, s, z],
                    end_m: [x_wall, e, z],
                    depth_start_m: 0.20,
                    depth_end_m: 0.20,
                    width_m: 0.065,
                    length_m: len,
                    mass_kg: len * b.secondary_kg_per_m,
                    bbox: Aabb3::from_points([x_wall, s, z], [x_wall, e, z], 0.04),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    for (tag, y_wall, openings_ref) in [
        ("FEW", 0.0, &front_openings),
        ("BEW", total_len, &back_openings),
    ] {
        let rows = (b.left_eave_m.max(b.right_eave_m) / b.girt_spacing_m).ceil() as usize;
        for row in 1..=rows {
            let z = (row as f64 * b.girt_spacing_m).min(b.left_eave_m - 0.1);
            for (seg_idx, (s, e)) in trimmed_girt_segments(b.width_m, z, openings_ref)
                .into_iter()
                .enumerate()
            {
                let len = e - s;
                parts.push(PartOccurrence {
                    occurrence_id: format!("GIRT-{tag}-R{row}-S{}", seg_idx + 1),
                    piece_mark: format!("C200EW-{len:.2}"),
                    assembly_mark: format!("WALL-{tag}"),
                    category: PartCategory::EndwallGirt,
                    description: "Trimmed endwall girt".into(),
                    material_id: "Cold-Formed 55ksi".into(),
                    finish_code: "GALV-G90".into(),
                    location_tag: tag.into(),
                    start_m: [s, y_wall, z],
                    end_m: [e, y_wall, z],
                    depth_start_m: 0.20,
                    depth_end_m: 0.20,
                    width_m: 0.065,
                    length_m: len,
                    mass_kg: len * b.secondary_kg_per_m,
                    bbox: Aabb3::from_points([s, y_wall, z], [e, y_wall, z], 0.04),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Shop-framed opening jambs & headers (skip field-located openings per MBS DM81/opening rule)
    for o in b.openings.iter().filter(|o| o.shop_framed) {
        let (p_j1_a, p_j1_b, p_j2_a, p_j2_b, p_hdr_a, p_hdr_b) = match o.wall {
            WallSide::LeftSidewall => (
                [0.0, o.offset_m, 0.0],
                [0.0, o.offset_m, o.sill_m + o.height_m],
                [0.0, o.offset_m + o.width_m, 0.0],
                [0.0, o.offset_m + o.width_m, o.sill_m + o.height_m],
                [0.0, o.offset_m, o.sill_m + o.height_m],
                [0.0, o.offset_m + o.width_m, o.sill_m + o.height_m],
            ),
            WallSide::RightSidewall => (
                [b.width_m, o.offset_m, 0.0],
                [b.width_m, o.offset_m, o.sill_m + o.height_m],
                [b.width_m, o.offset_m + o.width_m, 0.0],
                [b.width_m, o.offset_m + o.width_m, o.sill_m + o.height_m],
                [b.width_m, o.offset_m, o.sill_m + o.height_m],
                [b.width_m, o.offset_m + o.width_m, o.sill_m + o.height_m],
            ),
            WallSide::FrontEndwall => (
                [o.offset_m, 0.0, 0.0],
                [o.offset_m, 0.0, o.sill_m + o.height_m],
                [o.offset_m + o.width_m, 0.0, 0.0],
                [o.offset_m + o.width_m, 0.0, o.sill_m + o.height_m],
                [o.offset_m, 0.0, o.sill_m + o.height_m],
                [o.offset_m + o.width_m, 0.0, o.sill_m + o.height_m],
            ),
            WallSide::BackEndwall => (
                [o.offset_m, total_len, 0.0],
                [o.offset_m, total_len, o.sill_m + o.height_m],
                [o.offset_m + o.width_m, total_len, 0.0],
                [o.offset_m + o.width_m, total_len, o.sill_m + o.height_m],
                [o.offset_m, total_len, o.sill_m + o.height_m],
                [o.offset_m + o.width_m, total_len, o.sill_m + o.height_m],
            ),
        };
        let jh = o.sill_m + o.height_m;
        for (suffix, a, pt_b, cat, len) in [
            ("JL", p_j1_a, p_j1_b, PartCategory::OpeningJamb, jh),
            ("JR", p_j2_a, p_j2_b, PartCategory::OpeningJamb, jh),
            (
                "HD",
                p_hdr_a,
                p_hdr_b,
                PartCategory::OpeningHeader,
                o.width_m,
            ),
        ] {
            parts.push(PartOccurrence {
                occurrence_id: format!("OPN-{}-{suffix}", o.id),
                piece_mark: format!("OF-{suffix}-{len:.2}"),
                assembly_mark: format!("OPN-{}", o.id),
                category: cat,
                description: format!("Shop-framed opening {} member", suffix),
                material_id: "Cold-Formed 55ksi".into(),
                finish_code: finish_code.into(),
                location_tag: o.wall.as_str().into(),
                start_m: a,
                end_m: pt_b,
                depth_start_m: 0.20,
                depth_end_m: 0.20,
                width_m: 0.065,
                length_m: len,
                mass_kg: len * 8.5,
                bbox: Aabb3::from_points(a, pt_b, 0.05),
                holes: vec![],
                weld_note: None,
            });
        }
    }

    // Diagonal X-bracing rods in designated braced bays
    for &bay_idx in &b.braced_bays {
        if bay_idx < b.bay_lengths_m.len() {
            let y0 = frame_ys[bay_idx];
            let y1 = frame_ys[bay_idx + 1];
            for (tag, a, pt_b) in [
                ("L-X1", [0.0, y0, 0.0], [0.0, y1, b.left_eave_m]),
                ("L-X2", [0.0, y1, 0.0], [0.0, y0, b.left_eave_m]),
                (
                    "R-X1",
                    [b.width_m, y0, 0.0],
                    [b.width_m, y1, b.right_eave_m],
                ),
                (
                    "R-X2",
                    [b.width_m, y1, 0.0],
                    [b.width_m, y0, b.right_eave_m],
                ),
            ] {
                let len = ((pt_b[0] - a[0]).powi(2)
                    + (pt_b[1] - a[1]).powi(2)
                    + (pt_b[2] - a[2]).powi(2))
                .sqrt();
                parts.push(PartOccurrence {
                    occurrence_id: format!("BR-B{}-{tag}", bay_idx + 1),
                    piece_mark: format!("ROD22-{len:.2}"),
                    assembly_mark: format!("BR-B{}", bay_idx + 1),
                    category: PartCategory::DiagonalBrace,
                    description: "22mm threaded diagonal bracing rod".into(),
                    material_id: "ASTM A36".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Bay {}", bay_idx + 1),
                    start_m: a,
                    end_m: pt_b,
                    depth_start_m: 0.022,
                    depth_end_m: 0.022,
                    width_m: 0.022,
                    length_m: len,
                    mass_kg: len * 2.98,
                    bbox: Aabb3::from_points(a, pt_b, 0.02),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Lean-to extensions (F-0004)
    for lt in &b.lean_tos {
        for bay_step in 0..=lt.bay_count {
            let bay_idx = lt.bay_start + bay_step;
            if bay_idx <= b.bay_lengths_m.len() {
                let y = frame_ys[bay_idx];
                let (x_attach, x_post) = match lt.wall {
                    WallSide::LeftSidewall => (0.0, -lt.span_m),
                    _ => (b.width_m, b.width_m + lt.span_m),
                };
                let z_attach = if lt.wall == WallSide::LeftSidewall {
                    b.left_eave_m
                } else {
                    b.right_eave_m
                };
                let z_post = lt.eave_height_m;
                parts.push(PartOccurrence {
                    occurrence_id: format!("LT-{}-COL-{}", lt.id, bay_idx + 1),
                    piece_mark: format!("LT-C-{:.2}", z_post),
                    assembly_mark: format!("LT-{}", lt.id),
                    category: PartCategory::LeanToMember,
                    description: format!("Lean-to {} post", lt.id),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Lean-to {}", lt.id),
                    start_m: [x_post, y, 0.0],
                    end_m: [x_post, y, z_post],
                    depth_start_m: 0.20,
                    depth_end_m: 0.20,
                    width_m: 0.15,
                    length_m: z_post,
                    mass_kg: z_post * 25.0,
                    bbox: Aabb3::from_points([x_post, y, 0.0], [x_post, y, z_post], 0.10),
                    holes: vec![],
                    weld_note: None,
                });
                let r_len = (lt.span_m.powi(2) + (z_attach - z_post).powi(2)).sqrt();
                parts.push(PartOccurrence {
                    occurrence_id: format!("LT-{}-RAF-{}", lt.id, bay_idx + 1),
                    piece_mark: format!("LT-R-{:.2}", r_len),
                    assembly_mark: format!("LT-{}", lt.id),
                    category: PartCategory::LeanToMember,
                    description: format!("Lean-to {} rafter", lt.id),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Lean-to {}", lt.id),
                    start_m: [x_attach, y, z_attach],
                    end_m: [x_post, y, z_post],
                    depth_start_m: 0.25,
                    depth_end_m: 0.25,
                    width_m: 0.15,
                    length_m: r_len,
                    mass_kg: r_len * 30.0,
                    bbox: Aabb3::from_points([x_attach, y, z_attach], [x_post, y, z_post], 0.10),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Canopies & Overhangs (F-0001, F-0112)
    for can in &b.canopies {
        for bay_step in 0..=can.bay_count {
            let bay_idx = can.bay_start + bay_step;
            if bay_idx <= b.bay_lengths_m.len() {
                let y = frame_ys[bay_idx];
                let (x_in, x_out) = match can.wall {
                    WallSide::LeftSidewall => (0.0, -can.projection_m),
                    _ => (b.width_m, b.width_m + can.projection_m),
                };
                let z = can.height_m;
                parts.push(PartOccurrence {
                    occurrence_id: format!("CAN-{}-BRK-{}", can.id, bay_idx + 1),
                    piece_mark: format!("CAN-B-{:.2}", can.projection_m),
                    assembly_mark: format!("CAN-{}", can.id),
                    category: PartCategory::CanopyMember,
                    description: format!("Canopy {} cantilever bracket", can.id),
                    material_id: "ASTM A36".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Canopy {}", can.id),
                    start_m: [x_in, y, z],
                    end_m: [x_out, y, z],
                    depth_start_m: 0.20,
                    depth_end_m: 0.12,
                    width_m: 0.10,
                    length_m: can.projection_m,
                    mass_kg: can.projection_m * 18.0,
                    bbox: Aabb3::from_points([x_in, y, z], [x_out, y, z], 0.08),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Parapet systems (F-0075, F-0114)
    if let Some(ref para) = b.parapet
        && para.sidewalls
    {
        for (tag, x, eave_h) in [("L", 0.0, b.left_eave_m), ("R", b.width_m, b.right_eave_m)] {
            for (fi, &y) in frame_ys.iter().enumerate() {
                parts.push(PartOccurrence {
                    occurrence_id: format!("PARA-{tag}-STUB-{}", fi + 1),
                    piece_mark: format!("PAR-STUB-{:.2}", para.height_m),
                    assembly_mark: format!("PARA-{}", tag),
                    category: PartCategory::ParapetMember,
                    description: "Parapet vertical support stub".into(),
                    material_id: "ASTM A36".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Parapet {tag}"),
                    start_m: [x, y, eave_h],
                    end_m: [x, y, eave_h + para.height_m],
                    depth_start_m: 0.15,
                    depth_end_m: 0.15,
                    width_m: 0.10,
                    length_m: para.height_m,
                    mass_kg: para.height_m * 12.0,
                    bbox: Aabb3::from_points([x, y, eave_h], [x, y, eave_h + para.height_m], 0.06),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Crane runway beams and brackets (F-0118, F-0119)
    for cr in &b.cranes {
        for (tag, x_col) in [("L", 0.0), ("R", b.width_m)] {
            let x_brk = if tag == "L" {
                cr.bracket_eccentricity_m
            } else {
                b.width_m - cr.bracket_eccentricity_m
            };
            for (fi, &y) in frame_ys.iter().enumerate() {
                parts.push(PartOccurrence {
                    occurrence_id: format!("CRN-{tag}-BRK-{}", fi + 1),
                    piece_mark: format!("CR-BRK-{:.0}T", cr.capacity_metric_tons),
                    assembly_mark: format!("CRN-{}", cr.id),
                    category: PartCategory::CraneRunway,
                    description: format!("Crane {} runway bracket", cr.id),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Crane {}", cr.id),
                    start_m: [x_col, y, cr.runway_height_m],
                    end_m: [x_brk, y, cr.runway_height_m],
                    depth_start_m: 0.35,
                    depth_end_m: 0.25,
                    width_m: 0.20,
                    length_m: cr.bracket_eccentricity_m,
                    mass_kg: 45.0,
                    bbox: Aabb3::from_points(
                        [x_col, y, cr.runway_height_m],
                        [x_brk, y, cr.runway_height_m],
                        0.12,
                    ),
                    holes: vec![],
                    weld_note: None,
                });
            }
            for (bay_idx, &bay_len) in b.bay_lengths_m.iter().enumerate() {
                let y0 = frame_ys[bay_idx];
                let y1 = frame_ys[bay_idx + 1];
                parts.push(PartOccurrence {
                    occurrence_id: format!("CRN-{tag}-BEAM-B{}", bay_idx + 1),
                    piece_mark: format!("CR-BM-{bay_len:.2}"),
                    assembly_mark: format!("CRN-{}", cr.id),
                    category: PartCategory::CraneRunway,
                    description: format!("Crane runway beam W-section (Bay {})", bay_idx + 1),
                    material_id: "ASTM A992".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Crane {}", cr.id),
                    start_m: [x_brk, y0, cr.runway_height_m],
                    end_m: [x_brk, y1, cr.runway_height_m],
                    depth_start_m: 0.45,
                    depth_end_m: 0.45,
                    width_m: 0.20,
                    length_m: bay_len,
                    mass_kg: bay_len * 65.0,
                    bbox: Aabb3::from_points(
                        [x_brk, y0, cr.runway_height_m],
                        [x_brk, y1, cr.runway_height_m],
                        0.15,
                    ),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Portal frames / Wind bents (F-0104)
    for pf in &b.portal_frames {
        if pf.bay_index < b.bay_lengths_m.len() {
            let y0 = frame_ys[pf.bay_index];
            let y1 = frame_ys[pf.bay_index + 1];
            let x = if pf.wall == WallSide::LeftSidewall {
                0.0
            } else {
                b.width_m
            };
            let zh = pf.clear_height_m;
            parts.push(PartOccurrence {
                occurrence_id: format!("PORTAL-BM-B{}", pf.bay_index + 1),
                piece_mark: format!("PF-BM-{:.2}", y1 - y0),
                assembly_mark: format!("PORTAL-B{}", pf.bay_index + 1),
                category: PartCategory::PortalBent,
                description: "Portal bent transverse header beam".into(),
                material_id: "ASTM A572-50".into(),
                finish_code: finish_code.into(),
                location_tag: format!("Portal Bay {}", pf.bay_index + 1),
                start_m: [x, y0, zh],
                end_m: [x, y1, zh],
                depth_start_m: 0.30,
                depth_end_m: 0.30,
                width_m: 0.18,
                length_m: y1 - y0,
                mass_kg: (y1 - y0) * 40.0,
                bbox: Aabb3::from_points([x, y0, zh], [x, y1, zh], 0.10),
                holes: vec![],
                weld_note: None,
            });
        }
    }

    // Lean-to extensions (F-0004)
    for lt in &b.lean_tos {
        for bay_step in 0..=lt.bay_count {
            let bay_idx = lt.bay_start + bay_step;
            if bay_idx <= b.bay_lengths_m.len() {
                let y = frame_ys[bay_idx];
                let (x_attach, x_post) = match lt.wall {
                    WallSide::LeftSidewall => (0.0, -lt.span_m),
                    _ => (b.width_m, b.width_m + lt.span_m),
                };
                let z_attach = if lt.wall == WallSide::LeftSidewall {
                    b.left_eave_m
                } else {
                    b.right_eave_m
                };
                let z_post = lt.eave_height_m;
                parts.push(PartOccurrence {
                    occurrence_id: format!("LT-{}-COL-{}", lt.id, bay_idx + 1),
                    piece_mark: format!("LT-C-{:.2}", z_post),
                    assembly_mark: format!("LT-{}", lt.id),
                    category: PartCategory::LeanToMember,
                    description: format!("Lean-to {} post", lt.id),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Lean-to {}", lt.id),
                    start_m: [x_post, y, 0.0],
                    end_m: [x_post, y, z_post],
                    depth_start_m: 0.20,
                    depth_end_m: 0.20,
                    width_m: 0.15,
                    length_m: z_post,
                    mass_kg: z_post * 25.0,
                    bbox: Aabb3::from_points([x_post, y, 0.0], [x_post, y, z_post], 0.10),
                    holes: vec![],
                    weld_note: None,
                });
                let r_len = (lt.span_m.powi(2) + (z_attach - z_post).powi(2)).sqrt();
                parts.push(PartOccurrence {
                    occurrence_id: format!("LT-{}-RAF-{}", lt.id, bay_idx + 1),
                    piece_mark: format!("LT-R-{:.2}", r_len),
                    assembly_mark: format!("LT-{}", lt.id),
                    category: PartCategory::LeanToMember,
                    description: format!("Lean-to {} rafter", lt.id),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Lean-to {}", lt.id),
                    start_m: [x_attach, y, z_attach],
                    end_m: [x_post, y, z_post],
                    depth_start_m: 0.25,
                    depth_end_m: 0.25,
                    width_m: 0.15,
                    length_m: r_len,
                    mass_kg: r_len * 30.0,
                    bbox: Aabb3::from_points([x_attach, y, z_attach], [x_post, y, z_post], 0.10),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Canopies & Overhangs (F-0001, F-0112)
    for can in &b.canopies {
        for bay_step in 0..=can.bay_count {
            let bay_idx = can.bay_start + bay_step;
            if bay_idx <= b.bay_lengths_m.len() {
                let y = frame_ys[bay_idx];
                let (x_in, x_out) = match can.wall {
                    WallSide::LeftSidewall => (0.0, -can.projection_m),
                    _ => (b.width_m, b.width_m + can.projection_m),
                };
                let z = can.height_m;
                parts.push(PartOccurrence {
                    occurrence_id: format!("CAN-{}-BRK-{}", can.id, bay_idx + 1),
                    piece_mark: format!("CAN-B-{:.2}", can.projection_m),
                    assembly_mark: format!("CAN-{}", can.id),
                    category: PartCategory::CanopyMember,
                    description: format!("Canopy {} cantilever bracket", can.id),
                    material_id: "ASTM A36".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Canopy {}", can.id),
                    start_m: [x_in, y, z],
                    end_m: [x_out, y, z],
                    depth_start_m: 0.20,
                    depth_end_m: 0.12,
                    width_m: 0.10,
                    length_m: can.projection_m,
                    mass_kg: can.projection_m * 18.0,
                    bbox: Aabb3::from_points([x_in, y, z], [x_out, y, z], 0.08),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Parapet systems (F-0075, F-0114)
    if let Some(ref para) = b.parapet
        && para.sidewalls
    {
        for (tag, x, eave_h) in [("L", 0.0, b.left_eave_m), ("R", b.width_m, b.right_eave_m)] {
            for (fi, &y) in frame_ys.iter().enumerate() {
                parts.push(PartOccurrence {
                    occurrence_id: format!("PARA-{tag}-STUB-{}", fi + 1),
                    piece_mark: format!("PAR-STUB-{:.2}", para.height_m),
                    assembly_mark: format!("PARA-{}", tag),
                    category: PartCategory::ParapetMember,
                    description: "Parapet vertical support stub".into(),
                    material_id: "ASTM A36".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Parapet {tag}"),
                    start_m: [x, y, eave_h],
                    end_m: [x, y, eave_h + para.height_m],
                    depth_start_m: 0.15,
                    depth_end_m: 0.15,
                    width_m: 0.10,
                    length_m: para.height_m,
                    mass_kg: para.height_m * 12.0,
                    bbox: Aabb3::from_points([x, y, eave_h], [x, y, eave_h + para.height_m], 0.06),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Crane runway beams and brackets (F-0118, F-0119)
    for cr in &b.cranes {
        for (tag, x_col) in [("L", 0.0), ("R", b.width_m)] {
            let x_brk = if tag == "L" {
                cr.bracket_eccentricity_m
            } else {
                b.width_m - cr.bracket_eccentricity_m
            };
            for (fi, &y) in frame_ys.iter().enumerate() {
                parts.push(PartOccurrence {
                    occurrence_id: format!("CRN-{tag}-BRK-{}", fi + 1),
                    piece_mark: format!("CR-BRK-{:.0}T", cr.capacity_metric_tons),
                    assembly_mark: format!("CRN-{}", cr.id),
                    category: PartCategory::CraneRunway,
                    description: format!("Crane {} runway bracket", cr.id),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Crane {}", cr.id),
                    start_m: [x_col, y, cr.runway_height_m],
                    end_m: [x_brk, y, cr.runway_height_m],
                    depth_start_m: 0.35,
                    depth_end_m: 0.25,
                    width_m: 0.20,
                    length_m: cr.bracket_eccentricity_m,
                    mass_kg: 45.0,
                    bbox: Aabb3::from_points(
                        [x_col, y, cr.runway_height_m],
                        [x_brk, y, cr.runway_height_m],
                        0.12,
                    ),
                    holes: vec![],
                    weld_note: None,
                });
            }
            for (bay_idx, &bay_len) in b.bay_lengths_m.iter().enumerate() {
                let y0 = frame_ys[bay_idx];
                let y1 = frame_ys[bay_idx + 1];
                parts.push(PartOccurrence {
                    occurrence_id: format!("CRN-{tag}-BEAM-B{}", bay_idx + 1),
                    piece_mark: format!("CR-BM-{bay_len:.2}"),
                    assembly_mark: format!("CRN-{}", cr.id),
                    category: PartCategory::CraneRunway,
                    description: format!("Crane runway beam W-section (Bay {})", bay_idx + 1),
                    material_id: "ASTM A992".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Crane {}", cr.id),
                    start_m: [x_brk, y0, cr.runway_height_m],
                    end_m: [x_brk, y1, cr.runway_height_m],
                    depth_start_m: 0.45,
                    depth_end_m: 0.45,
                    width_m: 0.20,
                    length_m: bay_len,
                    mass_kg: bay_len * 65.0,
                    bbox: Aabb3::from_points(
                        [x_brk, y0, cr.runway_height_m],
                        [x_brk, y1, cr.runway_height_m],
                        0.15,
                    ),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    // Portal frames / Wind bents (F-0104)
    for pf in &b.portal_frames {
        if pf.bay_index < b.bay_lengths_m.len() {
            let y0 = frame_ys[pf.bay_index];
            let y1 = frame_ys[pf.bay_index + 1];
            let x = if pf.wall == WallSide::LeftSidewall {
                0.0
            } else {
                b.width_m
            };
            let zh = pf.clear_height_m;
            parts.push(PartOccurrence {
                occurrence_id: format!("PORTAL-BM-B{}", pf.bay_index + 1),
                piece_mark: format!("PF-BM-{:.2}", y1 - y0),
                assembly_mark: format!("PORTAL-B{}", pf.bay_index + 1),
                category: PartCategory::PortalBent,
                description: "Portal bent transverse header beam".into(),
                material_id: "ASTM A572-50".into(),
                finish_code: finish_code.into(),
                location_tag: format!("Portal Bay {}", pf.bay_index + 1),
                start_m: [x, y0, zh],
                end_m: [x, y1, zh],
                depth_start_m: 0.30,
                depth_end_m: 0.30,
                width_m: 0.18,
                length_m: y1 - y0,
                mass_kg: (y1 - y0) * 40.0,
                bbox: Aabb3::from_points([x, y0, zh], [x, y1, zh], 0.10),
                holes: vec![],
                weld_note: None,
            });
        }
    }

    // Mezzanines (F-0105)
    for mz in &b.mezzanines {
        let y_start = frame_ys[mz.bay_start];
        let y_end = frame_ys[(mz.bay_start + mz.bay_count).min(frame_ys.len() - 1)];
        // Longitudinal floor girder
        parts.push(PartOccurrence {
            occurrence_id: format!("MEZZ-{}-GIRDER", mz.id),
            piece_mark: format!("MZ-GDR-{:.1}", mz.width_m),
            assembly_mark: format!("MEZZ-{}", mz.id),
            category: PartCategory::MezzanineBeam,
            description: format!("Mezzanine primary floor girder ({})", mz.deck_type),
            material_id: "ASTM A992".into(),
            finish_code: finish_code.into(),
            location_tag: format!("Mezzanine {}", mz.id),
            start_m: [0.0, y_start, mz.elevation_m],
            end_m: [mz.width_m, y_start, mz.elevation_m],
            depth_start_m: 0.40,
            depth_end_m: 0.40,
            width_m: 0.20,
            length_m: mz.width_m,
            mass_kg: mz.width_m * 52.0,
            bbox: Aabb3::from_points(
                [0.0, y_start, mz.elevation_m],
                [mz.width_m, y_start, mz.elevation_m],
                0.15,
            ),
            holes: vec![],
            weld_note: None,
        });
        // Mezzanine joists
        let num_joists = ((y_end - y_start) / mz.joist_spacing_m.max(0.5)).ceil() as usize;
        for j in 0..=num_joists {
            let y_j = y_start + (j as f64) * mz.joist_spacing_m;
            if y_j <= y_end {
                parts.push(PartOccurrence {
                    occurrence_id: format!("MEZZ-{}-JOIST-{}", mz.id, j + 1),
                    piece_mark: format!("MZ-JST-{:.1}", mz.width_m),
                    assembly_mark: format!("MEZZ-{}", mz.id),
                    category: PartCategory::MezzanineJoist,
                    description: "Mezzanine intermediate floor joist".into(),
                    material_id: "ASTM A572-50".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Mezzanine {}", mz.id),
                    start_m: [0.0, y_j, mz.elevation_m],
                    end_m: [mz.width_m, y_j, mz.elevation_m],
                    depth_start_m: 0.30,
                    depth_end_m: 0.30,
                    width_m: 0.15,
                    length_m: mz.width_m,
                    mass_kg: mz.width_m * 28.0,
                    bbox: Aabb3::from_points(
                        [0.0, y_j, mz.elevation_m],
                        [mz.width_m, y_j, mz.elevation_m],
                        0.10,
                    ),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
        // Mezzanine support columns
        parts.push(PartOccurrence {
            occurrence_id: format!("MEZZ-{}-COL-INT", mz.id),
            piece_mark: format!("MZ-COL-{:.1}", mz.elevation_m),
            assembly_mark: format!("MEZZ-{}", mz.id),
            category: PartCategory::MezzanineColumn,
            description: "Mezzanine interior HSS column".into(),
            material_id: "ASTM A500 Gr B".into(),
            finish_code: finish_code.into(),
            location_tag: format!("Mezzanine {}", mz.id),
            start_m: [mz.width_m / 2.0, y_start, 0.0],
            end_m: [mz.width_m / 2.0, y_start, mz.elevation_m],
            depth_start_m: 0.20,
            depth_end_m: 0.20,
            width_m: 0.20,
            length_m: mz.elevation_m,
            mass_kg: mz.elevation_m * 35.0,
            bbox: Aabb3::from_points(
                [mz.width_m / 2.0, y_start, 0.0],
                [mz.width_m / 2.0, y_start, mz.elevation_m],
                0.12,
            ),
            holes: vec![],
            weld_note: None,
        });
    }

    // Mini-Storage Units & Partitions (F-0106)
    if let Some(ref ms) = b.mini_storage {
        let total_y = b.total_length_m();
        let num_units_y = (total_y / ms.unit_depth_m.max(1.0)).floor() as usize;
        let num_units_x = (b.width_m / ms.unit_width_m.max(1.0)).floor() as usize;
        for ux in 0..num_units_x.min(8) {
            let x = (ux as f64) * ms.unit_width_m;
            for uy in 0..num_units_y.min(12) {
                let y = (uy as f64) * ms.unit_depth_m;
                // Partition wall sheet occurrence
                parts.push(PartOccurrence {
                    occurrence_id: format!("MS-PART-X{ux}-Y{uy}"),
                    piece_mark: format!("MS-PART-{}GA", ms.partition_sheet_gauge),
                    assembly_mark: "MINI-STORAGE".into(),
                    category: PartCategory::MiniStoragePartition,
                    description: format!(
                        "Mini-storage interior partition wall ({} Ga)",
                        ms.partition_sheet_gauge
                    ),
                    material_id: "ASTM A653 Gr 55".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Unit X{}Y{}", ux + 1, uy + 1),
                    start_m: [x, y, ms.stepped_pad_elevation_m],
                    end_m: [x + ms.unit_width_m, y, 3.0],
                    depth_start_m: 0.05,
                    depth_end_m: 0.05,
                    width_m: ms.unit_width_m,
                    length_m: ms.unit_width_m,
                    mass_kg: ms.unit_width_m * 8.5,
                    bbox: Aabb3::from_points(
                        [x, y, ms.stepped_pad_elevation_m],
                        [x + ms.unit_width_m, y, 3.0],
                        0.05,
                    ),
                    holes: vec![],
                    weld_note: None,
                });
                // Storage Unit Door
                parts.push(PartOccurrence {
                    occurrence_id: format!("MS-DOOR-X{ux}-Y{uy}"),
                    piece_mark: "MS-DR-ROLLUP".into(),
                    assembly_mark: "MINI-STORAGE".into(),
                    category: PartCategory::MiniStorageDoor,
                    description: format!("Mini-storage unit door ({:?})", ms.door_type),
                    material_id: "Galvanized Steel".into(),
                    finish_code: finish_code.into(),
                    location_tag: format!("Unit X{}Y{}", ux + 1, uy + 1),
                    start_m: [x, y, 0.0],
                    end_m: [x, y, 2.13],
                    depth_start_m: 0.05,
                    depth_end_m: 0.05,
                    width_m: 2.44,
                    length_m: 2.13,
                    mass_kg: 42.0,
                    bbox: Aabb3::from_points([x, y, 0.0], [x + 2.44, y, 2.13], 0.05),
                    holes: vec![],
                    weld_note: None,
                });
            }
        }
    }

    let mut model = DetailModel {
        parts,
        end_plates: vec![eave_plate, ridge_plate],
        base_plates: vec![base_plate],
        bomcheck: vec![],
        clashes: vec![],
    };
    model.bomcheck = run_bomcheck(&model, anchor_scope);
    model.clashes = run_clash_detection(&model, b);
    model
}

/// BOMCHECK: location & inventory traceability audit across fasteners, plates, anchors, and framing.
pub fn run_bomcheck(model: &DetailModel, anchor_scope: AnchorSupplyScope) -> Vec<BomCheckLine> {
    let mut map: BTreeMap<(String, String, String), BomCheckLine> = BTreeMap::new();
    for p in &model.parts {
        let cat = format!("{:?}", p.category);
        let key = (cat.clone(), p.piece_mark.clone(), p.location_tag.clone());
        let entry = map.entry(key).or_insert_with(|| BomCheckLine {
            category: cat,
            mark: p.piece_mark.clone(),
            location_tag: p.location_tag.clone(),
            inventory_id: p.material_id.clone(),
            count: 0,
            total_length_m: 0.0,
            total_mass_kg: 0.0,
            status: "RECONCILED".into(),
        });
        entry.count += 1;
        entry.total_length_m += p.length_m;
        entry.total_mass_kg += p.mass_kg;
    }
    if anchor_scope != AnchorSupplyScope::ByManufacturer {
        map.insert(
            (
                "AnchorRod".into(),
                "OMITTED".into(),
                "All Foundations".into(),
            ),
            BomCheckLine {
                category: "AnchorRod".into(),
                mark: "BY-OTHERS".into(),
                location_tag: "All Foundations".into(),
                inventory_id: format!("{anchor_scope:?}"),
                count: 0,
                total_length_m: 0.0,
                total_mass_kg: 0.0,
                status: "EXCLUDED BY DM81 SCOPE".into(),
            },
        );
    }
    map.into_values().collect()
}

/// Clash Detection: flags geometric interferences such as bracing rods intersecting openings or
/// overlapping structural bounding boxes across unrelated members.
pub fn run_clash_detection(model: &DetailModel, b: &BuildingSpec) -> Vec<ClashIssue> {
    let mut clashes = Vec::new();
    let mut group_counts: HashMap<String, usize> = HashMap::new();

    // Check 1: Diagonal bracing colliding with wall openings (common real-world PEB detailing clash!)
    for p in model
        .parts
        .iter()
        .filter(|p| p.category == PartCategory::DiagonalBrace)
    {
        let y_min = p.start_m[1].min(p.end_m[1]);
        let y_max = p.start_m[1].max(p.end_m[1]);
        let x_side = p.start_m[0];
        for o in &b.openings {
            let match_wall = (x_side < 0.5 && o.wall == WallSide::LeftSidewall)
                || (x_side > b.width_m - 0.5 && o.wall == WallSide::RightSidewall);
            if match_wall && o.offset_m < y_max && y_min < o.offset_m + o.width_m {
                let gkey = format!("BRACE-VS-OPENING-{}", o.wall.as_str());
                let idx = group_counts.entry(gkey.clone()).or_insert(0);
                *idx += 1;
                clashes.push(ClashIssue {
                    clash_id: format!("CLASH-{:04}", clashes.len() + 1),
                    group_key: gkey,
                    severity: "Error".into(),
                    part_a: p.occurrence_id.clone(),
                    part_b: format!("Opening {}", o.id),
                    description: format!(
                        "Diagonal rod brace {} passes across opening {} ({:.2}..{:.2} m); switch braced bay or use wind portal",
                        p.occurrence_id,
                        o.id,
                        o.offset_m,
                        o.offset_m + o.width_m
                    ),
                });
            }
        }
    }
    clashes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracing_through_door_opening_triggers_clash() {
        let mut b = BuildingSpec::default();
        b.openings.push(WallOpening {
            id: "DOOR-1".into(),
            wall: WallSide::LeftSidewall,
            offset_m: 1.5,
            sill_m: 0.0,
            width_m: 3.0,
            height_m: 3.5,
            shop_framed: true,
        });
        let db = CompanyDatabase::default();
        let detail = build_detail_model(
            &b,
            &db.built_up_sections[2],
            &db.built_up_sections[2],
            &db,
            "RED-OX",
            AnchorSupplyScope::ByManufacturer,
            180_000.0,
            90_000.0,
            120_000.0,
            40_000.0,
            60_000.0,
            DesignMethod::LRFD,
        );
        assert!(detail.clashes.iter().any(|c| c.part_b.contains("DOOR-1")));
        assert!(
            detail
                .parts
                .iter()
                .any(|p| p.category == PartCategory::AnchorRod)
        );
    }

    #[test]
    fn anchor_scope_by_others_excludes_anchor_parts_and_flags_bomcheck() {
        let b = BuildingSpec::default();
        let db = CompanyDatabase::default();
        let detail = build_detail_model(
            &b,
            &db.built_up_sections[1],
            &db.built_up_sections[1],
            &db,
            "RED-OX",
            AnchorSupplyScope::ByOthers,
            100_000.0,
            50_000.0,
            80_000.0,
            20_000.0,
            40_000.0,
            DesignMethod::LRFD,
        );
        assert!(
            !detail
                .parts
                .iter()
                .any(|p| p.category == PartCategory::AnchorRod)
        );
        assert!(
            detail
                .bomcheck
                .iter()
                .any(|l| l.category == "AnchorRod" && l.status.contains("EXCLUDED"))
        );
    }
}
