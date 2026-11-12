use crate::{ENGINE, Result, model::*, solver::*};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberCheck {
    pub member: String,
    pub axial_n: f64,
    pub moment_nm: f64,
    pub elastic_stress_pa: f64,
    pub yield_ratio: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameRun {
    pub combination: String,
    pub tributary_m: f64,
    pub solution: Solution,
    pub checks: Vec<MemberCheck>,
    pub max_translation_m: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    pub occurrence: String,
    pub mark: String,
    pub description: String,
    pub length_m: f64,
    pub mass_kg: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BomLine {
    pub mark: String,
    pub description: String,
    pub count: usize,
    pub length_m: f64,
    pub mass_kg: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostLine {
    pub description: String,
    pub quantity: f64,
    pub unit: String,
    pub rate_minor: u64,
    pub total_minor: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub engine: String,
    pub revision: String,
    pub frames: Vec<FrameRun>,
    pub parts: Vec<Part>,
    pub bom: Vec<BomLine>,
    pub costs: Vec<CostLine>,
    pub subtotal_minor: u64,
    pub markup_minor: u64,
    pub total_minor: u64,
    pub roof_area_m2: f64,
    pub wall_area_m2: f64,
    pub blockers: Vec<String>,
}
pub fn fingerprint(p: &Project) -> Result<String> {
    validate(p)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(p).map_err(|e| e.to_string())?)
    ))
}
impl Run {
    pub fn is_current(&self, p: &Project) -> bool {
        self.engine == ENGINE && fingerprint(p).is_ok_and(|h| h == self.revision)
    }
    pub fn production_ready(&self) -> bool {
        self.blockers.is_empty()
    }
    pub fn steel_mass_kg(&self) -> f64 {
        self.parts.iter().map(|p| p.mass_kg).sum()
    }
    pub fn governing_ratio(&self) -> f64 {
        self.frames
            .iter()
            .flat_map(|f| f.checks.iter())
            .map(|c| c.yield_ratio)
            .fold(0.0, f64::max)
    }
}
pub fn analyze_frame(p: &Project, c: &Combination, tributary: f64) -> Result<FrameRun> {
    validate(p)?;
    if !tributary.is_finite()
        || tributary <= 0.0
        || tributary > p.building.length_m
        || c.factors.len() != p.loads.len()
        || c.factors.iter().any(|x| !x.is_finite() || x.abs() > 10.0)
    {
        return Err("Invalid frame tributary width or load combination".into());
    }
    let b = &p.building;
    let s = &p.section;
    let roof: f64 = p
        .loads
        .iter()
        .zip(&c.factors)
        .map(|(l, f)| l.roof_down_kpa * f * 1000.0 * tributary)
        .sum();
    let wall: f64 = p
        .loads
        .iter()
        .zip(&c.factors)
        .map(|(l, f)| l.wall_right_kpa * f * 1000.0 * tributary)
        .sum();
    let pts = b.frame_points();
    let mut nodes = vec![];
    let mut elements = vec![];
    // Eight elements per member capture distributed-load deflections at interior nodes.
    for member in 0..4 {
        for j in 0..8 {
            let t = j as f64 / 8.0;
            nodes.push(Node {
                x: pts[member][0] * (1.0 - t) + pts[member + 1][0] * t,
                y: pts[member][1] * (1.0 - t) + pts[member + 1][1] * t,
                restrained: if member == 0 && j == 0 {
                    [true, true, b.fixed_bases]
                } else {
                    [false; 3]
                },
            });
            let dx = pts[member + 1][0] - pts[member][0];
            let dy = pts[member + 1][1] - pts[member][1];
            elements.push(Element {
                a: member * 8 + j,
                b: member * 8 + j + 1,
                e: s.e_pa,
                area: s.area_m2(),
                inertia: s.inertia_m4(),
                qx: if member == 0 { wall } else { 0.0 },
                qy: if member == 1 || member == 2 {
                    -roof * dx.abs() / dx.hypot(dy)
                } else {
                    0.0
                },
            });
        }
    }
    nodes.push(Node {
        x: b.width_m,
        y: 0.0,
        restrained: [true, true, b.fixed_bases],
    });
    let solution = solve(&nodes, &elements, &vec![[0.0; 3]; nodes.len()])?;
    let mut checks = vec![];
    for member in 0..4 {
        let mut axial: f64 = 0.0;
        let mut moment: f64 = 0.0;
        for (i, e) in elements.iter().enumerate().skip(member * 8).take(8) {
            let q = solution.end_forces[i];
            let dx = nodes[e.b].x - nodes[e.a].x;
            let dy = nodes[e.b].y - nodes[e.a].y;
            let l = dx.hypot(dy);
            let qy = -dy / l * e.qx + dx / l * e.qy;
            axial = axial.max(q[0].abs()).max(q[3].abs());
            moment = moment.max(q[2].abs()).max(q[5].abs());
            if qy.abs() > 1e-12 {
                let x = -q[1] / qy;
                if x > 0.0 && x < l {
                    moment = moment.max((q[2] - q[1] * x - qy * x * x / 2.0).abs());
                }
            }
        }
        let stress = axial / s.area_m2() + moment / s.modulus_m3();
        checks.push(MemberCheck {
            member: ["Left column", "Left rafter", "Right rafter", "Right column"][member].into(),
            axial_n: axial,
            moment_nm: moment,
            elastic_stress_pa: stress,
            yield_ratio: stress / s.fy_pa,
        });
    }
    let max_translation_m = solution
        .displacement
        .iter()
        .map(|u| u[0].hypot(u[1]))
        .fold(0.0, f64::max);
    Ok(FrameRun {
        combination: c.id.clone(),
        tributary_m: tributary,
        solution,
        checks,
        max_translation_m,
    })
}
fn cost(description: &str, quantity: f64, unit: &str, rate: u64) -> Result<CostLine> {
    // Quantity is quantized to 0.001 unit, then integer arithmetic rounds half-up to minor currency.
    let milli = (quantity * 1000.0).round() as u128;
    let amount = (milli * rate as u128 + 500) / 1000;
    Ok(CostLine {
        description: description.into(),
        quantity: milli as f64 / 1000.0,
        unit: unit.into(),
        rate_minor: rate,
        total_minor: u64::try_from(amount).map_err(|_| "Cost overflow")?,
    })
}
pub fn process(p: &Project) -> Result<Run> {
    validate(p)?;
    let b = &p.building;
    let mut frames = vec![];
    for c in &p.combinations {
        frames.push(analyze_frame(p, c, b.bay_m() / 2.0)?);
        if b.bays > 1 {
            frames.push(analyze_frame(p, c, b.bay_m())?);
        }
    }
    let mut parts = vec![];
    for i in 0..=b.bays {
        for (member, length) in [
            ("C-L", b.eave_m),
            ("C-R", b.eave_m),
            ("R-L", b.rafter_m()),
            ("R-R", b.rafter_m()),
        ] {
            parts.push(Part {
                occurrence: format!("frame-{i}/{member}"),
                mark: if member.starts_with('C') { "C1" } else { "R1" }.into(),
                description: format!(
                    "{} / {}",
                    if member.starts_with('C') {
                        "Column"
                    } else {
                        "Rafter"
                    },
                    p.section.id
                ),
                length_m: length,
                mass_kg: length * p.section.area_m2() * p.section.density_kg_m3,
            });
        }
    }
    let roof_intervals = (b.rafter_m() / b.purlin_spacing_m).ceil() as usize;
    let wall_intervals = (b.eave_m / b.girt_spacing_m).ceil() as usize;
    // Eave/ridge lines on each roof plane, and wall lines above the base. Openings do not trim preliminary girts.
    for bay in 0..b.bays {
        for row in 0..2 * (roof_intervals + 1) {
            parts.push(Part {
                occurrence: format!("purlin-{bay}-{row}"),
                mark: "P1".into(),
                description: "Preliminary roof purlin".into(),
                length_m: b.bay_m(),
                mass_kg: b.bay_m() * b.secondary_kg_per_m,
            });
        }
    }
    for bay in 0..b.bays {
        for row in 0..2 * wall_intervals {
            parts.push(Part {
                occurrence: format!("girt-{bay}-{row}"),
                mark: "G1".into(),
                description: "Preliminary sidewall girt (untrimmed)".into(),
                length_m: b.bay_m(),
                mass_kg: b.bay_m() * b.secondary_kg_per_m,
            });
        }
    }
    for end in 0..2 {
        for row in 0..wall_intervals {
            parts.push(Part {
                occurrence: format!("end-girt-{end}-{row}"),
                mark: "G2".into(),
                description: "Preliminary endwall girt (support design excluded)".into(),
                length_m: b.width_m,
                mass_kg: b.width_m * b.secondary_kg_per_m,
            });
        }
    }
    let mut grouped = std::collections::BTreeMap::<String, BomLine>::new();
    for part in &parts {
        let line = grouped.entry(part.mark.clone()).or_insert(BomLine {
            mark: part.mark.clone(),
            description: part.description.clone(),
            count: 0,
            length_m: part.length_m,
            mass_kg: 0.0,
        });
        line.count += 1;
        line.mass_kg += part.mass_kg;
    }
    let mass = parts.iter().map(|p| p.mass_kg).sum::<f64>();
    let r = &p.rates;
    let costs = vec![
        cost(
            "Structural and secondary steel",
            mass,
            "kg",
            r.steel_minor_per_kg,
        )?,
        cost(
            "Fabrication allowance",
            mass,
            "kg",
            r.fabrication_minor_per_kg,
        )?,
        cost("Finish allowance", mass, "kg", r.finish_minor_per_kg)?,
        cost(
            "Net envelope panels",
            b.roof_area_m2() + b.wall_area_m2(),
            "m2",
            r.panel_minor_per_m2,
        )?,
    ];
    let (subtotal_minor, markup_minor, total_minor) = cost_totals(&costs, r.markup_basis_points)?;
    let mut blockers=vec!["Code design not implemented: elastic yield screening excludes buckling, interaction equations, bracing and second-order effects.".into(),"Loads and combinations are user supplied; no hazard lookup or jurisdiction compliance verification.".into(),"Connections, bolts, welds, base plates, foundations and fabrication details are not designed.".into(),"Secondary members and envelope quantities are preliminary; no stability, support, opening trim, laps, waste or fastener design.".into(),"Independent engineer and fabricator approval workflow is not implemented; production issuance is blocked.".into()];
    if frames
        .iter()
        .flat_map(|f| &f.checks)
        .any(|c| c.yield_ratio > 1.0)
    {
        blockers
            .push("At least one member exceeds the preliminary elastic yield threshold.".into());
    }
    if !b.openings.is_empty() {
        blockers.push("Openings affect net panel area only; load redistribution and opening framing are excluded.".into());
    }
    Ok(Run {
        engine: ENGINE.into(),
        revision: fingerprint(p)?,
        frames,
        parts,
        bom: grouped.into_values().collect(),
        costs,
        subtotal_minor,
        markup_minor,
        total_minor,
        roof_area_m2: b.roof_area_m2(),
        wall_area_m2: b.wall_area_m2(),
        blockers,
    })
}

fn cost_totals(lines: &[CostLine], markup_basis_points: u32) -> Result<(u64, u64, u64)> {
    let subtotal = lines.iter().try_fold(0u64, |sum, line| {
        sum.checked_add(line.total_minor)
            .ok_or("Cost subtotal overflow")
    })?;
    let markup = u64::try_from((subtotal as u128 * markup_basis_points as u128 + 5000) / 10000)
        .map_err(|_| "Cost markup overflow")?;
    let total = subtotal.checked_add(markup).ok_or("Cost total overflow")?;
    Ok((subtotal, markup, total))
}

/// Enumerates candidate sections by mass; feasibility is PRELIMINARY yield/displacement only.
pub fn select_section(
    p: &Project,
    catalog: &[Section],
    max_translation_m: f64,
) -> Result<Option<Section>> {
    validate(p)?;
    if !max_translation_m.is_finite() || max_translation_m <= 0.0 {
        return Err("Positive finite displacement limit required".into());
    }
    let mut candidates = catalog.to_vec();
    for s in &candidates {
        let mut candidate = p.clone();
        candidate.section = s.clone();
        validate(&candidate)?;
    }
    candidates.sort_by(|a, b| {
        (a.area_m2() * a.density_kg_m3).total_cmp(&(b.area_m2() * b.density_kg_m3))
    });
    for section in candidates {
        let mut candidate = p.clone();
        candidate.section = section.clone();
        let r = process(&candidate)?;
        if r.governing_ratio() <= 1.0
            && r.frames
                .iter()
                .all(|f| f.max_translation_m <= max_translation_m)
        {
            return Ok(Some(section));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monetary_overflow_is_an_error() {
        let line = CostLine {
            description: "Overflow fixture".into(),
            quantity: 1.0,
            unit: "kg".into(),
            rate_minor: u64::MAX,
            total_minor: u64::MAX,
        };
        assert!(cost_totals(std::slice::from_ref(&line), 1).is_err());
        assert!(cost_totals(&[line.clone(), line], 0).is_err());
    }
}
