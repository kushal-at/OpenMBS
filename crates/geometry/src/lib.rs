//! Parametric 3D metal building geometry: unequal bays, symmetric/asymmetric gable
//! frames, 4-wall openings with framing flags, purlin/girt lines, and 3D AABB primitives.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dist(self, other: Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2))
            .sqrt()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aabb3 {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb3 {
    pub fn from_points(a: [f64; 3], b: [f64; 3], pad: f64) -> Self {
        Self {
            min: [
                a[0].min(b[0]) - pad,
                a[1].min(b[1]) - pad,
                a[2].min(b[2]) - pad,
            ],
            max: [
                a[0].max(b[0]) + pad,
                a[1].max(b[1]) + pad,
                a[2].max(b[2]) + pad,
            ],
        }
    }

    pub fn intersects(&self, other: &Self, clearance: f64) -> bool {
        self.min[0] < other.max[0] + clearance
            && self.max[0] + clearance > other.min[0]
            && self.min[1] < other.max[1] + clearance
            && self.max[1] + clearance > other.min[1]
            && self.min[2] < other.max[2] + clearance
            && self.max[2] + clearance > other.min[2]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WallSide {
    LeftSidewall,
    RightSidewall,
    FrontEndwall,
    BackEndwall,
}

impl WallSide {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LeftSidewall => "Left Sidewall",
            Self::RightSidewall => "Right Sidewall",
            Self::FrontEndwall => "Front Endwall",
            Self::BackEndwall => "Back Endwall",
        }
    }
    pub fn is_sidewall(self) -> bool {
        matches!(self, Self::LeftSidewall | Self::RightSidewall)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallOpening {
    pub id: String,
    pub wall: WallSide,
    /// Horizontal offset from wall origin (Y=0 for sidewalls, X=0 for endwalls), m.
    pub offset_m: f64,
    pub sill_m: f64,
    pub width_m: f64,
    pub height_m: f64,
    /// Shop-framed openings include structural jamb/header members; field-located skip shop framing.
    pub shop_framed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LeanToSpec {
    pub id: String,
    pub wall: WallSide,
    pub span_m: f64,
    pub eave_height_m: f64,
    pub slope: f64,
    pub bay_start: usize,
    pub bay_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanopySpec {
    pub id: String,
    pub wall: WallSide,
    pub projection_m: f64,
    pub height_m: f64,
    pub bay_start: usize,
    pub bay_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ParapetSpec {
    pub height_m: f64,
    pub sidewalls: bool,
    pub endwalls: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CraneSpec {
    pub id: String,
    pub capacity_metric_tons: f64,
    pub runway_height_m: f64,
    pub bracket_eccentricity_m: f64,
    pub wheel_base_m: f64,
    pub max_wheel_load_kn: f64,
    pub lateral_thrust_pct: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PortalFrameSpec {
    pub bay_index: usize,
    pub wall: WallSide,
    pub clear_height_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MezzanineSpec {
    pub id: String,
    pub elevation_m: f64,
    pub bay_start: usize,
    pub bay_count: usize,
    pub width_m: f64,
    pub live_load_kpa: f64,
    pub beam_spacing_m: f64,
    pub joist_spacing_m: f64,
    pub deck_type: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageUnitType {
    StandardDriveUp,
    ClimateControlled,
    InteriorCorridor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageDoorType {
    RollUp8x7,
    RollUp9x7,
    RollUp10x10,
    SwingingWalk3x7,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiniStorageSpec {
    pub unit_width_m: f64,
    pub unit_depth_m: f64,
    pub corridor_width_m: f64,
    pub unit_type: StorageUnitType,
    pub door_type: StorageDoorType,
    pub partition_sheet_gauge: u32,
    pub climate_controlled: bool,
    pub stepped_pad_elevation_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndwallType {
    /// Post-and-beam endwall with corner/wind columns and lighter endwall rafters.
    PostAndBeam,
    /// Expandable rigid frame matching interior frame capacity.
    ExpandableRigidFrame,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildingSpec {
    pub width_m: f64,
    /// Explicit bay spacing along length Y (supports unequal bay spacings like MBS Bay Spacing UI-0004).
    pub bay_lengths_m: Vec<f64>,
    pub left_eave_m: f64,
    pub right_eave_m: f64,
    /// Horizontal X position of roof ridge from left wall X=0. Usually width_m / 2.0.
    pub ridge_x_m: f64,
    pub ridge_height_m: f64,
    pub fixed_bases: bool,
    pub endwall_type: EndwallType,
    pub endwall_post_bays: usize,
    pub purlin_spacing_m: f64,
    pub girt_spacing_m: f64,
    pub flange_brace_spacing_m: f64,
    /// Bay indices (0-based) containing roof and sidewall diagonal rod X-bracing.
    pub braced_bays: Vec<usize>,
    pub secondary_kg_per_m: f64,
    pub panel_kg_per_m2: f64,
    pub purlin_lap_m: f64,
    pub openings: Vec<WallOpening>,
    #[serde(default)]
    pub lean_tos: Vec<LeanToSpec>,
    #[serde(default)]
    pub canopies: Vec<CanopySpec>,
    #[serde(default)]
    pub parapet: Option<ParapetSpec>,
    #[serde(default)]
    pub cranes: Vec<CraneSpec>,
    #[serde(default)]
    pub portal_frames: Vec<PortalFrameSpec>,
    #[serde(default)]
    pub mezzanines: Vec<MezzanineSpec>,
    #[serde(default)]
    pub mini_storage: Option<MiniStorageSpec>,
}

impl Default for BuildingSpec {
    fn default() -> Self {
        Self {
            width_m: 24.0,
            bay_lengths_m: vec![6.0; 6],
            left_eave_m: 6.0,
            right_eave_m: 6.0,
            ridge_x_m: 12.0,
            ridge_height_m: 7.2, // 1:10 roof rise/run over 12 m half-width
            fixed_bases: true,
            endwall_type: EndwallType::PostAndBeam,
            endwall_post_bays: 3,
            purlin_spacing_m: 1.5,
            girt_spacing_m: 1.5,
            flange_brace_spacing_m: 3.0,
            braced_bays: vec![0, 5],
            secondary_kg_per_m: 6.0,
            panel_kg_per_m2: 5.0,
            purlin_lap_m: 0.30,
            openings: vec![],
            lean_tos: vec![],
            canopies: vec![],
            parapet: None,
            cranes: vec![],
            portal_frames: vec![],
            mezzanines: vec![],
            mini_storage: None,
        }
    }
}

impl BuildingSpec {
    pub fn total_length_m(&self) -> f64 {
        self.bay_lengths_m.iter().sum()
    }

    pub fn bays(&self) -> usize {
        self.bay_lengths_m.len()
    }

    pub fn frame_y_positions(&self) -> Vec<f64> {
        let mut ys = Vec::with_capacity(self.bay_lengths_m.len() + 1);
        let mut acc = 0.0;
        ys.push(acc);
        for &b in &self.bay_lengths_m {
            acc += b;
            ys.push(acc);
        }
        ys
    }

    pub fn tributary_width_m(&self, frame_idx: usize) -> f64 {
        let n = self.bay_lengths_m.len();
        let left = if frame_idx > 0 {
            0.5 * self.bay_lengths_m[frame_idx - 1]
        } else {
            0.0
        };
        let right = if frame_idx < n {
            0.5 * self.bay_lengths_m[frame_idx]
        } else {
            0.0
        };
        left + right
    }

    pub fn left_rafter_length_m(&self) -> f64 {
        self.ridge_x_m.hypot(self.ridge_height_m - self.left_eave_m)
    }

    pub fn right_rafter_length_m(&self) -> f64 {
        (self.width_m - self.ridge_x_m).hypot(self.ridge_height_m - self.right_eave_m)
    }

    pub fn left_slope(&self) -> f64 {
        (self.ridge_height_m - self.left_eave_m) / self.ridge_x_m
    }

    pub fn right_slope(&self) -> f64 {
        (self.ridge_height_m - self.right_eave_m) / (self.width_m - self.ridge_x_m)
    }

    /// Key transverse frame nodes in 2D [x, z]: [LeftBase, LeftEave, Ridge, RightEave, RightBase].
    pub fn frame_points_xz(&self) -> [[f64; 2]; 5] {
        [
            [0.0, 0.0],
            [0.0, self.left_eave_m],
            [self.ridge_x_m, self.ridge_height_m],
            [self.width_m, self.right_eave_m],
            [self.width_m, 0.0],
        ]
    }

    pub fn roof_height_at_x(&self, x: f64) -> f64 {
        if x <= self.ridge_x_m {
            self.left_eave_m + x * self.left_slope()
        } else {
            self.ridge_height_m - (x - self.ridge_x_m) * self.right_slope()
        }
    }

    pub fn gross_roof_area_m2(&self) -> f64 {
        (self.left_rafter_length_m() + self.right_rafter_length_m()) * self.total_length_m()
    }

    pub fn net_wall_area_m2(&self) -> f64 {
        let len = self.total_length_m();
        let sidewalls = (self.left_eave_m + self.right_eave_m) * len;
        let single_endwall = 0.5 * (self.left_eave_m + self.ridge_height_m) * self.ridge_x_m
            + 0.5 * (self.ridge_height_m + self.right_eave_m) * (self.width_m - self.ridge_x_m);
        let openings: f64 = self.openings.iter().map(|o| o.width_m * o.height_m).sum();
        (sidewalls + 2.0 * single_endwall - openings).max(0.0)
    }

    pub fn validate(&self) -> Result<()> {
        if !self.width_m.is_finite() || !(1.0..=200.0).contains(&self.width_m) {
            return Err("Building width must be finite and between 1 m and 200 m".into());
        }
        if self.bay_lengths_m.is_empty() || self.bay_lengths_m.len() > 100 {
            return Err("Building must have between 1 and 100 bays".into());
        }
        for (idx, &b) in self.bay_lengths_m.iter().enumerate() {
            if !b.is_finite() || !(0.5..=50.0).contains(&b) {
                return Err(format!(
                    "Bay {} length must be between 0.5 m and 50 m",
                    idx + 1
                ));
            }
        }
        if self.total_length_m() > 1000.0 {
            return Err("Total building length must not exceed 1000 m".into());
        }
        for (name, h) in [
            ("left eave", self.left_eave_m),
            ("right eave", self.right_eave_m),
            ("ridge height", self.ridge_height_m),
        ] {
            if !h.is_finite() || !(1.0..=60.0).contains(&h) {
                return Err(format!("{name} must be finite and between 1 m and 60 m"));
            }
        }
        if !self.ridge_x_m.is_finite()
            || self.ridge_x_m <= 0.1
            || self.ridge_x_m >= self.width_m - 0.1
        {
            return Err("Ridge X coordinate must lie strictly inside building width".into());
        }
        if self.ridge_height_m < self.left_eave_m.max(self.right_eave_m) {
            return Err("Ridge height must be >= both eave heights".into());
        }
        for (name, v, lo, hi) in [
            ("purlin spacing", self.purlin_spacing_m, 0.1, 10.0),
            ("girt spacing", self.girt_spacing_m, 0.1, 10.0),
            (
                "flange brace spacing",
                self.flange_brace_spacing_m,
                0.2,
                20.0,
            ),
            ("secondary kg/m", self.secondary_kg_per_m, 0.1, 500.0),
            ("panel kg/m2", self.panel_kg_per_m2, 0.1, 100.0),
            ("purlin lap", self.purlin_lap_m, 0.0, 3.0),
        ] {
            if !v.is_finite() || !(lo..=hi).contains(&v) {
                return Err(format!("{name} must be finite and between {lo} and {hi}"));
            }
        }
        if self.endwall_post_bays == 0 || self.endwall_post_bays > 40 {
            return Err("Endwall post bays must be between 1 and 40".into());
        }
        for &bb in &self.braced_bays {
            if bb >= self.bay_lengths_m.len() {
                return Err(format!("Braced bay index {} out of range", bb + 1));
            }
        }

        let mut ids = HashSet::new();
        let len = self.total_length_m();
        for (i, o) in self.openings.iter().enumerate() {
            if o.id.trim().is_empty() || !ids.insert(o.id.as_str()) {
                return Err("Every opening must have a unique non-empty ID".into());
            }
            if [o.offset_m, o.sill_m, o.width_m, o.height_m]
                .iter()
                .any(|v| !v.is_finite())
                || o.offset_m < 0.0
                || o.sill_m < 0.0
                || o.width_m <= 0.0
                || o.height_m <= 0.0
            {
                return Err(format!(
                    "Opening {} has invalid non-positive dimensions",
                    o.id
                ));
            }
            let max_span = if o.wall.is_sidewall() {
                len
            } else {
                self.width_m
            };
            if o.offset_m + o.width_m > max_span + 1e-9 {
                return Err(format!("Opening {} exceeds wall horizontal bounds", o.id));
            }
            let max_top = match o.wall {
                WallSide::LeftSidewall => self.left_eave_m,
                WallSide::RightSidewall => self.right_eave_m,
                WallSide::FrontEndwall | WallSide::BackEndwall => self
                    .roof_height_at_x(o.offset_m)
                    .min(self.roof_height_at_x(o.offset_m + o.width_m)),
            };
            if o.sill_m + o.height_m > max_top + 1e-9 {
                return Err(format!("Opening {} top exceeds wall/roof height", o.id));
            }
            for other in &self.openings[..i] {
                if o.wall == other.wall
                    && o.offset_m < other.offset_m + other.width_m
                    && other.offset_m < o.offset_m + o.width_m
                    && o.sill_m < other.sill_m + other.height_m
                    && other.sill_m < o.sill_m + o.height_m
                {
                    return Err(format!("Openings {} and {} overlap", o.id, other.id));
                }
            }
        }

        for lt in &self.lean_tos {
            if lt.span_m <= 0.0 || lt.eave_height_m <= 0.0 || lt.bay_count == 0 {
                return Err(format!("Lean-to {} has non-positive geometry", lt.id));
            }
            if lt.bay_start + lt.bay_count > self.bay_lengths_m.len() {
                return Err(format!("Lean-to {} extends beyond building bays", lt.id));
            }
        }
        for can in &self.canopies {
            if can.projection_m <= 0.0 || can.bay_count == 0 {
                return Err(format!("Canopy {} has non-positive geometry", can.id));
            }
            if can.bay_start + can.bay_count > self.bay_lengths_m.len() {
                return Err(format!("Canopy {} extends beyond building bays", can.id));
            }
        }
        for can in &self.canopies {
            if can.projection_m <= 0.0 || can.bay_count == 0 {
                return Err(format!("Canopy {} has non-positive geometry", can.id));
            }
            if can.bay_start + can.bay_count > self.bay_lengths_m.len() {
                return Err(format!("Canopy {} extends beyond building bays", can.id));
            }
        }
        if let Some(ref p) = self.parapet
            && !(0.1..=10.0).contains(&p.height_m)
        {
            return Err("Parapet height must be between 0.1 m and 10 m".into());
        }
        for cr in &self.cranes {
            if cr.capacity_metric_tons <= 0.0 || cr.runway_height_m <= 0.0 {
                return Err(format!(
                    "Crane {} has non-positive capacity or height",
                    cr.id
                ));
            }
            if cr.runway_height_m > self.left_eave_m.min(self.right_eave_m) {
                return Err(format!("Crane {} runway is higher than column eave", cr.id));
            }
        }
        for pf in &self.portal_frames {
            if pf.bay_index >= self.bay_lengths_m.len() {
                return Err(format!(
                    "Portal frame bay {} out of range",
                    pf.bay_index + 1
                ));
            }
        }
        for mz in &self.mezzanines {
            if mz.elevation_m <= 0.0 || mz.width_m <= 0.0 || mz.bay_count == 0 {
                return Err(format!("Mezzanine {} has non-positive geometry", mz.id));
            }
            if mz.bay_start + mz.bay_count > self.bay_lengths_m.len() {
                return Err(format!("Mezzanine {} extends beyond building bays", mz.id));
            }
            if mz.elevation_m >= self.left_eave_m.min(self.right_eave_m) {
                return Err(format!("Mezzanine {} elevation exceeds eave height", mz.id));
            }
        }
        if let Some(ref ms) = self.mini_storage
            && (ms.unit_width_m <= 0.0 || ms.unit_depth_m <= 0.0)
        {
            return Err("Mini-storage has non-positive unit geometry".into());
        }

        Ok(())
    }

    /// Preset 1: 40' x 60' Agricultural Barndominium / Shop with Lean-To Porch
    pub fn preset_ag_barndominium_40x60() -> Self {
        Self {
            width_m: 12.192,
            bay_lengths_m: vec![6.096, 6.096, 6.096],
            left_eave_m: 4.877,
            right_eave_m: 4.877,
            ridge_x_m: 6.096,
            ridge_height_m: 6.401,
            fixed_bases: false,
            endwall_type: EndwallType::PostAndBeam,
            endwall_post_bays: 2,
            purlin_spacing_m: 1.372,
            girt_spacing_m: 1.524,
            flange_brace_spacing_m: 3.048,
            braced_bays: vec![0, 2],
            secondary_kg_per_m: 5.5,
            panel_kg_per_m2: 4.8,
            purlin_lap_m: 0.30,
            openings: vec![
                WallOpening {
                    id: "OHD-1".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 6.5,
                    sill_m: 0.0,
                    width_m: 3.658,
                    height_m: 4.267,
                    shop_framed: true,
                },
                WallOpening {
                    id: "WD-1".into(),
                    wall: WallSide::LeftSidewall,
                    offset_m: 2.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
            ],
            lean_tos: vec![LeanToSpec {
                id: "LT-PORCH".into(),
                wall: WallSide::LeftSidewall,
                span_m: 3.658,
                eave_height_m: 3.5,
                slope: 0.15,
                bay_start: 0,
                bay_count: 3,
            }],
            canopies: vec![],
            parapet: None,
            cranes: vec![],
            portal_frames: vec![],
            mezzanines: vec![],
            mini_storage: None,
        }
    }

    /// Preset 2: 60' x 100' Commercial Warehouse with Loading Dock Canopy
    pub fn preset_commercial_warehouse_60x100() -> Self {
        Self {
            width_m: 18.288,
            bay_lengths_m: vec![7.620; 4],
            left_eave_m: 6.096,
            right_eave_m: 6.096,
            ridge_x_m: 9.144,
            ridge_height_m: 6.858,
            fixed_bases: true,
            endwall_type: EndwallType::PostAndBeam,
            endwall_post_bays: 3,
            purlin_spacing_m: 1.5,
            girt_spacing_m: 1.5,
            flange_brace_spacing_m: 3.0,
            braced_bays: vec![0, 3],
            secondary_kg_per_m: 6.2,
            panel_kg_per_m2: 5.2,
            purlin_lap_m: 0.30,
            openings: vec![
                WallOpening {
                    id: "DOCK-1".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 8.0,
                    sill_m: 0.0,
                    width_m: 3.048,
                    height_m: 3.658,
                    shop_framed: true,
                },
                WallOpening {
                    id: "DOCK-2".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 16.0,
                    sill_m: 0.0,
                    width_m: 3.048,
                    height_m: 3.658,
                    shop_framed: true,
                },
                WallOpening {
                    id: "EXIT-1".into(),
                    wall: WallSide::LeftSidewall,
                    offset_m: 2.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
                WallOpening {
                    id: "EXIT-2".into(),
                    wall: WallSide::LeftSidewall,
                    offset_m: 26.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
            ],
            lean_tos: vec![],
            canopies: vec![CanopySpec {
                id: "CAN-DOCK".into(),
                wall: WallSide::RightSidewall,
                projection_m: 1.524,
                height_m: 4.5,
                bay_start: 1,
                bay_count: 2,
            }],
            parapet: None,
            cranes: vec![],
            portal_frames: vec![],
            mezzanines: vec![],
            mini_storage: None,
        }
    }

    /// Preset 3: 100' x 200' Industrial Manufacturing Facility with 10-Ton Overhead Crane
    pub fn preset_industrial_crane_100x200() -> Self {
        Self {
            width_m: 30.480,
            bay_lengths_m: vec![7.620; 8],
            left_eave_m: 8.534,
            right_eave_m: 8.534,
            ridge_x_m: 15.240,
            ridge_height_m: 9.804,
            fixed_bases: true,
            endwall_type: EndwallType::ExpandableRigidFrame,
            endwall_post_bays: 4,
            purlin_spacing_m: 1.5,
            girt_spacing_m: 1.5,
            flange_brace_spacing_m: 3.0,
            braced_bays: vec![1, 6],
            secondary_kg_per_m: 7.5,
            panel_kg_per_m2: 5.8,
            purlin_lap_m: 0.35,
            openings: vec![
                WallOpening {
                    id: "EQ-1".into(),
                    wall: WallSide::FrontEndwall,
                    offset_m: 12.0,
                    sill_m: 0.0,
                    width_m: 5.486,
                    height_m: 5.486,
                    shop_framed: true,
                },
                WallOpening {
                    id: "WD-IND1".into(),
                    wall: WallSide::LeftSidewall,
                    offset_m: 2.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
                WallOpening {
                    id: "WD-IND2".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 2.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
            ],
            lean_tos: vec![],
            canopies: vec![],
            parapet: None,
            cranes: vec![CraneSpec {
                id: "CRANE-10T".into(),
                capacity_metric_tons: 10.0,
                runway_height_m: 6.5,
                bracket_eccentricity_m: 0.40,
                wheel_base_m: 3.5,
                max_wheel_load_kn: 85.0,
                lateral_thrust_pct: 10.0,
            }],
            portal_frames: vec![PortalFrameSpec {
                bay_index: 3,
                wall: WallSide::LeftSidewall,
                clear_height_m: 5.5,
            }],
            mezzanines: vec![],
            mini_storage: None,
        }
    }

    /// Preset 4: 120' x 300' Distribution Logistics Center with Mezzanine & Perimeter Parapet
    pub fn preset_distribution_center_120x300() -> Self {
        Self {
            width_m: 36.576,
            bay_lengths_m: vec![9.144; 10],
            left_eave_m: 9.754,
            right_eave_m: 9.754,
            ridge_x_m: 18.288,
            ridge_height_m: 10.516,
            fixed_bases: true,
            endwall_type: EndwallType::ExpandableRigidFrame,
            endwall_post_bays: 5,
            purlin_spacing_m: 1.5,
            girt_spacing_m: 1.5,
            flange_brace_spacing_m: 3.0,
            braced_bays: vec![1, 4, 8],
            secondary_kg_per_m: 8.2,
            panel_kg_per_m2: 6.0,
            purlin_lap_m: 0.40,
            openings: vec![
                WallOpening {
                    id: "DOCK-01".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 20.0,
                    sill_m: 0.0,
                    width_m: 2.743,
                    height_m: 3.048,
                    shop_framed: true,
                },
                WallOpening {
                    id: "DOCK-02".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 25.0,
                    sill_m: 0.0,
                    width_m: 2.743,
                    height_m: 3.048,
                    shop_framed: true,
                },
                WallOpening {
                    id: "DOCK-03".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 30.0,
                    sill_m: 0.0,
                    width_m: 2.743,
                    height_m: 3.048,
                    shop_framed: true,
                },
                WallOpening {
                    id: "DOCK-04".into(),
                    wall: WallSide::RightSidewall,
                    offset_m: 35.0,
                    sill_m: 0.0,
                    width_m: 2.743,
                    height_m: 3.048,
                    shop_framed: true,
                },
                WallOpening {
                    id: "EGRESS-1".into(),
                    wall: WallSide::LeftSidewall,
                    offset_m: 5.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
                WallOpening {
                    id: "EGRESS-2".into(),
                    wall: WallSide::LeftSidewall,
                    offset_m: 85.0,
                    sill_m: 0.0,
                    width_m: 0.914,
                    height_m: 2.134,
                    shop_framed: true,
                },
            ],
            lean_tos: vec![],
            canopies: vec![],
            parapet: Some(ParapetSpec {
                height_m: 0.914,
                sidewalls: true,
                endwalls: true,
            }),
            cranes: vec![],
            portal_frames: vec![],
            mezzanines: vec![MezzanineSpec {
                id: "MEZZ-OFFICE".into(),
                elevation_m: 4.267,
                bay_start: 0,
                bay_count: 2,
                width_m: 12.192,
                live_load_kpa: 2.5,
                beam_spacing_m: 6.0,
                joist_spacing_m: 1.5,
                deck_type: "Composite Concrete on B-Deck".into(),
            }],
            mini_storage: None,
        }
    }

    /// Preset 5: 30' x 90' Multi-Unit Self-Storage Building (MBS Mini-Storage)
    pub fn preset_mini_storage_30x90() -> Self {
        let mut openings = Vec::new();
        for i in 0..9 {
            openings.push(WallOpening {
                id: format!("UNIT-{:02}", i + 1),
                wall: WallSide::LeftSidewall,
                offset_m: 0.30 + (i as f64) * 3.048,
                sill_m: 0.0,
                width_m: 2.438,
                height_m: 2.134,
                shop_framed: true,
            });
        }
        Self {
            width_m: 9.144,
            bay_lengths_m: vec![3.048; 9],
            left_eave_m: 3.048,
            right_eave_m: 3.048,
            ridge_x_m: 4.572,
            ridge_height_m: 3.2385,
            fixed_bases: false,
            endwall_type: EndwallType::PostAndBeam,
            endwall_post_bays: 2,
            purlin_spacing_m: 1.2,
            girt_spacing_m: 1.5,
            flange_brace_spacing_m: 3.0,
            braced_bays: vec![0, 4, 8],
            secondary_kg_per_m: 4.5,
            panel_kg_per_m2: 4.5,
            purlin_lap_m: 0.25,
            openings,
            lean_tos: vec![],
            canopies: vec![],
            parapet: None,
            cranes: vec![],
            portal_frames: vec![],
            mezzanines: vec![],
            mini_storage: Some(MiniStorageSpec {
                unit_width_m: 3.048,
                unit_depth_m: 3.048,
                corridor_width_m: 1.5,
                unit_type: StorageUnitType::StandardDriveUp,
                door_type: StorageDoorType::RollUp8x7,
                partition_sheet_gauge: 26,
                climate_controlled: false,
                stepped_pad_elevation_m: 0.0,
            }),
        }
    }

    /// List of all standard industrial building presets with metadata.
    pub fn presets() -> Vec<(&'static str, &'static str, &'static str, Self)> {
        vec![
            (
                "ag_barndominium",
                "Ag Barndominium / Shop (40' x 60')",
                "Residential/Ag workshop with 12' lean-to porch and 12'x14' roll-up door",
                Self::preset_ag_barndominium_40x60(),
            ),
            (
                "commercial_warehouse",
                "Commercial Warehouse (60' x 100')",
                "Retail/distribution center with dual loading docks and 5' weather canopy",
                Self::preset_commercial_warehouse_60x100(),
            ),
            (
                "industrial_crane",
                "Industrial Manufacturing (100' x 200')",
                "Heavy manufacturing facility with 10-Ton bridge crane, portal frame, and oversized equipment door",
                Self::preset_industrial_crane_100x200(),
            ),
            (
                "distribution_center",
                "Logistics Mega Center (120' x 300')",
                "High-throughput logistics facility with office mezzanine, perimeter parapet, and 4 dock bays",
                Self::preset_distribution_center_120x300(),
            ),
            (
                "mini_storage",
                "MBS Multi-Unit Self-Storage (30' x 90')",
                "Drive-up self-storage complex with 9 roll-up doors and interior steel partitions",
                Self::preset_mini_storage_30x90(),
            ),
        ]
    }
}

/// Computes trimmed horizontal segments `[start, end]` along a wall after subtracting openings
/// intersecting the girt elevation `z_m`.
pub fn trimmed_girt_segments(
    wall_span_m: f64,
    z_m: f64,
    openings: &[&WallOpening],
) -> Vec<(f64, f64)> {
    let mut blocked: Vec<(f64, f64)> = openings
        .iter()
        .filter(|o| z_m > o.sill_m + 1e-6 && z_m < o.sill_m + o.height_m - 1e-6)
        .map(|o| (o.offset_m, o.offset_m + o.width_m))
        .collect();
    blocked.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut segs = Vec::new();
    let mut cursor = 0.0;
    for (s, e) in blocked {
        if s > cursor + 1e-4 {
            segs.push((cursor, s));
        }
        cursor = cursor.max(e);
    }
    if wall_span_m > cursor + 1e-4 {
        segs.push((cursor, wall_span_m));
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal_bays_compute_exact_tributaries() {
        let b = BuildingSpec {
            bay_lengths_m: vec![5.0, 8.0, 7.0],
            ..Default::default()
        };
        assert!((b.total_length_m() - 20.0).abs() < 1e-12);
        assert!((b.tributary_width_m(0) - 2.5).abs() < 1e-12);
        assert!((b.tributary_width_m(1) - 6.5).abs() < 1e-12);
        assert!((b.tributary_width_m(2) - 7.5).abs() < 1e-12);
        assert!((b.tributary_width_m(3) - 3.5).abs() < 1e-12);
    }

    #[test]
    fn girt_trimming_splits_around_door_opening() {
        let o = WallOpening {
            id: "D1".into(),
            wall: WallSide::LeftSidewall,
            offset_m: 6.0,
            sill_m: 0.0,
            width_m: 4.0,
            height_m: 4.2,
            shop_framed: true,
        };
        let segs = trimmed_girt_segments(18.0, 2.0, &[&o]);
        assert_eq!(segs, vec![(0.0, 6.0), (10.0, 18.0)]);
        let above = trimmed_girt_segments(18.0, 4.5, &[&o]);
        assert_eq!(above, vec![(0.0, 18.0)]);
    }

    #[test]
    fn endwall_opening_respects_roof_slope_envelope() {
        let mut b = BuildingSpec::default();
        b.openings.push(WallOpening {
            id: "EW1".into(),
            wall: WallSide::FrontEndwall,
            offset_m: 0.5,
            sill_m: 0.0,
            width_m: 4.0,
            height_m: 6.02, // roof at x=0.5 is 6.05 m; wait at 6.02 it fits! Let's test overflow at 6.10
            shop_framed: true,
        });
        assert!(b.validate().is_ok());
        b.openings[0].height_m = 6.20;
        assert!(b.validate().is_err());
    }

    #[test]
    fn all_flagship_presets_pass_validation() {
        for (id, title, _desc, preset) in BuildingSpec::presets() {
            let res = preset.validate();
            assert!(
                res.is_ok(),
                "Preset '{}' ({}) failed validation: {:?}",
                id,
                title,
                res.err()
            );
        }
    }
}
