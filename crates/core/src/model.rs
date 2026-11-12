use crate::Result;
use serde::{Deserialize, Serialize};

/// All persisted fields carry their SI unit in their name. Inputs include their source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub customer: String,
    pub site: String,
    pub input_source: String,
    pub building: Building,
    pub section: Section,
    pub loads: Vec<LoadCase>,
    pub combinations: Vec<Combination>,
    pub rates: Rates,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Building {
    pub width_m: f64,
    pub length_m: f64,
    pub eave_m: f64,
    pub roof_rise_per_run: f64,
    pub bays: usize,
    pub fixed_bases: bool,
    pub purlin_spacing_m: f64,
    pub girt_spacing_m: f64,
    pub secondary_kg_per_m: f64,
    pub panel_kg_per_m2: f64,
    pub openings: Vec<Opening>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Opening {
    pub id: String,
    /// Left/right sidewall. Position measured along building length.
    pub wall: String,
    pub offset_m: f64,
    pub sill_m: f64,
    pub width_m: f64,
    pub height_m: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub id: String,
    pub source: String,
    pub depth_m: f64,
    pub flange_width_m: f64,
    pub web_m: f64,
    pub flange_m: f64,
    pub e_pa: f64,
    pub fy_pa: f64,
    pub density_kg_m3: f64,
}
impl Section {
    pub fn area_m2(&self) -> f64 {
        2.0 * self.flange_width_m * self.flange_m
            + (self.depth_m - 2.0 * self.flange_m) * self.web_m
    }
    pub fn inertia_m4(&self) -> f64 {
        (self.flange_width_m * self.depth_m.powi(3)
            - (self.flange_width_m - self.web_m) * (self.depth_m - 2.0 * self.flange_m).powi(3))
            / 12.0
    }
    pub fn modulus_m3(&self) -> f64 {
        self.inertia_m4() / (self.depth_m / 2.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadCase {
    pub id: String,
    pub source: String,
    /// Positive downward, referenced to horizontal roof projection. Include self weight explicitly.
    pub roof_down_kpa: f64,
    /// Positive global X, applied to the left wall only, uniform over height.
    pub wall_right_kpa: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Combination {
    pub id: String,
    pub factors: Vec<f64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rates {
    pub currency: String,
    pub source: String,
    pub steel_minor_per_kg: u64,
    pub panel_minor_per_m2: u64,
    pub fabrication_minor_per_kg: u64,
    pub finish_minor_per_kg: u64,
    pub markup_basis_points: u32,
}
impl Building {
    pub fn bay_m(&self) -> f64 {
        self.length_m / self.bays as f64
    }
    pub fn ridge_m(&self) -> f64 {
        self.eave_m + self.width_m / 2.0 * self.roof_rise_per_run
    }
    pub fn rafter_m(&self) -> f64 {
        (self.width_m / 2.0).hypot(self.width_m / 2.0 * self.roof_rise_per_run)
    }
    pub fn roof_area_m2(&self) -> f64 {
        2.0 * self.rafter_m() * self.length_m
    }
    pub fn wall_area_m2(&self) -> f64 {
        2.0 * (self.length_m + self.width_m) * self.eave_m
            + self.width_m * (self.ridge_m() - self.eave_m)
            - self
                .openings
                .iter()
                .map(|o| o.width_m * o.height_m)
                .sum::<f64>()
    }
    pub fn frame_points(&self) -> [[f64; 2]; 5] {
        [
            [0.0, 0.0],
            [0.0, self.eave_m],
            [self.width_m / 2.0, self.ridge_m()],
            [self.width_m, self.eave_m],
            [self.width_m, 0.0],
        ]
    }
}
impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: 1,
            name: "Demonstration warehouse".into(),
            customer: "Example customer".into(),
            site: "User-defined site".into(),
            input_source: "Illustrative inputs; replace with reviewed project data".into(),
            building: Building {
                width_m: 24.0,
                length_m: 36.0,
                eave_m: 6.0,
                roof_rise_per_run: 0.1,
                bays: 6,
                fixed_bases: true,
                purlin_spacing_m: 1.5,
                girt_spacing_m: 1.5,
                secondary_kg_per_m: 6.0,
                panel_kg_per_m2: 5.0,
                openings: vec![],
            },
            section: Section {
                id: "Welded I 600x220x8x12".into(),
                source: "Illustrative prismatic section; no mill certification".into(),
                depth_m: 0.6,
                flange_width_m: 0.22,
                web_m: 0.008,
                flange_m: 0.012,
                e_pa: 200e9,
                fy_pa: 250e6,
                density_kg_m3: 7850.0,
            },
            loads: vec![
                LoadCase {
                    id: "D".into(),
                    source: "Illustrative total dead load including self weight".into(),
                    roof_down_kpa: 0.4,
                    wall_right_kpa: 0.0,
                },
                LoadCase {
                    id: "Lr".into(),
                    source: "Illustrative roof live load".into(),
                    roof_down_kpa: 0.6,
                    wall_right_kpa: 0.0,
                },
                LoadCase {
                    id: "W".into(),
                    source: "Illustrative net wind pressures, not code-derived".into(),
                    roof_down_kpa: -0.7,
                    wall_right_kpa: 0.5,
                },
            ],
            combinations: vec![
                Combination {
                    id: "Gravity example".into(),
                    factors: vec![1.0, 1.0, 0.0],
                },
                Combination {
                    id: "Uplift example".into(),
                    factors: vec![1.0, 0.0, 1.0],
                },
            ],
            rates: Rates {
                currency: "USD".into(),
                source: "Illustrative rates in cents, not a supplier quotation".into(),
                steel_minor_per_kg: 150,
                panel_minor_per_m2: 1200,
                fabrication_minor_per_kg: 75,
                finish_minor_per_kg: 25,
                markup_basis_points: 1500,
            },
        }
    }
}
pub fn validate(p: &Project) -> Result<()> {
    let fail = |s: &str| Err(s.to_string());
    if p.schema_version != 1 {
        return fail("Unsupported project schema");
    }
    if p.name.trim().is_empty() || p.input_source.trim().is_empty() {
        return fail("Project name and input source are required");
    }
    let b = &p.building;
    let s = &p.section;
    for (name, x, min, max) in [
        ("width", b.width_m, 1.0, 200.0),
        ("length", b.length_m, 1.0, 1000.0),
        ("eave", b.eave_m, 1.0, 50.0),
        ("roof slope", b.roof_rise_per_run, 0.01, 1.0),
        ("purlin spacing", b.purlin_spacing_m, 0.1, 10.0),
        ("girt spacing", b.girt_spacing_m, 0.1, 10.0),
        ("secondary mass", b.secondary_kg_per_m, 0.1, 500.0),
        ("panel mass", b.panel_kg_per_m2, 0.1, 100.0),
        ("section depth", s.depth_m, 0.05, 3.0),
        ("flange width", s.flange_width_m, 0.01, 2.0),
        ("web", s.web_m, 0.001, 0.2),
        ("flange", s.flange_m, 0.001, 0.2),
        ("elastic modulus", s.e_pa, 1e9, 500e9),
        ("yield strength", s.fy_pa, 1e6, 2e9),
        ("density", s.density_kg_m3, 100.0, 30000.0),
    ] {
        if !x.is_finite() || x < min || x > max {
            return Err(format!("{name} must be finite and between {min} and {max}"));
        }
    }
    if b.bays == 0 || b.bays > 100 || b.bay_m() < 0.5 {
        return fail("Use 1–100 bays, each at least 0.5 m");
    }
    if 2.0 * s.flange_m >= s.depth_m
        || s.web_m >= s.flange_width_m
        || s.id.trim().is_empty()
        || s.source.trim().is_empty()
    {
        return fail("Invalid section geometry or missing section provenance");
    }
    if p.loads.is_empty()
        || p.loads.len() > 32
        || p.combinations.is_empty()
        || p.combinations.len() > 64
    {
        return fail("Require 1–32 load cases and 1–64 combinations");
    }
    let mut ids = std::collections::HashSet::new();
    for l in &p.loads {
        if l.id.trim().is_empty()
            || !ids.insert(&l.id)
            || l.source.trim().is_empty()
            || !l.roof_down_kpa.is_finite()
            || !l.wall_right_kpa.is_finite()
            || l.roof_down_kpa.abs() > 100.0
            || l.wall_right_kpa.abs() > 100.0
        {
            return fail(
                "Load cases require unique IDs, provenance and finite pressures within ±100 kPa",
            );
        }
    }
    ids.clear();
    for c in &p.combinations {
        if c.id.trim().is_empty()
            || !ids.insert(&c.id)
            || c.factors.len() != p.loads.len()
            || c.factors.iter().any(|f| !f.is_finite() || f.abs() > 10.0)
            || c.factors.iter().all(|f| *f == 0.0)
        {
            return fail("Invalid combination IDs or factors");
        }
    }
    ids.clear();
    for (i, o) in b.openings.iter().enumerate() {
        if o.id.trim().is_empty()
            || !ids.insert(&o.id)
            || !["left", "right"].contains(&o.wall.as_str())
            || [o.offset_m, o.sill_m, o.width_m, o.height_m]
                .iter()
                .any(|x| !x.is_finite())
            || o.offset_m < 0.0
            || o.sill_m < 0.0
            || o.width_m <= 0.0
            || o.height_m <= 0.0
            || o.offset_m + o.width_m > b.length_m
            || o.sill_m + o.height_m > b.eave_m
        {
            return fail("Opening must have a unique ID and fit within a left/right wall");
        }
        for other in &b.openings[..i] {
            if o.wall == other.wall
                && o.offset_m < other.offset_m + other.width_m
                && other.offset_m < o.offset_m + o.width_m
                && o.sill_m < other.sill_m + other.height_m
                && other.sill_m < o.sill_m + o.height_m
            {
                return fail("Wall openings overlap");
            }
        }
    }
    if p.rates.currency.len() != 3
        || !p.rates.currency.chars().all(|c| c.is_ascii_uppercase())
        || p.rates.source.trim().is_empty()
        || p.rates.markup_basis_points > 10000
        || [
            p.rates.steel_minor_per_kg,
            p.rates.panel_minor_per_m2,
            p.rates.fabrication_minor_per_kg,
            p.rates.finish_minor_per_kg,
        ]
        .iter()
        .any(|r| *r > 1_000_000_000)
    {
        return fail("Invalid currency, rate provenance, rate range or markup");
    }
    Ok(())
}
