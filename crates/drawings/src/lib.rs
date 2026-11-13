//! Pure-Rust drafting & BIM interchange:
//! - Erection drawings (Anchor Bolt Plan + Roof/Frame Arrangement) in SVG & DXF
//! - Shop fabrication assembly sheets (tapered elevation, hole & weld tables, BOM) in SVG & 1:1 DXF
//! - Valid ISO-10303-21 IFC4 STEP file export for 3D BIM viewers & clash inspection

use openmbs_detailing::{DetailModel, PartCategory, PartOccurrence};
use openmbs_geometry::BuildingSpec;
use openmbs_units::{Length, UnitSystem, format_ft_in};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawingControl {
    pub title_company: String,
    pub logo_name: String,
    pub suppress_welds: bool,
    pub unit_system: UnitSystem,
}

impl Default for DrawingControl {
    fn default() -> Self {
        Self {
            title_company: "OPEN MBS".into(),
            logo_name: "OPENMBS_STD".into(),
            suppress_welds: false,
            unit_system: UnitSystem::MetricSI,
        }
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn fmt_dim(m: f64, sys: UnitSystem) -> String {
    match sys {
        UnitSystem::MetricSI => format!("{m:.3} m"),
        UnitSystem::ImperialUS => format_ft_in(Length::from_m(m)),
    }
}

/// Sheet E1: Anchor Bolt Plan & Frame Reaction Summary (SVG).
#[allow(clippy::too_many_arguments)]
pub fn anchor_plan_svg(
    project_name: &str,
    revision: &str,
    b: &BuildingSpec,
    detail: &DetailModel,
    ctrl: &DrawingControl,
    max_comp_kn: f64,
    max_uplift_kn: f64,
    max_shear_kn: f64,
) -> String {
    let sys = ctrl.unit_system;
    let total_len = b.total_length_m();
    let ys = b.frame_y_positions();
    let scale = (960.0 / total_len).min(460.0 / b.width_m);
    let ox = 110.0;
    let oy = 120.0;

    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 840"><rect width="1200" height="840" fill="#f8fafc"/><rect x="20" y="20" width="1160" height="800" fill="none" stroke="#1e293b" stroke-width="2"/><g font-family="Consolas, monospace" fill="#0f172a">"##
    );
    let _ = writeln!(
        s,
        r##"<text x="40" y="52" font-size="22" font-weight="bold">{} — ANCHOR BOLT &amp; FOUNDATION REACTION PLAN (SHEET E1)</text><text x="40" y="76" font-size="12" fill="#475569">Project: {} | Rev: {} | Out-to-Out: {} x {} | PRELIMINARY - NOT FOR CONSTRUCTION</text>"##,
        xml(&ctrl.title_company),
        xml(project_name),
        &revision[..revision.len().min(12)],
        fmt_dim(b.width_m, sys),
        fmt_dim(total_len, sys),
    );

    // Building steel outline
    let w_px = total_len * scale;
    let h_px = b.width_m * scale;
    let _ = writeln!(
        s,
        r##"<rect x="{ox}" y="{oy}" width="{w_px}" height="{h_px}" fill="none" stroke="#0284c7" stroke-width="2" stroke-dasharray="6,4"/>"##
    );

    // Column grid lines (1..N) + base plate & anchor symbol
    for (idx, &y) in ys.iter().enumerate() {
        let gx = ox + y * scale;
        let _ = writeln!(
            s,
            r##"<line x1="{gx}" y1="{}" x2="{gx}" y2="{}" stroke="#94a3b8" stroke-width="1" stroke-dasharray="4,4"/><circle cx="{gx}" cy="{}" r="11" fill="white" stroke="#0f172a"/><text x="{gx}" y="{}" text-anchor="middle" font-size="11">{}</text>"##,
            oy - 15.0,
            oy + h_px + 15.0,
            oy - 32.0,
            oy - 28.0,
            idx + 1
        );
        // Base plates at A (bottom) and B (top)
        for gy in [oy, oy + h_px] {
            let _ = writeln!(
                s,
                r##"<rect x="{}" y="{}" width="10" height="10" fill="#0f172a"/><text x="{}" y="{}" font-size="9">BP1</text>"##,
                gx - 5.0,
                gy - 5.0,
                gx + 7.0,
                gy - 6.0
            );
        }
        if idx + 1 < ys.len() {
            let mid_x = gx + 0.5 * b.bay_lengths_m[idx] * scale;
            let _ = writeln!(
                s,
                r##"<text x="{mid_x}" y="{}" text-anchor="middle" font-size="10">{}</text>"##,
                oy + h_px + 28.0,
                fmt_dim(b.bay_lengths_m[idx], sys)
            );
        }
    }

    // Braced bay X markers on sidewalls
    for &bb in &b.braced_bays {
        if bb < b.bay_lengths_m.len() {
            let x0 = ox + ys[bb] * scale;
            let x1 = ox + ys[bb + 1] * scale;
            for gy in [oy, oy + h_px] {
                let _ = writeln!(
                    s,
                    r##"<line x1="{x0}" y1="{}" x2="{x1}" y2="{}" stroke="#dc2626" stroke-width="2"/><line x1="{x0}" y1="{}" x2="{x1}" y2="{}" stroke="#dc2626" stroke-width="2"/>"##,
                    gy - 8.0,
                    gy + 8.0,
                    gy + 8.0,
                    gy - 8.0
                );
            }
        }
    }

    // Anchor & Reaction summary table
    let bp = detail.base_plates.first();
    let ty = oy + h_px + 65.0;
    let _ = writeln!(
        s,
        r##"<rect x="40" y="{ty}" width="1120" height="120" fill="#f1f5f9" stroke="#cbd5e1"/><text x="55" y="{}" font-size="13" font-weight="bold">FOUNDATION &amp; ANCHOR INTERFACE SUMMARY</text>"##,
        ty + 22.0
    );
    if let Some(bp) = bp {
        let _ = writeln!(
            s,
            r##"<text x="55" y="{}" font-size="12">Base Plate: BP1 ({:.0} x {:.0} x {:.0} mm) | Anchors: {}x {} | Supply Scope (DM81): {:?}</text><text x="55" y="{}" font-size="12">Factored Envelope Reactions: Max Compression = {:.1} kN | Max Uplift = {:.1} kN | Max Shear = {:.1} kN</text><text x="55" y="{}" font-size="11" fill="#475569">Note: Anchor reactions govern embedment and concrete pier design by foundation engineer of record.</text>"##,
            ty + 48.0,
            bp.plate_width_m * 1000.0,
            bp.plate_length_m * 1000.0,
            bp.plate_thickness_m * 1000.0,
            bp.anchor_count,
            xml(&bp.anchor_id),
            bp.anchor_scope,
            ty + 72.0,
            max_comp_kn,
            max_uplift_kn,
            max_shear_kn,
            ty + 98.0
        );
    }
    s.push_str("</g></svg>");
    s
}

/// Sheet RF1: Shop Assembly Fabrication Drawing for Tapered/Prismatic Rafter & Material Bill (SVG).
pub fn shop_assembly_svg(
    project_name: &str,
    member_mark: &str,
    detail: &DetailModel,
    ctrl: &DrawingControl,
) -> String {
    let sys = ctrl.unit_system;
    let matching: Vec<&PartOccurrence> = detail
        .parts
        .iter()
        .filter(|p| p.piece_mark == member_mark)
        .collect();
    let sample = matching.first().copied();

    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 780"><rect width="1200" height="780" fill="#ffffff"/><rect x="20" y="20" width="1160" height="740" fill="none" stroke="#0f172a" stroke-width="2"/><g font-family="Consolas, monospace" fill="#0f172a">"##
    );
    let _ = writeln!(
        s,
        r##"<text x="40" y="52" font-size="22" font-weight="bold">{} — SHOP ASSEMBLY DRAWING: MARK {} (QTY {})</text><text x="40" y="74" font-size="12" fill="#475569">Project: {} | Logo: {} | PRELIMINARY FABRICATION REFERENCE ONLY</text>"##,
        xml(&ctrl.title_company),
        xml(member_mark),
        matching.len(),
        xml(project_name),
        xml(&ctrl.logo_name),
    );

    if let Some(p) = sample {
        let px0 = 90.0;
        let py_mid = 230.0;
        let draw_w = 1000.0;
        let scale_d = 170.0 / p.depth_start_m.max(p.depth_end_m).max(0.2);
        let h0 = p.depth_start_m * scale_d;
        let h1 = p.depth_end_m * scale_d;

        // Draw tapered web polygon
        let _ = writeln!(
            s,
            r##"<polygon points="{px0},{} {},{} {},{} {px0},{}" fill="#e2e8f0" stroke="#0f172a" stroke-width="2.5"/>"##,
            py_mid - h0 * 0.5,
            px0 + draw_w,
            py_mid - h1 * 0.5,
            px0 + draw_w,
            py_mid + h1 * 0.5,
            py_mid + h0 * 0.5,
        );
        // End plates at both ends
        let _ = writeln!(
            s,
            r##"<rect x="{}" y="{}" width="8" height="{}" fill="#334155"/><rect x="{}" y="{}" width="8" height="{}" fill="#334155"/>"##,
            px0 - 8.0,
            py_mid - h0 * 0.5 - 12.0,
            h0 + 24.0,
            px0 + draw_w,
            py_mid - h1 * 0.5 - 12.0,
            h1 + 24.0,
        );
        // Hole circles
        for h in &p.holes {
            let hx = px0 + (h.x_from_start_m / p.length_m.max(0.1)) * draw_w;
            let hy = py_mid;
            let _ = writeln!(
                s,
                r##"<circle cx="{hx}" cy="{hy}" r="4.5" fill="white" stroke="#dc2626" stroke-width="1.5"/>"##
            );
        }

        // Dimensions
        let _ = writeln!(
            s,
            r##"<text x="{}" y="{}" text-anchor="middle" font-size="13">Length = {} | Mass/ea = {:.1} kg | Total = {:.1} kg</text><text x="45" y="{}" font-size="11">d1 = {:.0} mm</text><text x="{}" y="{}" font-size="11">d2 = {:.0} mm</text>"##,
            px0 + 0.5 * draw_w,
            py_mid + h0.max(h1) * 0.5 + 45.0,
            fmt_dim(p.length_m, sys),
            p.mass_kg,
            p.mass_kg * matching.len() as f64,
            py_mid,
            p.depth_start_m * 1000.0,
            px0 + draw_w + 15.0,
            py_mid,
            p.depth_end_m * 1000.0,
        );

        // Weld callout (respecting suppress_welds toggle!)
        if !ctrl.suppress_welds
            && let Some(weld) = &p.weld_note
        {
            let _ = writeln!(
                s,
                r##"<text x="{px0}" y="120" font-size="12" fill="#0369a1">WELD SPEC: {}</text>"##,
                xml(weld)
            );
        } else if ctrl.suppress_welds {
            let _ = writeln!(
                s,
                r##"<text x="{px0}" y="120" font-size="12" fill="#b45309">WELD SYMBOLS SUPPRESSED BY DRAWING CONTROL</text>"##
            );
        }

        // Shop Material & Cut Bill Table
        let _ = writeln!(
            s,
            r##"<rect x="40" y="420" width="1120" height="290" fill="#f8fafc" stroke="#cbd5e1"/><text x="55" y="448" font-size="14" font-weight="bold">MATERIAL LIST &amp; PIECE TRACEABILITY</text><text x="55" y="476" font-size="12">Mark: {} | Description: {}</text><text x="55" y="500" font-size="12">Material: {} | Finish: {} | Qty in Project: {}</text><text x="55" y="524" font-size="12">Single Piece Mass: {:.2} kg | Group Total Mass: {:.2} kg</text>"##,
            xml(&p.piece_mark),
            xml(&p.description),
            xml(&p.material_id),
            xml(&p.finish_code),
            matching.len(),
            p.mass_kg,
            p.mass_kg * matching.len() as f64,
        );
        for (i, h) in p.holes.iter().take(6).enumerate() {
            let _ = writeln!(
                s,
                r##"<text x="55" y="{}" font-size="11">Hole {}: x={} from left end, y_off={:.0} mm, dia={:.1} mm ({})</text>"##,
                556 + i * 22,
                i + 1,
                fmt_dim(h.x_from_start_m, sys),
                h.y_offset_m * 1000.0,
                h.diameter_m * 1000.0,
                xml(&h.note),
            );
        }
    }
    s.push_str("</g></svg>");
    s
}

/// Sheet ER1: Roof & Wall Framing Erection Plan (SVG).
pub fn erection_plan_svg(
    project_name: &str,
    revision: &str,
    b: &BuildingSpec,
    detail: &DetailModel,
    ctrl: &DrawingControl,
) -> String {
    let sys = ctrl.unit_system;
    let total_len = b.total_length_m();
    let ys = b.frame_y_positions();
    let scale = (960.0 / total_len).min(460.0 / b.width_m);
    let ox = 110.0;
    let oy = 120.0;

    let mut s = String::with_capacity(8192);
    let _ = writeln!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 840"><rect width="1200" height="840" fill="#f8fafc"/><rect x="20" y="20" width="1160" height="800" fill="none" stroke="#1e293b" stroke-width="2"/><g font-family="Consolas, monospace" fill="#0f172a">"##
    );
    let _ = writeln!(
        s,
        r##"<text x="40" y="52" font-size="22" font-weight="bold">{} — ROOF &amp; WALL ERECTION FRAMING PLAN (SHEET ER1)</text><text x="40" y="76" font-size="12" fill="#475569">Project: {} | Rev: {} | Framing Out-to-Out: {} x {} | FIELD ERECTION DRAWING</text>"##,
        xml(&ctrl.title_company),
        xml(project_name),
        &revision[..revision.len().min(12)],
        fmt_dim(b.width_m, sys),
        fmt_dim(total_len, sys),
    );

    let w_px = total_len * scale;
    let h_px = b.width_m * scale;
    let _ = writeln!(
        s,
        r##"<rect x="{ox}" y="{oy}" width="{w_px}" height="{h_px}" fill="#f1f5f9" stroke="#0284c7" stroke-width="2"/>"##
    );

    // Frame grid lines
    for (idx, &y) in ys.iter().enumerate() {
        let gx = ox + y * scale;
        let _ = writeln!(
            s,
            r##"<line x1="{gx}" y1="{}" x2="{gx}" y2="{}" stroke="#334155" stroke-width="2"/><circle cx="{gx}" cy="{}" r="11" fill="white" stroke="#0f172a"/><text x="{gx}" y="{}" text-anchor="middle" font-size="11">{}</text><text x="{gx}" y="{}" font-size="9" fill="#0284c7">FL-{}</text>"##,
            oy - 10.0,
            oy + h_px + 10.0,
            oy - 30.0,
            oy - 26.0,
            idx + 1,
            oy + 14.0,
            idx + 1
        );
    }

    // Longitudinal purlin run lines
    let num_purlin_runs = (b.width_m / b.purlin_spacing_m.max(0.5)).floor() as usize;
    for p_idx in 1..num_purlin_runs {
        let py = oy + (p_idx as f64) * b.purlin_spacing_m * scale;
        if py < oy + h_px {
            let _ = writeln!(
                s,
                r##"<line x1="{ox}" y1="{py}" x2="{}" y2="{py}" stroke="#64748b" stroke-width="1" stroke-dasharray="8,3"/><text x="{}" y="{}" font-size="8" fill="#64748b">PURLIN-{}</text>"##,
                ox + w_px,
                ox + 6.0,
                py - 2.0,
                p_idx
            );
        }
    }

    // Diagonal X-Braces in Braced Bays
    for &bb in &b.braced_bays {
        if bb < b.bay_lengths_m.len() {
            let x0 = ox + ys[bb] * scale;
            let x1 = ox + ys[bb + 1] * scale;
            let _ = writeln!(
                s,
                r##"<line x1="{x0}" y1="{oy}" x2="{x1}" y2="{}" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="5,3"/><line x1="{x0}" y1="{}" x2="{x1}" y2="{oy}" stroke="#dc2626" stroke-width="1.5" stroke-dasharray="5,3"/><text x="{}" y="{}" text-anchor="middle" font-size="10" fill="#dc2626" font-weight="bold">ROOF X-BRACE (Bay {})</text>"##,
                oy + h_px,
                oy + h_px,
                0.5 * (x0 + x1),
                oy + 0.5 * h_px,
                bb + 1
            );
        }
    }

    // Piece mark callout table
    let purlins = detail
        .parts
        .iter()
        .filter(|p| p.category == PartCategory::Purlin)
        .count();
    let girts = detail
        .parts
        .iter()
        .filter(|p| {
            p.category == PartCategory::SidewallGirt || p.category == PartCategory::EndwallGirt
        })
        .count();
    let frames = ys.len();
    let _ = writeln!(
        s,
        r##"<rect x="40" y="{}" width="1120" height="150" fill="#ffffff" stroke="#cbd5e1"/><text x="55" y="{}" font-size="13" font-weight="bold">ERECTION COMPONENT SCHEDULE</text><text x="55" y="{}" font-size="11">Rigid Frames: {} lines (FL-1 to FL-{}) | Primary Splice Plates: DG-4 Moment End-Plates</text><text x="55" y="{}" font-size="11">Roof Secondary: {} Cold-Formed Z-Purlins (Continuous Lap = {:.2} m) | Flange Bracing Spacing = {:.2} m</text><text x="55" y="{}" font-size="11">Wall Secondary: {} Cold-Formed C-Girts | Braced Bays: {:?}</text><text x="55" y="{}" font-size="10" fill="#475569">NOTE: Verify all anchor bolt locations per Sheet E1 prior to frame erection. Torque A325 bolts to calibrated values.</text>"##,
        (oy + h_px + 50.0).min(650.0),
        (oy + h_px + 75.0).min(675.0),
        (oy + h_px + 98.0).min(698.0),
        frames,
        frames,
        (oy + h_px + 120.0).min(720.0),
        purlins,
        b.purlin_lap_m,
        b.flange_brace_spacing_m,
        (oy + h_px + 142.0).min(742.0),
        girts,
        b.braced_bays,
        (oy + h_px + 168.0).min(768.0),
    );

    s.push_str("</g></svg>");
    s
}

/// Sheet E1: Foundation Anchor Bolt & Base Plate Layout Plan (AutoCAD DXF).
#[allow(clippy::too_many_arguments)]
pub fn anchor_plan_dxf(
    project_name: &str,
    revision: &str,
    b: &BuildingSpec,
    detail: &DetailModel,
    ctrl: &DrawingControl,
    max_comp_kn: f64,
    max_uplift_kn: f64,
    max_shear_kn: f64,
) -> String {
    let mut d = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n".to_string();
    let l_mm = b.total_length_m() * 1000.0;
    let w_mm = b.width_m * 1000.0;
    let ys = b.frame_y_positions();

    let line = |d: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, layer: &str| {
        let _ = writeln!(
            d,
            "0\nLINE\n8\n{layer}\n10\n{x1:.3}\n20\n{y1:.3}\n30\n0\n11\n{x2:.3}\n21\n{y2:.3}\n31\n0"
        );
    };
    let circle = |d: &mut String, cx: f64, cy: f64, r: f64, layer: &str| {
        let _ = writeln!(
            d,
            "0\nCIRCLE\n8\n{layer}\n10\n{cx:.3}\n20\n{cy:.3}\n30\n0\n40\n{r:.3}"
        );
    };
    let text = |d: &mut String, x: f64, y: f64, h: f64, val: &str, layer: &str| {
        let _ = writeln!(
            d,
            "0\nTEXT\n8\n{layer}\n10\n{x:.3}\n20\n{y:.3}\n30\n0\n40\n{h:.3}\n1\n{val}"
        );
    };

    // Building Steel Perimeter
    line(&mut d, 0.0, 0.0, l_mm, 0.0, "PERIMETER");
    line(&mut d, l_mm, 0.0, l_mm, w_mm, "PERIMETER");
    line(&mut d, l_mm, w_mm, 0.0, w_mm, "PERIMETER");
    line(&mut d, 0.0, w_mm, 0.0, 0.0, "PERIMETER");

    // Grid lines FL-1 to FL-N
    for (idx, &y) in ys.iter().enumerate() {
        let gx = y * 1000.0;
        line(&mut d, gx, -600.0, gx, w_mm + 600.0, "GRID_LINES");
        circle(&mut d, gx, w_mm + 1400.0, 450.0, "GRID_BUBBLES");
        let label = format!("{}", idx + 1);
        text(&mut d, gx - 120.0, w_mm + 1280.0, 320.0, &label, "GRID_TEXT");
    }

    // Base plates & Anchors
    let bp_w = detail
        .base_plates
        .first()
        .map(|bp| bp.plate_width_m * 1000.0)
        .unwrap_or(400.0);
    let bp_l = detail
        .base_plates
        .first()
        .map(|bp| bp.plate_length_m * 1000.0)
        .unwrap_or(400.0);
    for &y in &ys {
        let gx = y * 1000.0;
        for &gy in &[0.0, w_mm] {
            line(
                &mut d,
                gx - bp_l * 0.5,
                gy - bp_w * 0.5,
                gx + bp_l * 0.5,
                gy - bp_w * 0.5,
                "BASEPLATES",
            );
            line(
                &mut d,
                gx + bp_l * 0.5,
                gy - bp_w * 0.5,
                gx + bp_l * 0.5,
                gy + bp_w * 0.5,
                "BASEPLATES",
            );
            line(
                &mut d,
                gx + bp_l * 0.5,
                gy + bp_w * 0.5,
                gx - bp_l * 0.5,
                gy + bp_w * 0.5,
                "BASEPLATES",
            );
            line(
                &mut d,
                gx - bp_l * 0.5,
                gy + bp_w * 0.5,
                gx - bp_l * 0.5,
                gy - bp_w * 0.5,
                "BASEPLATES",
            );

            // 4 Anchor bolts
            let off_x = bp_l * 0.3;
            let off_y = bp_w * 0.3;
            for sx in &[-1.0, 1.0] {
                for sy in &[-1.0, 1.0] {
                    circle(&mut d, gx + sx * off_x, gy + sy * off_y, 16.0, "ANCHOR_RODS");
                }
            }
            text(&mut d, gx + bp_l * 0.6, gy - 100.0, 180.0, "BP1", "TEXT");
        }
    }

    // Sidewall X-Bracing
    for &bb in &b.braced_bays {
        if bb < b.bay_lengths_m.len() {
            let x0 = ys[bb] * 1000.0;
            let x1 = ys[bb + 1] * 1000.0;
            for &gy in &[0.0, w_mm] {
                line(&mut d, x0, gy - 250.0, x1, gy + 250.0, "X_BRACING");
                line(&mut d, x0, gy + 250.0, x1, gy - 250.0, "X_BRACING");
            }
        }
    }

    // Title Block and Reaction Schedule
    text(
        &mut d,
        0.0,
        -1800.0,
        450.0,
        &format!("{} - SHEET E1: FOUNDATION ANCHOR BOLT PLAN", ctrl.title_company),
        "TITLE_BLOCK",
    );
    text(
        &mut d,
        0.0,
        -2500.0,
        280.0,
        &format!(
            "Project: {} | Rev: {} | Out-to-Out: {:.1}m x {:.1}m",
            project_name, revision, b.width_m, b.total_length_m()
        ),
        "TITLE_BLOCK",
    );
    text(
        &mut d,
        0.0,
        -3100.0,
        240.0,
        &format!(
            "Factored Envelope Reactions: Max Comp = {:.1} kN | Max Uplift = {:.1} kN | Max Shear = {:.1} kN",
            max_comp_kn, max_uplift_kn, max_shear_kn
        ),
        "SCHEDULE",
    );

    d.push_str("0\nENDSEC\n0\nEOF\n");
    d
}

/// Sheet ER1: Roof & Wall Erection Framing Plan (AutoCAD DXF).
pub fn erection_plan_dxf(
    project_name: &str,
    revision: &str,
    b: &BuildingSpec,
    detail: &DetailModel,
    ctrl: &DrawingControl,
) -> String {
    let mut d = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n".to_string();
    let l_mm = b.total_length_m() * 1000.0;
    let w_mm = b.width_m * 1000.0;
    let ys = b.frame_y_positions();

    let line = |d: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, layer: &str| {
        let _ = writeln!(
            d,
            "0\nLINE\n8\n{layer}\n10\n{x1:.3}\n20\n{y1:.3}\n30\n0\n11\n{x2:.3}\n21\n{y2:.3}\n31\n0"
        );
    };
    let circle = |d: &mut String, cx: f64, cy: f64, r: f64, layer: &str| {
        let _ = writeln!(
            d,
            "0\nCIRCLE\n8\n{layer}\n10\n{cx:.3}\n20\n{cy:.3}\n30\n0\n40\n{r:.3}"
        );
    };
    let text = |d: &mut String, x: f64, y: f64, h: f64, val: &str, layer: &str| {
        let _ = writeln!(
            d,
            "0\nTEXT\n8\n{layer}\n10\n{x:.3}\n20\n{y:.3}\n30\n0\n40\n{h:.3}\n1\n{val}"
        );
    };

    // Framing Perimeter
    line(&mut d, 0.0, 0.0, l_mm, 0.0, "PERIMETER");
    line(&mut d, l_mm, 0.0, l_mm, w_mm, "PERIMETER");
    line(&mut d, l_mm, w_mm, 0.0, w_mm, "PERIMETER");
    line(&mut d, 0.0, w_mm, 0.0, 0.0, "PERIMETER");

    // Frame lines FL-1 to FL-N
    for (idx, &y) in ys.iter().enumerate() {
        let gx = y * 1000.0;
        line(&mut d, gx, 0.0, gx, w_mm, "RIGID_FRAMES");
        circle(&mut d, gx, w_mm + 1400.0, 450.0, "GRID_BUBBLES");
        let label = format!("FL-{}", idx + 1);
        text(&mut d, gx - 200.0, w_mm + 1280.0, 300.0, &label, "GRID_TEXT");
    }

    // Purlin runs along building length
    let num_purlins = (b.width_m / b.purlin_spacing_m.max(0.5)).floor() as usize;
    for p_idx in 1..num_purlins {
        let py = (p_idx as f64) * b.purlin_spacing_m * 1000.0;
        if py < w_mm {
            line(&mut d, 0.0, py, l_mm, py, "PURLIN_RUNS");
            let p_lbl = format!("PURLIN-{}", p_idx);
            text(&mut d, 300.0, py + 50.0, 160.0, &p_lbl, "PURLIN_TEXT");
        }
    }

    // Roof X-Braces in Braced Bays
    for &bb in &b.braced_bays {
        if bb < b.bay_lengths_m.len() {
            let x0 = ys[bb] * 1000.0;
            let x1 = ys[bb + 1] * 1000.0;
            line(&mut d, x0, 0.0, x1, w_mm, "ROOF_X_BRACE");
            line(&mut d, x0, w_mm, x1, 0.0, "ROOF_X_BRACE");
            let bay_lbl = format!("ROOF X-BRACE (Bay {})", bb + 1);
            text(&mut d, 0.5 * (x0 + x1) - 600.0, 0.5 * w_mm, 250.0, &bay_lbl, "ROOF_X_BRACE");
        }
    }

    // Title Block and Schedule
    let purlins = detail
        .parts
        .iter()
        .filter(|p| p.category == PartCategory::Purlin)
        .count();
    let girts = detail
        .parts
        .iter()
        .filter(|p| {
            p.category == PartCategory::SidewallGirt || p.category == PartCategory::EndwallGirt
        })
        .count();
    text(
        &mut d,
        0.0,
        -1800.0,
        450.0,
        &format!("{} - SHEET ER1: ROOF & WALL ERECTION FRAMING PLAN", ctrl.title_company),
        "TITLE_BLOCK",
    );
    text(
        &mut d,
        0.0,
        -2500.0,
        280.0,
        &format!(
            "Project: {} | Rev: {} | Framing Lines: FL-1..FL-{} | Braced Bays: {:?}",
            project_name, revision, ys.len(), b.braced_bays
        ),
        "TITLE_BLOCK",
    );
    text(
        &mut d,
        0.0,
        -3100.0,
        240.0,
        &format!(
            "Secondary Framing: {} Z-Purlins (Continuous Lap = {:.2}m) | {} C-Girts",
            purlins, b.purlin_lap_m, girts
        ),
        "SCHEDULE",
    );

    d.push_str("0\nENDSEC\n0\nEOF\n");
    d
}

/// 1:1 Fabrication DXF for a single piece assembly (dimensionless/mm-exact profile for CNC/CAD).
pub fn piece_fabrication_dxf(part: &PartOccurrence) -> String {
    let mut d = "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n".to_string();
    let l_mm = part.length_m * 1000.0;
    let h0 = part.depth_start_m * 500.0;
    let h1 = part.depth_end_m * 500.0;
    let mut line = |x1: f64, y1: f64, x2: f64, y2: f64, layer: &str| {
        let _ = writeln!(
            d,
            "0\nLINE\n8\n{layer}\n10\n{x1:.3}\n20\n{y1:.3}\n30\n0\n11\n{x2:.3}\n21\n{y2:.3}\n31\n0"
        );
    };
    line(0.0, -h0, l_mm, -h1, "PROFILE");
    line(l_mm, -h1, l_mm, h1, "PROFILE");
    line(l_mm, h1, 0.0, h0, "PROFILE");
    line(0.0, h0, 0.0, -h0, "PROFILE");
    for h in &part.holes {
        let _ = writeln!(
            d,
            "0\nCIRCLE\n8\nHOLES\n10\n{:.3}\n20\n{:.3}\n30\n0\n40\n{:.3}",
            h.x_from_start_m * 1000.0,
            h.y_offset_m * 1000.0,
            h.diameter_m * 500.0
        );
    }
    let _ = writeln!(
        d,
        "0\nTEXT\n8\nMARK\n10\n10.0\n20\n0.0\n30\n0\n40\n25.0\n1\n{}\n0\nENDSEC\n0\nEOF",
        part.piece_mark
    );
    d
}

/// Pure-Rust ISO-10303-21 STEP Physical File Format (IFC4) exporter for 3D BIM & clash viewers.
pub fn export_ifc4(project_name: &str, detail: &DetailModel) -> String {
    let mut out = String::with_capacity(16384);
    let mut eid = 1usize;
    let mut next_id = || {
        let id = eid;
        eid += 1;
        id
    };

    let _ = writeln!(out, "ISO-10303-21;");
    let _ = writeln!(out, "HEADER;");
    let _ = writeln!(
        out,
        "FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');"
    );
    let _ = writeln!(
        out,
        "FILE_NAME('{}.ifc','2026-11-12T00:00:00',('OpenMBS'),('OpenMBS'),'OpenMBS IFC4 Writer','OpenMBS','');",
        project_name.replace('\'', "")
    );
    let _ = writeln!(out, "FILE_SCHEMA(('IFC4'));");
    let _ = writeln!(out, "ENDSEC;");
    let _ = writeln!(out, "DATA;");

    let owner = next_id();
    let app = next_id();
    let person = next_id();
    let org = next_id();
    let p_and_o = next_id();
    let _ = writeln!(out, "#{person}=IFCPERSON($,$,'OpenMBS',$,$,$,$,$);");
    let _ = writeln!(out, "#{org}=IFCORGANIZATION($,'OpenMBS',$,$,$);");
    let _ = writeln!(
        out,
        "#{p_and_o}=IFCPERSONANDORGANIZATION(#{person},#{org},$);"
    );
    let _ = writeln!(
        out,
        "#{app}=IFCAPPLICATION(#{org},'0.2','OpenMBS','OpenMBS');"
    );
    let _ = writeln!(
        out,
        "#{owner}=IFCOWNERHISTORY(#{p_and_o},#{app},$,.NOCHANGE.,$,$,$,0);"
    );

    let origin = next_id();
    let dir_z = next_id();
    let dir_x = next_id();
    let world_ax = next_id();
    let ctx = next_id();
    let u_len = next_id();
    let u_mass = next_id();
    let units = next_id();
    let _ = writeln!(out, "#{origin}=IFCCARTESIANPOINT((0.,0.,0.));");
    let _ = writeln!(out, "#{dir_z}=IFCDIRECTION((0.,0.,1.));");
    let _ = writeln!(out, "#{dir_x}=IFCDIRECTION((1.,0.,0.));");
    let _ = writeln!(
        out,
        "#{world_ax}=IFCAXIS2PLACEMENT3D(#{origin},#{dir_z},#{dir_x});"
    );
    let _ = writeln!(
        out,
        "#{ctx}=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{world_ax},$);"
    );
    let _ = writeln!(out, "#{u_len}=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);");
    let _ = writeln!(out, "#{u_mass}=IFCSIUNIT(*,.MASSUNIT.,.KILO.,.GRAM.);");
    let _ = writeln!(out, "#{units}=IFCUNITASSIGNMENT((#{u_len},#{u_mass}));");

    let proj = next_id();
    let site = next_id();
    let bldg = next_id();
    let storey = next_id();
    let site_pl = next_id();
    let bldg_pl = next_id();
    let storey_pl = next_id();
    let _ = writeln!(
        out,
        "#{proj}=IFCPROJECT('0OPENMBSPROJ00000001',#{owner},'{}',$,$,$,$,(#{ctx}),#{units});",
        project_name.replace('\'', "")
    );
    let _ = writeln!(out, "#{site_pl}=IFCLOCALPLACEMENT($,#{world_ax});");
    let _ = writeln!(
        out,
        "#{site}=IFCSITE('0OPENMBSSITE00000001',#{owner},'Site',$,$,#{site_pl},$,$,.ELEMENT.,$,$,$,$,$);"
    );
    let _ = writeln!(out, "#{bldg_pl}=IFCLOCALPLACEMENT(#{site_pl},#{world_ax});");
    let _ = writeln!(
        out,
        "#{bldg}=IFCBUILDING('0OPENMBSBLDG00000001',#{owner},'Building',$,$,#{bldg_pl},$,$,.ELEMENT.,$,$,$);"
    );
    let _ = writeln!(
        out,
        "#{storey_pl}=IFCLOCALPLACEMENT(#{bldg_pl},#{world_ax});"
    );
    let _ = writeln!(
        out,
        "#{storey}=IFCBUILDINGSTOREY('0OPENMBSSTOR00000001',#{owner},'Ground',$,$,#{storey_pl},$,$,.ELEMENT.,0.);"
    );

    let r_agg1 = next_id();
    let r_agg2 = next_id();
    let r_agg3 = next_id();
    let _ = writeln!(
        out,
        "#{r_agg1}=IFCRELAGGREGATES('0RELAGG0000000000001',#{owner},$,$,#{proj},(#{site}));"
    );
    let _ = writeln!(
        out,
        "#{r_agg2}=IFCRELAGGREGATES('0RELAGG0000000000002',#{owner},$,$,#{site},(#{bldg}));"
    );
    let _ = writeln!(
        out,
        "#{r_agg3}=IFCRELAGGREGATES('0RELAGG0000000000003',#{owner},$,$,#{bldg},(#{storey}));"
    );

    let mut contained = Vec::new();
    for (idx, part) in detail.parts.iter().enumerate() {
        // Export structural members and plates (skip thousands of tiny bolt solids to keep IFC concise)
        if matches!(part.category, PartCategory::StructuralBolt) {
            continue;
        }
        let dx = part.end_m[0] - part.start_m[0];
        let dy = part.end_m[1] - part.start_m[1];
        let dz = part.end_m[2] - part.start_m[2];
        let len = (dx * dx + dy * dy + dz * dz)
            .sqrt()
            .max(part.length_m)
            .max(0.01);
        let (uz_x, uz_y, uz_z) = if (dx * dx + dy * dy + dz * dz) > 1e-8 {
            let l = (dx * dx + dy * dy + dz * dz).sqrt();
            (dx / l, dy / l, dz / l)
        } else {
            (0.0, 0.0, 1.0)
        };
        let (ux_x, ux_y, ux_z) = if uz_z.abs() < 0.99 {
            // cross([0,0,1], u_z)
            let cx = -uz_y;
            let cy = uz_x;
            let cl = (cx * cx + cy * cy).sqrt().max(1e-9);
            (cx / cl, cy / cl, 0.0)
        } else {
            (1.0, 0.0, 0.0)
        };

        let p_id = next_id();
        let dz_id = next_id();
        let dx_id = next_id();
        let ax_id = next_id();
        let pl_id = next_id();
        let prof_id = next_id();
        let prof_ax = next_id();
        let prof_pt = next_id();
        let ext_dir = next_id();
        let solid_id = next_id();
        let rep_id = next_id();
        let prod_def = next_id();
        let elem_id = next_id();

        let _ = writeln!(
            out,
            "#{p_id}=IFCCARTESIANPOINT(({:.4},{:.4},{:.4}));",
            part.start_m[0], part.start_m[1], part.start_m[2]
        );
        let _ = writeln!(
            out,
            "#{dz_id}=IFCDIRECTION(({uz_x:.6},{uz_y:.6},{uz_z:.6}));"
        );
        let _ = writeln!(
            out,
            "#{dx_id}=IFCDIRECTION(({ux_x:.6},{ux_y:.6},{ux_z:.6}));"
        );
        let _ = writeln!(
            out,
            "#{ax_id}=IFCAXIS2PLACEMENT3D(#{p_id},#{dz_id},#{dx_id});"
        );
        let _ = writeln!(out, "#{pl_id}=IFCLOCALPLACEMENT(#{storey_pl},#{ax_id});");

        let _ = writeln!(out, "#{prof_pt}=IFCCARTESIANPOINT((0.,0.));");
        let _ = writeln!(out, "#{prof_ax}=IFCAXIS2PLACEMENT2D(#{prof_pt},$);");
        let _ = writeln!(
            out,
            "#{prof_id}=IFCRECTANGLEPROFILEDEF(.AREA.,$,#{prof_ax},{:.4},{:.4});",
            part.width_m.max(0.02),
            (0.5 * (part.depth_start_m + part.depth_end_m)).max(0.02)
        );
        let _ = writeln!(out, "#{ext_dir}=IFCDIRECTION((0.,0.,1.));");
        let _ = writeln!(
            out,
            "#{solid_id}=IFCEXTRUDEDAREASOLID(#{prof_id},#{world_ax},#{ext_dir},{len:.4});"
        );
        let _ = writeln!(
            out,
            "#{rep_id}=IFCSHAPEREPRESENTATION(#{ctx},'Body','SweptSolid',(#{solid_id}));"
        );
        let _ = writeln!(
            out,
            "#{prod_def}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep_id}));"
        );

        let ifc_type = match part.category {
            PartCategory::PrimaryColumn | PartCategory::EndwallColumn => "IFCCOLUMN",
            PartCategory::PrimaryRafter => "IFCBEAM",
            PartCategory::ConnectionPlate | PartCategory::Stiffener => "IFCPLATE",
            _ => "IFCMEMBER",
        };
        let guid = format!("0OMBS{:017}", idx + 1);
        let _ = writeln!(
            out,
            "#{elem_id}={ifc_type}('{guid}',#{owner},'{}','{}',$,#{pl_id},#{prod_def},'{}',$);",
            part.occurrence_id.replace('\'', ""),
            part.description.replace('\'', ""),
            part.piece_mark.replace('\'', "")
        );
        contained.push(format!("#{elem_id}"));
    }

    if !contained.is_empty() {
        let rel_id = next_id();
        let _ = writeln!(
            out,
            "#{rel_id}=IFCRELCONTAINEDINSPATIALSTRUCTURE('0RELCONT000000000001',#{owner},$,$,({}),#{storey});",
            contained.join(",")
        );
    }

    let _ = writeln!(out, "ENDSEC;");
    let _ = writeln!(out, "END-ISO-10303-21;");
    out
}

/// Groups BOM occurrences into shop sheet candidates (one per unique structural piece mark).
pub fn shop_sheet_marks(detail: &DetailModel) -> Vec<String> {
    let mut marks = BTreeMap::new();
    for p in &detail.parts {
        if matches!(
            p.category,
            PartCategory::PrimaryColumn | PartCategory::PrimaryRafter | PartCategory::EndwallColumn
        ) {
            marks.insert(p.piece_mark.clone(), ());
        }
    }
    marks.into_keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmbs_codes::DesignMethod;
    use openmbs_detailing::build_detail_model;
    use openmbs_inventory::{AnchorSupplyScope, CompanyDatabase};

    #[test]
    fn generates_valid_ifc4_and_shop_dxf_outputs() {
        let b = BuildingSpec::default();
        let db = CompanyDatabase::default();
        let detail = build_detail_model(
            &b,
            &db.built_up_sections[2],
            &db.built_up_sections[2],
            &db,
            "RED-OX",
            AnchorSupplyScope::ByManufacturer,
            150_000.0,
            80_000.0,
            100_000.0,
            30_000.0,
            50_000.0,
            DesignMethod::LRFD,
        );
        let ifc = export_ifc4("Demo", &detail);
        assert!(ifc.starts_with("ISO-10303-21;"));
        assert!(ifc.contains("IFCPROJECT"));
        assert!(ifc.contains("IFCBEAM"));
        assert!(ifc.contains("IFCCOLUMN"));
        assert!(ifc.contains("END-ISO-10303-21;"));

        let rafter = detail
            .parts
            .iter()
            .find(|p| p.category == PartCategory::PrimaryRafter)
            .unwrap();
        let dxf = piece_fabrication_dxf(rafter);
        assert!(dxf.contains("PROFILE"));
        assert!(dxf.contains("HOLES"));
        assert!(dxf.contains("EOF"));
    }
}
