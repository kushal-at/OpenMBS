//! Manufacturer-configured steel catalogs, fasteners, finishes, conditional rules,
//! and DBCHECK validation (independent clean-room Rust model).

use openmbs_units::UnitSystem;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialGrade {
    pub id: String,
    pub spec: String,
    pub e_pa: f64,
    pub fy_pa: f64,
    pub fu_pa: f64,
    pub density_kg_m3: f64,
}

impl Default for MaterialGrade {
    fn default() -> Self {
        Self {
            id: "ASTM A572-50".into(),
            spec: "ASTM A572 Grade 50 / Plate".into(),
            e_pa: 200e9,
            fy_pa: 344.738e6, // 50 ksi
            fu_pa: 448.159e6, // 65 ksi
            density_kg_m3: 7850.0,
        }
    }
}

/// Prismatic or linearly tapered doubly-symmetric welded I-section specification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuiltUpSectionSpec {
    pub id: String,
    pub source: String,
    pub depth_start_m: f64,
    pub depth_end_m: f64,
    pub flange_width_m: f64,
    pub flange_thickness_m: f64,
    pub web_thickness_m: f64,
    pub material_id: String,
}

impl BuiltUpSectionSpec {
    pub fn prismatic(
        id: impl Into<String>,
        source: impl Into<String>,
        depth_m: f64,
        flange_width_m: f64,
        web_thickness_m: f64,
        flange_thickness_m: f64,
        material_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            depth_start_m: depth_m,
            depth_end_m: depth_m,
            flange_width_m,
            flange_thickness_m,
            web_thickness_m,
            material_id: material_id.into(),
        }
    }

    pub fn area_at(&self, xi: f64) -> f64 {
        let d = self.depth_start_m * (1.0 - xi) + self.depth_end_m * xi;
        2.0 * self.flange_width_m * self.flange_thickness_m
            + (d - 2.0 * self.flange_thickness_m) * self.web_thickness_m
    }

    pub fn inertia_at(&self, xi: f64) -> f64 {
        let d = self.depth_start_m * (1.0 - xi) + self.depth_end_m * xi;
        let hw = d - 2.0 * self.flange_thickness_m;
        (self.flange_width_m * d.powi(3)
            - (self.flange_width_m - self.web_thickness_m) * hw.powi(3))
            / 12.0
    }

    pub fn mass_per_m(&self, density_kg_m3: f64) -> f64 {
        self.area_at(0.5) * density_kg_m3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColdFormedShape {
    ZSection,
    CSection,
    EaveStrut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColdFormedProfile {
    pub id: String,
    pub shape: ColdFormedShape,
    pub depth_m: f64,
    pub flange_width_m: f64,
    pub lip_m: f64,
    pub thickness_m: f64,
    pub fy_pa: f64,
    pub e_pa: f64,
    pub mass_kg_per_m: f64,
    pub ix_m4: f64,
    pub sx_m3: f64,
    pub ae_ratio: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoltSpec {
    pub id: String,
    pub grade: String,
    pub diameter_m: f64,
    /// Standard hole diameter in metres (per AISC J3.3).
    pub hole_diameter_m: f64,
    /// Nominal tensile strength Fnt (Pa).
    pub fnt_pa: f64,
    /// Nominal shear strength Fnv (Pa, threads excluded / N-condition).
    pub fnv_pa: f64,
    pub unit_mass_kg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorSupplyScope {
    /// MBS DM81 = Y: manufacturer supplies anchor bolts, nuts, washers.
    ByManufacturer,
    /// MBS DM81 = N: supplied by foundation contractor / others.
    ByOthers,
    /// Setting templates / nuts only supplied by manufacturer.
    TemplatesOnly,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnchorRodSpec {
    pub id: String,
    pub grade: String,
    pub diameter_m: f64,
    pub length_m: f64,
    pub embedment_m: f64,
    pub unit_mass_kg: f64,
    pub fnt_pa: f64,
    pub fnv_pa: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinishSpec {
    pub code: String,
    pub description: String,
    pub category: String,
    pub rgb: [u8; 3],
    pub finish_minor_per_kg: u64,
    pub panel_minor_per_m2_adder: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CostRule {
    pub id: String,
    pub priority: u32,
    pub match_finish_code: Option<String>,
    pub match_part_prefix: Option<String>,
    pub min_member_mass_kg_per_m: Option<f64>,
    pub steel_minor_per_kg: Option<u64>,
    pub fabrication_minor_per_kg: Option<u64>,
    pub finish_minor_per_kg: Option<u64>,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompanyDatabase {
    pub catalog_name: String,
    pub unit_system: UnitSystem,
    pub materials: Vec<MaterialGrade>,
    pub built_up_sections: Vec<BuiltUpSectionSpec>,
    pub cold_formed_profiles: Vec<ColdFormedProfile>,
    pub bolts: Vec<BoltSpec>,
    pub anchor_rods: Vec<AnchorRodSpec>,
    pub finishes: Vec<FinishSpec>,
    pub cost_rules: Vec<CostRule>,
    pub plate_thicknesses_m: Vec<f64>,
}

impl Default for CompanyDatabase {
    fn default() -> Self {
        Self {
            catalog_name: "OpenMBS Standard Manufacturer Catalog v1".into(),
            unit_system: UnitSystem::MetricSI,
            materials: vec![
                MaterialGrade::default(),
                MaterialGrade {
                    id: "ASTM A36".into(),
                    spec: "ASTM A36 Carbon Steel".into(),
                    e_pa: 200e9,
                    fy_pa: 248.211e6, // 36 ksi
                    fu_pa: 400.0e6,
                    density_kg_m3: 7850.0,
                },
            ],
            built_up_sections: vec![
                BuiltUpSectionSpec::prismatic(
                    "BU-400x180x6x10",
                    "Standard built-up plate I",
                    0.40,
                    0.18,
                    0.006,
                    0.010,
                    "ASTM A572-50",
                ),
                BuiltUpSectionSpec::prismatic(
                    "BU-600x220x8x12",
                    "Standard built-up plate I",
                    0.60,
                    0.22,
                    0.008,
                    0.012,
                    "ASTM A572-50",
                ),
                BuiltUpSectionSpec {
                    id: "TAP-400/850x220x8x14".into(),
                    source: "Tapered rigid-frame rafter/column family".into(),
                    depth_start_m: 0.40,
                    depth_end_m: 0.85,
                    flange_width_m: 0.22,
                    flange_thickness_m: 0.014,
                    web_thickness_m: 0.008,
                    material_id: "ASTM A572-50".into(),
                },
                BuiltUpSectionSpec {
                    id: "TAP-500/1050x250x8x16".into(),
                    source: "Heavy tapered rigid-frame family".into(),
                    depth_start_m: 0.50,
                    depth_end_m: 1.05,
                    flange_width_m: 0.25,
                    flange_thickness_m: 0.016,
                    web_thickness_m: 0.008,
                    material_id: "ASTM A572-50".into(),
                },
            ],
            cold_formed_profiles: vec![
                ColdFormedProfile {
                    id: "200Z18".into(),
                    shape: ColdFormedShape::ZSection,
                    depth_m: 0.2032,
                    flange_width_m: 0.0635,
                    lip_m: 0.022,
                    thickness_m: 0.0018,
                    fy_pa: 379.2e6,
                    e_pa: 203.4e9,
                    mass_kg_per_m: 5.20,
                    ix_m4: 4.18e-6,
                    sx_m3: 4.11e-5,
                    ae_ratio: 0.85,
                },
                ColdFormedProfile {
                    id: "200Z25".into(),
                    shape: ColdFormedShape::ZSection,
                    depth_m: 0.2032,
                    flange_width_m: 0.0635,
                    lip_m: 0.024,
                    thickness_m: 0.0025,
                    fy_pa: 379.2e6,
                    e_pa: 203.4e9,
                    mass_kg_per_m: 7.25,
                    ix_m4: 5.75e-6,
                    sx_m3: 5.66e-5,
                    ae_ratio: 0.92,
                },
                ColdFormedProfile {
                    id: "200C20".into(),
                    shape: ColdFormedShape::CSection,
                    depth_m: 0.2032,
                    flange_width_m: 0.0635,
                    lip_m: 0.020,
                    thickness_m: 0.0020,
                    fy_pa: 379.2e6,
                    e_pa: 203.4e9,
                    mass_kg_per_m: 5.75,
                    ix_m4: 4.55e-6,
                    sx_m3: 4.48e-5,
                    ae_ratio: 0.88,
                },
            ],
            bolts: vec![
                BoltSpec {
                    id: "M20-A325".into(),
                    grade: "ASTM F3125 Gr. A325".into(),
                    diameter_m: 0.020,
                    hole_diameter_m: 0.022,
                    fnt_pa: 620.5e6,
                    fnv_pa: 372.3e6,
                    unit_mass_kg: 0.28,
                },
                BoltSpec {
                    id: "M24-A325".into(),
                    grade: "ASTM F3125 Gr. A325".into(),
                    diameter_m: 0.024,
                    hole_diameter_m: 0.026,
                    fnt_pa: 620.5e6,
                    fnv_pa: 372.3e6,
                    unit_mass_kg: 0.46,
                },
            ],
            anchor_rods: vec![
                AnchorRodSpec {
                    id: "ANCH-M24-600".into(),
                    grade: "ASTM F1554 Gr. 36".into(),
                    diameter_m: 0.024,
                    length_m: 0.600,
                    embedment_m: 0.450,
                    unit_mass_kg: 2.35,
                    fnt_pa: 310.3e6,
                    fnv_pa: 186.2e6,
                },
                AnchorRodSpec {
                    id: "ANCH-M30-750".into(),
                    grade: "ASTM F1554 Gr. 55".into(),
                    diameter_m: 0.030,
                    length_m: 0.750,
                    embedment_m: 0.600,
                    unit_mass_kg: 4.45,
                    fnt_pa: 388.0e6,
                    fnv_pa: 233.0e6,
                },
            ],
            finishes: vec![
                FinishSpec {
                    code: "RED-OX".into(),
                    description: "Standard Red Oxide Shop Primer".into(),
                    category: "Primer".into(),
                    rgb: [152, 54, 46],
                    finish_minor_per_kg: 25,
                    panel_minor_per_m2_adder: 0,
                },
                FinishSpec {
                    code: "GRAY-PR".into(),
                    description: "Gray Alkyd Primer".into(),
                    category: "Primer".into(),
                    rgb: [130, 138, 146],
                    finish_minor_per_kg: 28,
                    panel_minor_per_m2_adder: 0,
                },
                FinishSpec {
                    code: "GALV-HD".into(),
                    description: "Hot-Dip Galvanized Framing".into(),
                    category: "Galvanized".into(),
                    rgb: [190, 198, 204],
                    finish_minor_per_kg: 85,
                    panel_minor_per_m2_adder: 0,
                },
            ],
            cost_rules: vec![CostRule {
                id: "RULE-GALV-FAB".into(),
                priority: 100,
                match_finish_code: Some("GALV-HD".into()),
                match_part_prefix: None,
                min_member_mass_kg_per_m: None,
                steel_minor_per_kg: None,
                fabrication_minor_per_kg: Some(95),
                finish_minor_per_kg: Some(85),
                description: "Hot-dip galvanized prep & surface finish override".into(),
            }],
            plate_thicknesses_m: vec![0.006, 0.008, 0.010, 0.012, 0.016, 0.020, 0.025, 0.032],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DbCheckIssue {
    pub rule_id: String,
    pub severity: DbSeverity,
    pub table: String,
    pub record_id: String,
    pub field: String,
    pub actual: String,
    pub expected: String,
}

/// DBCHECK equivalent: validates configured materials, sections, plates, bolts,
/// anchor inventories, finishes, and conditional rules against operational invariants.
pub fn dbcheck(
    db: &CompanyDatabase,
    expected_units: UnitSystem,
    anchor_scope: AnchorSupplyScope,
) -> Vec<DbCheckIssue> {
    let mut issues = Vec::new();

    if db.unit_system != expected_units {
        issues.push(DbCheckIssue {
            rule_id: "DB-001".into(),
            severity: DbSeverity::Warning,
            table: "CompanyDatabase".into(),
            record_id: db.catalog_name.clone(),
            field: "unit_system".into(),
            actual: db.unit_system.short_label().into(),
            expected: expected_units.short_label().into(),
        });
    }

    let mut mat_ids = HashSet::new();
    for m in &db.materials {
        if !mat_ids.insert(m.id.as_str()) {
            issues.push(DbCheckIssue {
                rule_id: "DB-002".into(),
                severity: DbSeverity::Error,
                table: "materials".into(),
                record_id: m.id.clone(),
                field: "id".into(),
                actual: "duplicate".into(),
                expected: "unique grade ID".into(),
            });
        }
        if !(100e9..=300e9).contains(&m.e_pa) || !(150e6..=800e6).contains(&m.fy_pa) {
            issues.push(DbCheckIssue {
                rule_id: "DB-003".into(),
                severity: DbSeverity::Error,
                table: "materials".into(),
                record_id: m.id.clone(),
                field: "e_pa/fy_pa".into(),
                actual: format!("E={:.1} GPa, Fy={:.1} MPa", m.e_pa / 1e9, m.fy_pa / 1e6),
                expected: "100-300 GPa, 150-800 MPa".into(),
            });
        }
    }

    let mut sec_ids = HashSet::new();
    for s in &db.built_up_sections {
        if !sec_ids.insert(s.id.as_str()) {
            issues.push(DbCheckIssue {
                rule_id: "DB-004".into(),
                severity: DbSeverity::Error,
                table: "built_up_sections".into(),
                record_id: s.id.clone(),
                field: "id".into(),
                actual: "duplicate".into(),
                expected: "unique section ID".into(),
            });
        }
        if !mat_ids.contains(s.material_id.as_str()) {
            issues.push(DbCheckIssue {
                rule_id: "DB-005".into(),
                severity: DbSeverity::Error,
                table: "built_up_sections".into(),
                record_id: s.id.clone(),
                field: "material_id".into(),
                actual: s.material_id.clone(),
                expected: "existing material grade".into(),
            });
        }
        if 2.0 * s.flange_thickness_m >= s.depth_start_m.min(s.depth_end_m)
            || s.web_thickness_m >= s.flange_width_m
        {
            issues.push(DbCheckIssue {
                rule_id: "DB-006".into(),
                severity: DbSeverity::Error,
                table: "built_up_sections".into(),
                record_id: s.id.clone(),
                field: "geometry".into(),
                actual: format!(
                    "d=[{:.3},{:.3}] tw={:.4} tf={:.4} bf={:.3}",
                    s.depth_start_m,
                    s.depth_end_m,
                    s.web_thickness_m,
                    s.flange_thickness_m,
                    s.flange_width_m
                ),
                expected: "2*tf < min(depth) and tw < bf".into(),
            });
        }
    }

    if db.bolts.is_empty() {
        issues.push(DbCheckIssue {
            rule_id: "DB-007".into(),
            severity: DbSeverity::Error,
            table: "bolts".into(),
            record_id: "-".into(),
            field: "bolts".into(),
            actual: "0 records".into(),
            expected: ">= 1 structural bolt record".into(),
        });
    } else {
        for b in &db.bolts {
            if b.hole_diameter_m <= b.diameter_m {
                issues.push(DbCheckIssue {
                    rule_id: "DB-008".into(),
                    severity: DbSeverity::Error,
                    table: "bolts".into(),
                    record_id: b.id.clone(),
                    field: "hole_diameter_m".into(),
                    actual: format!("{:.4}", b.hole_diameter_m),
                    expected: format!("> bolt diameter {:.4}", b.diameter_m),
                });
            }
        }
    }

    if anchor_scope == AnchorSupplyScope::ByManufacturer {
        if db.anchor_rods.is_empty() {
            issues.push(DbCheckIssue {
                rule_id: "DB-009".into(),
                severity: DbSeverity::Error,
                table: "anchor_rods".into(),
                record_id: "-".into(),
                field: "anchor_rods".into(),
                actual: "empty with DM81=ByManufacturer".into(),
                expected: "valid anchor record with positive length and weight".into(),
            });
        }
        for a in &db.anchor_rods {
            if a.length_m <= 0.0 || a.unit_mass_kg <= 0.0 || a.embedment_m >= a.length_m {
                issues.push(DbCheckIssue {
                    rule_id: "DB-010".into(),
                    severity: DbSeverity::Error,
                    table: "anchor_rods".into(),
                    record_id: a.id.clone(),
                    field: "length_m/unit_mass_kg".into(),
                    actual: format!(
                        "len={:.3} embed={:.3} mass={:.2}",
                        a.length_m, a.embedment_m, a.unit_mass_kg
                    ),
                    expected: "length > embedment > 0 and unit_mass > 0".into(),
                });
            }
        }
    }

    let finish_codes: HashSet<&str> = db.finishes.iter().map(|f| f.code.as_str()).collect();
    for rule in &db.cost_rules {
        if let Some(code) = &rule.match_finish_code
            && !finish_codes.contains(code.as_str())
        {
            issues.push(DbCheckIssue {
                rule_id: "DB-011".into(),
                severity: DbSeverity::Warning,
                table: "cost_rules".into(),
                record_id: rule.id.clone(),
                field: "match_finish_code".into(),
                actual: code.clone(),
                expected: "configured finish code".into(),
            });
        }
    }

    issues
}

/// Evaluates applicable DB_USER-style cost overrides in priority order and returns
/// resolved rates plus the winning rule ID for provenance.
pub fn resolve_cost_rates(
    db: &CompanyDatabase,
    finish_code: &str,
    part_prefix: &str,
    mass_kg_per_m: f64,
    base_steel_minor_per_kg: u64,
    base_fab_minor_per_kg: u64,
    base_finish_minor_per_kg: u64,
) -> (u64, u64, u64, String) {
    let mut steel = base_steel_minor_per_kg;
    let mut fab = base_fab_minor_per_kg;
    let mut finish = db
        .finishes
        .iter()
        .find(|f| f.code == finish_code)
        .map(|f| f.finish_minor_per_kg)
        .unwrap_or(base_finish_minor_per_kg);
    let mut trace = if db.finishes.iter().any(|f| f.code == finish_code) {
        format!("finish:{finish_code}")
    } else {
        "base-rates".to_string()
    };

    let mut rules = db.cost_rules.clone();
    rules.sort_by_key(|r| r.priority);
    for rule in rules {
        if let Some(f) = &rule.match_finish_code
            && f != finish_code
        {
            continue;
        }
        if let Some(p) = &rule.match_part_prefix
            && !part_prefix.starts_with(p)
        {
            continue;
        }
        if let Some(min_m) = rule.min_member_mass_kg_per_m
            && mass_kg_per_m < min_m
        {
            continue;
        }
        if let Some(v) = rule.steel_minor_per_kg {
            steel = v;
        }
        if let Some(v) = rule.fabrication_minor_per_kg {
            fab = v;
        }
        if let Some(v) = rule.finish_minor_per_kg {
            finish = v;
        }
        trace = format!("{trace} -> rule:{}", rule.id);
    }
    (steel, fab, finish, trace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_database_passes_dbcheck() {
        let db = CompanyDatabase::default();
        let issues = dbcheck(&db, UnitSystem::MetricSI, AnchorSupplyScope::ByManufacturer);
        assert!(issues.is_empty(), "Unexpected issues: {issues:?}");
    }

    #[test]
    fn missing_anchor_length_triggers_dbcheck_error() {
        let mut db = CompanyDatabase::default();
        db.anchor_rods[0].length_m = 0.0;
        let issues = dbcheck(&db, UnitSystem::MetricSI, AnchorSupplyScope::ByManufacturer);
        assert!(issues.iter().any(|i| i.rule_id == "DB-010"));
    }

    #[test]
    fn finish_rule_overrides_rates_with_trace() {
        let db = CompanyDatabase::default();
        let (steel, fab, fin, trace) = resolve_cost_rates(&db, "GALV-HD", "C1", 60.0, 150, 75, 25);
        assert_eq!(steel, 150);
        assert_eq!(fab, 95);
        assert_eq!(fin, 85);
        assert!(trace.contains("rule:RULE-GALV-FAB"));
    }
}
