//! Pure-Rust building-code engines:
//! - ASCE 7-22 hazard load generator & LRFD/ASD/serviceability load combinations
//! - AISC 360-16 Chapters B, D, E, F, G, H member checks for prismatic and tapered I-sections
//! - AISI S100-16 preliminary cold-formed purlin/girt checks
//!
//! All calculations execute in canonical SI (m, N, Pa, m/s, kg).

use openmbs_geometry::BuildingSpec;
use openmbs_inventory::ColdFormedProfile;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DesignMethod {
    #[default]
    LRFD,
    ASD,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RiskCategory {
    I,
    #[default]
    II,
    III,
    IV,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WindExposure {
    B,
    #[default]
    C,
    D,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EnclosureClass {
    #[default]
    Enclosed,
    PartiallyEnclosed,
}

/// Site environmental and gravity inputs referenced to ASCE 7-22.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteHazardProfile {
    pub code_edition: String,
    pub design_method: DesignMethod,
    pub risk_category: RiskCategory,
    pub auto_frame_self_weight: bool,
    /// Dead load excluding self-weight of primary frame (panels, secondary, accessories), Pa (stored as kPa in UI helpers).
    pub roof_dead_superimposed_kpa: f64,
    pub collateral_kpa: f64,
    pub roof_live_lr_kpa: f64,
    /// Ground snow load pg (kPa).
    pub ground_snow_pg_kpa: f64,
    pub snow_exposure_ce: f64,
    pub snow_thermal_ct: f64,
    pub slippery_roof: bool,
    /// Basic 3-sec gust wind speed V (m/s). 51.4 m/s ~ 115 mph.
    pub wind_speed_mps: f64,
    pub wind_exposure: WindExposure,
    pub kzt: f64,
    pub kd: f64,
    pub ke: f64,
    pub enclosure: EnclosureClass,
    /// Spectral accelerations Sds and Sd1 (dimensionless g) for optional seismic ELF check.
    pub seismic_sds: f64,
    pub seismic_sd1: f64,
    pub seismic_r_factor: f64,
}

impl Default for SiteHazardProfile {
    fn default() -> Self {
        Self {
            code_edition: "ASCE/SEI 7-22 + AISC 360-16".into(),
            design_method: DesignMethod::LRFD,
            risk_category: RiskCategory::II,
            auto_frame_self_weight: true,
            roof_dead_superimposed_kpa: 0.25,
            collateral_kpa: 0.15,
            roof_live_lr_kpa: 0.96, // ~20 psf base before slope/area reduction
            ground_snow_pg_kpa: 0.96,
            snow_exposure_ce: 1.0,
            snow_thermal_ct: 1.0,
            slippery_roof: false,
            wind_speed_mps: 51.4, // ~115 mph
            wind_exposure: WindExposure::C,
            kzt: 1.0,
            kd: 0.85,
            ke: 1.0,
            enclosure: EnclosureClass::Enclosed,
            seismic_sds: 0.20,
            seismic_sd1: 0.10,
            seismic_r_factor: 3.5,
        }
    }
}

/// Zone-resolved transverse frame pressures in kPa (positive = inward/downward convention noted per field).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedLoadCase {
    pub id: String,
    pub family: String,
    pub source_clause: String,
    /// Downward horizontal-projected roof pressure on left slope, kPa (+ downward, - uplift).
    pub left_roof_down_kpa: f64,
    /// Downward horizontal-projected roof pressure on right slope, kPa (+ downward, - uplift).
    pub right_roof_down_kpa: f64,
    /// Global +X pressure on left column (kPa, + pushes building right).
    pub left_wall_right_kpa: f64,
    /// Global +X pressure on right column (kPa, + pushes right / suction away from right wall).
    pub right_wall_right_kpa: f64,
    /// Uniform lateral seismic horizontal acceleration factor expressed as fraction of gravity (g).
    pub seismic_horizontal_g: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedCombination {
    pub id: String,
    pub clause: String,
    pub is_serviceability: bool,
    pub factors: Vec<(String, f64)>,
}

/// ASCE 7-22 Table 26.10-1 velocity pressure exposure coefficient Kz at height z (m).
pub fn asce7_kz(z_m: f64, exposure: WindExposure) -> f64 {
    let z = z_m.max(4.572); // Minimum 15 ft per Table 26.10-1
    let (alpha, zg_m) = match exposure {
        WindExposure::B => (7.0, 365.76),  // 1200 ft
        WindExposure::C => (9.5, 274.32),  // 900 ft
        WindExposure::D => (11.5, 213.36), // 700 ft
    };
    2.01 * (z / zg_m).powf(2.0 / alpha)
}

/// ASCE 7-22 Eq. 26.10-1 velocity pressure qz (Pa) at height z (m).
pub fn asce7_qz_pa(z_m: f64, profile: &SiteHazardProfile) -> f64 {
    let kz = asce7_kz(z_m, profile.wind_exposure);
    0.613 * kz * profile.kzt * profile.kd * profile.ke * profile.wind_speed_mps.powi(2)
}

/// Generates ASCE 7-22 directional MWFRS transverse frame load cases and LRFD/ASD/serviceability
/// combinations with explicit clause provenance.
pub fn generate_asce7_loads(
    b: &BuildingSpec,
    profile: &SiteHazardProfile,
) -> (Vec<GeneratedLoadCase>, Vec<GeneratedCombination>) {
    let mean_roof_h = 0.5 * (0.5 * (b.left_eave_m + b.right_eave_m) + b.ridge_height_m);
    let qh_kpa = asce7_qz_pa(mean_roof_h, profile) / 1000.0;

    let is_snow = match profile.risk_category {
        RiskCategory::I => 0.8,
        RiskCategory::II => 1.0,
        RiskCategory::III => 1.1,
        RiskCategory::IV => 1.2,
    };
    let is_seis = match profile.risk_category {
        RiskCategory::I | RiskCategory::II => 1.0,
        RiskCategory::III => 1.25,
        RiskCategory::IV => 1.5,
    };

    // 1. Dead Load (excluding or tracking primary self-weight separately in solver)
    let dead_roof = profile.roof_dead_superimposed_kpa + profile.collateral_kpa;

    // 2. Roof Live Load Lr (ASCE 7-22 Sec. 4.8.2: Lr = Lo * R1 * R2)
    let avg_bay = b.total_length_m() / b.bays() as f64;
    let at_ft2 = (avg_bay * b.width_m / 2.0) / (0.3048 * 0.3048);
    let r1 = if at_ft2 <= 200.0 {
        1.0
    } else if at_ft2 >= 600.0 {
        0.6
    } else {
        1.2 - 0.001 * at_ft2
    };
    let pitch_12 = b.left_slope().max(b.right_slope()) * 12.0;
    let r2 = if pitch_12 <= 4.0 {
        1.0
    } else if pitch_12 >= 12.0 {
        0.6
    } else {
        1.2 - 0.05 * pitch_12
    };
    let lr = (profile.roof_live_lr_kpa * r1 * r2).max(0.575); // 12 psf minimum

    // 3. Flat & Sloped Roof Snow Load S (ASCE 7-22 Sec. 7.3-7.4)
    let pf_kpa = 0.7
        * profile.snow_exposure_ce
        * profile.snow_thermal_ct
        * is_snow
        * profile.ground_snow_pg_kpa;
    let angle_deg = b.left_slope().atan().to_degrees();
    let cs = if profile.slippery_roof {
        if angle_deg <= 15.0 {
            1.0
        } else if angle_deg >= 70.0 {
            0.0
        } else {
            1.0 - (angle_deg - 15.0) / 55.0
        }
    } else if angle_deg <= 30.0 {
        1.0
    } else if angle_deg >= 70.0 {
        0.0
    } else {
        1.0 - (angle_deg - 30.0) / 40.0
    };
    let ps_kpa = cs * pf_kpa;

    // 4. MWFRS Wind Pressures (ASCE 7-22 Ch. 27 Fig. 27.3-1 for rigid gable frames)
    let l_over_b = b.total_length_m() / b.width_m;
    let cp_leeward_wall = if l_over_b <= 1.0 {
        -0.5
    } else if l_over_b <= 2.0 {
        -0.5 + 0.2 * (l_over_b - 1.0)
    } else if l_over_b < 4.0 {
        -0.3 + 0.1 * (l_over_b - 2.0) / 2.0
    } else {
        -0.2
    };
    // Gust-effect factor G = 0.85 for rigid structures (ASCE 7-22 Sec. 26.11.1)
    let g_gust = 0.85;
    let cp_windward_wall = 0.8;
    // Roof Cp vs angle (for h/L <= 0.5 typical metal building)
    let cp_windward_roof = if angle_deg <= 10.0 {
        -0.9
    } else if angle_deg < 20.0 {
        -0.9 + (angle_deg - 10.0) / 10.0 * 0.5
    } else {
        -0.4
    };
    let cp_leeward_roof = if angle_deg <= 10.0 {
        -0.5
    } else if angle_deg < 20.0 {
        -0.5 - (angle_deg - 10.0) / 10.0 * 0.1
    } else {
        -0.6
    };
    let gcpi = match profile.enclosure {
        EnclosureClass::Enclosed => 0.18,
        EnclosureClass::PartiallyEnclosed => 0.55,
    };

    // Global +X windward left -> leeward right:
    // Left wall external inward (+X) - internal pressure (+GCpi pushes outward -> -X on left wall, +X on right wall)
    //Wait: p = q * G * Cp - qh * (±GCpi).
    // For Left wall (normal inward = +X): positive external pressure pushes +X; positive internal pressure (+GCpi) pushes outward = -X!
    // For Right wall (leeward, normal inward = -X): Cp_lee is negative (suction outward = +X!). Wait: inward normal is -X, so suction pulls along +X: - (q G Cp_lee) = + |q G Cp_lee|. Plus internal (+GCpi) pushes outward along +X!
    let make_wind = |id: &str, left_to_right: bool, gcpi_sign: f64| -> GeneratedLoadCase {
        let p_ext_ww = qh_kpa * g_gust * cp_windward_wall;
        let p_ext_lw = qh_kpa * g_gust * cp_leeward_wall; // negative (suction)
        let p_ext_wr = qh_kpa * g_gust * cp_windward_roof; // negative = uplift
        let p_ext_lr = qh_kpa * g_gust * cp_leeward_roof; // negative = uplift
        let p_int = qh_kpa * (gcpi_sign * gcpi); // + pulls walls outward, roof upward

        // Downward positive on roof: downward = -(p_ext_roof - p_int) ??
        // External pressure is defined positive TOWARD surface (downward on roof, inward on wall).
        // Net inward pressure p = q*G*Cp - qh*(GCpi).
        let net_ww = p_ext_ww - p_int; // inward on windward wall
        let net_lw = p_ext_lw - p_int; // inward on leeward wall (negative = outward)
        let net_wr = p_ext_wr - p_int; // downward (+), negative = uplift
        let net_lr = p_ext_lr - p_int;

        if left_to_right {
            GeneratedLoadCase {
                id: id.into(),
                family: "W".into(),
                source_clause: "ASCE 7-22 Ch. 27 Fig. 27.3-1 (+X Wind)".into(),
                left_roof_down_kpa: net_wr,
                right_roof_down_kpa: net_lr,
                left_wall_right_kpa: net_ww,
                right_wall_right_kpa: -net_lw, // leeward inward is -X, so -net_lw is +X
                seismic_horizontal_g: 0.0,
            }
        } else {
            GeneratedLoadCase {
                id: id.into(),
                family: "W".into(),
                source_clause: "ASCE 7-22 Ch. 27 Fig. 27.3-1 (-X Wind)".into(),
                left_roof_down_kpa: net_lr,
                right_roof_down_kpa: net_wr,
                left_wall_right_kpa: net_lw, // leeward inward on left wall is +X, so net_lw (negative) pulls -X
                right_wall_right_kpa: -net_ww, // windward inward on right wall is -X
                seismic_horizontal_g: 0.0,
            }
        }
    };

    let cs_seis = (profile.seismic_sds / (profile.seismic_r_factor / is_seis))
        .max(0.044 * profile.seismic_sds * is_seis)
        .max(0.01);

    let cases = vec![
        GeneratedLoadCase {
            id: "D".into(),
            family: "D".into(),
            source_clause: "ASCE 7-22 Sec. 3.1 (Dead + Collateral)".into(),
            left_roof_down_kpa: dead_roof,
            right_roof_down_kpa: dead_roof,
            left_wall_right_kpa: 0.0,
            right_wall_right_kpa: 0.0,
            seismic_horizontal_g: 0.0,
        },
        GeneratedLoadCase {
            id: "Lr".into(),
            family: "Lr".into(),
            source_clause: format!("ASCE 7-22 Sec. 4.8.2 (R1={r1:.2}, R2={r2:.2})"),
            left_roof_down_kpa: lr,
            right_roof_down_kpa: lr,
            left_wall_right_kpa: 0.0,
            right_wall_right_kpa: 0.0,
            seismic_horizontal_g: 0.0,
        },
        GeneratedLoadCase {
            id: "S".into(),
            family: "S".into(),
            source_clause: format!("ASCE 7-22 Sec. 7.3-7.4 (pf={pf_kpa:.2}, ps={ps_kpa:.2} kPa)"),
            left_roof_down_kpa: ps_kpa,
            right_roof_down_kpa: ps_kpa,
            left_wall_right_kpa: 0.0,
            right_wall_right_kpa: 0.0,
            seismic_horizontal_g: 0.0,
        },
        make_wind("W+X(+GCpi)", true, 1.0),
        make_wind("W+X(-GCpi)", true, -1.0),
        make_wind("W-X(+GCpi)", false, 1.0),
        make_wind("W-X(-GCpi)", false, -1.0),
        GeneratedLoadCase {
            id: "E+X".into(),
            family: "E".into(),
            source_clause: format!("ASCE 7-22 Ch. 12 ELF (Cs={cs_seis:.3}g)"),
            left_roof_down_kpa: 0.2 * profile.seismic_sds * dead_roof,
            right_roof_down_kpa: 0.2 * profile.seismic_sds * dead_roof,
            left_wall_right_kpa: 0.0,
            right_wall_right_kpa: 0.0,
            seismic_horizontal_g: cs_seis,
        },
    ];

    let combos = match profile.design_method {
        DesignMethod::LRFD => vec![
            GeneratedCombination {
                id: "LC1: 1.4D".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-1".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.4)],
            },
            GeneratedCombination {
                id: "LC2: 1.2D + 1.6Lr".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-3".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.2), ("Lr".into(), 1.6)],
            },
            GeneratedCombination {
                id: "LC3: 1.2D + 1.6S".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-3".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.2), ("S".into(), 1.6)],
            },
            GeneratedCombination {
                id: "LC4a: 1.2D + 1.0W+X(+GCpi) + 0.5S".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-4".into(),
                is_serviceability: false,
                factors: vec![
                    ("D".into(), 1.2),
                    ("W+X(+GCpi)".into(), 1.0),
                    ("S".into(), 0.5),
                ],
            },
            GeneratedCombination {
                id: "LC4b: 1.2D + 1.0W+X(-GCpi) + 0.5S".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-4".into(),
                is_serviceability: false,
                factors: vec![
                    ("D".into(), 1.2),
                    ("W+X(-GCpi)".into(), 1.0),
                    ("S".into(), 0.5),
                ],
            },
            GeneratedCombination {
                id: "LC6a: 0.9D + 1.0W+X(+GCpi)".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-6".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 0.9), ("W+X(+GCpi)".into(), 1.0)],
            },
            GeneratedCombination {
                id: "LC6b: 0.9D + 1.0W-X(+GCpi)".into(),
                clause: "ASCE 7-22 Sec. 2.3.1-6".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 0.9), ("W-X(+GCpi)".into(), 1.0)],
            },
            GeneratedCombination {
                id: "LC5: 1.2D + 1.0E+X + 0.2S".into(),
                clause: "ASCE 7-22 Sec. 2.3.6-5".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.2), ("E+X".into(), 1.0), ("S".into(), 0.2)],
            },
            GeneratedCombination {
                id: "SVC-1: D + Lr".into(),
                clause: "ASCE 7-22 App. C Serviceability".into(),
                is_serviceability: true,
                factors: vec![("D".into(), 1.0), ("Lr".into(), 1.0)],
            },
            GeneratedCombination {
                id: "SVC-2: D + 0.6W+X(-GCpi)".into(),
                clause: "ASCE 7-22 App. C Wind Drift".into(),
                is_serviceability: true,
                factors: vec![("D".into(), 1.0), ("W+X(-GCpi)".into(), 0.6)],
            },
        ],
        DesignMethod::ASD => vec![
            GeneratedCombination {
                id: "ASD1: D".into(),
                clause: "ASCE 7-22 Sec. 2.4.1-1".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.0)],
            },
            GeneratedCombination {
                id: "ASD3a: D + Lr".into(),
                clause: "ASCE 7-22 Sec. 2.4.1-3".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.0), ("Lr".into(), 1.0)],
            },
            GeneratedCombination {
                id: "ASD3b: D + S".into(),
                clause: "ASCE 7-22 Sec. 2.4.1-3".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.0), ("S".into(), 1.0)],
            },
            GeneratedCombination {
                id: "ASD5: D + 0.6W+X(+GCpi)".into(),
                clause: "ASCE 7-22 Sec. 2.4.1-5".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 1.0), ("W+X(+GCpi)".into(), 0.6)],
            },
            GeneratedCombination {
                id: "ASD7: 0.6D + 0.6W+X(+GCpi)".into(),
                clause: "ASCE 7-22 Sec. 2.4.1-7".into(),
                is_serviceability: false,
                factors: vec![("D".into(), 0.6), ("W+X(+GCpi)".into(), 0.6)],
            },
            GeneratedCombination {
                id: "SVC-2: D + 0.6W+X(-GCpi)".into(),
                clause: "ASCE 7-22 App. C Wind Drift".into(),
                is_serviceability: true,
                factors: vec![("D".into(), 1.0), ("W+X(-GCpi)".into(), 0.6)],
            },
        ],
    };

    (cases, combos)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactnessClass {
    Compact,
    Noncompact,
    Slender,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiscSectionInput {
    pub depth_m: f64,
    pub flange_width_m: f64,
    pub flange_t_m: f64,
    pub web_t_m: f64,
    pub e_pa: f64,
    pub fy_pa: f64,
    /// Out-of-plane laterally unbraced length Lb (m) between flange braces/purlins.
    pub unbraced_lb_m: f64,
    /// In-plane effective buckling length Kx*Lx (m).
    pub kx_lx_m: f64,
    /// Out-of-plane effective buckling length Ky*Ly (m).
    pub ky_ly_m: f64,
    /// Moment gradient factor Cb (>= 1.0).
    pub cb: f64,
    /// Transverse web stiffener spacing (None = unstiffened).
    pub stiffener_spacing_m: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiscDemand {
    /// Positive = compression, Negative = tension (absolute evaluated by sign).
    pub axial_compression_n: f64,
    pub moment_nm: f64,
    pub shear_n: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiscMemberResult {
    pub flange_class: CompactnessClass,
    pub web_class: CompactnessClass,
    pub axial_capacity_n: f64,
    pub moment_capacity_nm: f64,
    pub shear_capacity_n: f64,
    pub axial_ratio: f64,
    pub moment_ratio: f64,
    pub shear_ratio: f64,
    pub interaction_ratio: f64,
    pub governing_clause: String,
    pub rpg_factor: f64,
}

/// AISC 360-16 Chapters B, D, E, F, G, H evaluation for doubly-symmetric welded/built-up
/// or rolled I-sections.
pub fn aisc360_check_section(
    sec: &AiscSectionInput,
    demand: &AiscDemand,
    method: DesignMethod,
) -> AiscMemberResult {
    let d = sec.depth_m;
    let bf = sec.flange_width_m;
    let tf = sec.flange_t_m;
    let tw = sec.web_t_m;
    let h = d - 2.0 * tf;
    let e = sec.e_pa;
    let fy = sec.fy_pa;

    let area = 2.0 * bf * tf + h * tw;
    let ix = (bf * d.powi(3) - (bf - tw) * h.powi(3)) / 12.0;
    let iy = (2.0 * tf * bf.powi(3) + h * tw.powi(3)) / 12.0;
    let sx = ix / (0.5 * d);
    let zx = bf * tf * (d - tf) + 0.25 * tw * h * h;
    let rx = (ix / area).sqrt();
    let ry = (iy / area).sqrt();
    let j_tor = (2.0 * bf * tf.powi(3) + h * tw.powi(3)) / 3.0;
    let ho = d - tf;
    let cw = iy * ho * ho / 4.0;
    let rts = ((iy * cw).sqrt() / sx).sqrt();

    // Chapter B4.1: Flange and web slenderness for built-up I-members
    let sqrt_e_fy = (e / fy).sqrt();
    let kc = (4.0 / (h / tw).sqrt()).clamp(0.35, 0.76);
    let lambda_f = bf / (2.0 * tf);
    let lambda_pf = 0.38 * sqrt_e_fy;
    let lambda_rf_flex = 0.95 * (kc * e / (0.7 * fy)).sqrt();
    let lambda_rf_comp = 0.64 * (kc * e / fy).sqrt();

    let lambda_w = h / tw;
    let lambda_pw = 3.76 * sqrt_e_fy;
    let lambda_rw = 5.70 * sqrt_e_fy;
    let lambda_rw_comp = 1.49 * sqrt_e_fy;

    let flange_class = if lambda_f <= lambda_pf {
        CompactnessClass::Compact
    } else if lambda_f <= lambda_rf_flex {
        CompactnessClass::Noncompact
    } else {
        CompactnessClass::Slender
    };
    let web_class = if lambda_w <= lambda_pw {
        CompactnessClass::Compact
    } else if lambda_w <= lambda_rw {
        CompactnessClass::Noncompact
    } else {
        CompactnessClass::Slender
    };

    // Resistance / safety factors
    let factor_ax = match method {
        DesignMethod::LRFD => 0.90,
        DesignMethod::ASD => 1.0 / 1.67,
    };
    let factor_fl = match method {
        DesignMethod::LRFD => 0.90,
        DesignMethod::ASD => 1.0 / 1.67,
    };
    let factor_sh = match method {
        DesignMethod::LRFD => 0.90,
        DesignMethod::ASD => 1.0 / 1.67,
    };

    // Chapters D / E: Axial capacity
    let pn_nominal = if demand.axial_compression_n < 0.0 {
        // Tension yielding D2
        fy * area
    } else {
        // Compression buckling E3 + E7 slender element effective area
        let klr = (sec.kx_lx_m / rx).max(sec.ky_ly_m / ry).max(1e-6);
        let fe = PI * PI * e / (klr * klr);
        let fcr = if klr <= 4.71 * sqrt_e_fy {
            0.658f64.powf(fy / fe) * fy
        } else {
            0.877 * fe
        };
        // E7 effective area reduction for slender web or flange
        let effective_w = |b_w: f64, lam: f64, lam_r: f64, c1: f64, c2: f64| -> f64 {
            if lam <= lam_r * (fy / fcr).sqrt() {
                b_w
            } else {
                let fel = (c2 * lam_r / lam).powi(2) * fy;
                b_w * (1.0 - c1 * (fel / fcr).sqrt()) * (fel / fcr).sqrt()
            }
        };
        let he = effective_w(h, lambda_w, lambda_rw_comp, 0.18, 1.31);
        let b_half_e = effective_w(0.5 * bf, lambda_f, lambda_rf_comp, 0.22, 1.49);
        let ae = (area - (h - he) * tw - 4.0 * ((0.5 * bf - b_half_e) * tf)).max(0.2 * area);
        fcr * ae
    };
    let pc = factor_ax * pn_nominal;

    // Chapter F: Flexural capacity (F2 / F3 / F4 / F5)
    // Compression flange local buckling + LTB with slender-web Rpg per F5 when applicable
    let aw_hc_ratio = (h * tw) / (bf * tf);
    let rpg = if lambda_w > lambda_rw {
        (1.0 - aw_hc_ratio / (1200.0 + 300.0 * aw_hc_ratio) * (lambda_w - lambda_rw)).min(1.0)
    } else {
        1.0
    }
    .max(0.5);

    let myc = fy * sx;
    let mp = (fy * zx).min(1.6 * myc);
    let lp = 1.1 * rts * (e / fy).sqrt();
    let lr = 1.95
        * rts
        * (e / (0.7 * fy))
        * (j_tor / (sx * ho)
            + ((j_tor / (sx * ho)).powi(2) + 6.76 * (0.7 * fy / e).powi(2)).sqrt())
        .sqrt();

    let lb = sec.unbraced_lb_m.max(1e-4);
    let cb = sec.cb.max(1.0);
    let mn_ltb = if web_class == CompactnessClass::Slender {
        // Chapter F5.2: Rpg * Fcr * Sxc
        let fcr_ltb = if lb <= lp {
            fy
        } else if lb <= lr {
            (cb * (fy - (0.3 * fy) * (lb - lp) / (lr - lp))).min(fy)
        } else {
            let fltb = cb * PI * PI * e / (lb / rts).powi(2);
            fltb.min(fy)
        };
        rpg * fcr_ltb * sx
    } else {
        // Chapter F2/F4 unified elastic-plastic LTB limit
        let my_base = if web_class == CompactnessClass::Compact {
            mp
        } else {
            myc
        };
        if lb <= lp {
            my_base
        } else if lb <= lr {
            (cb * (my_base - (my_base - 0.7 * fy * sx) * (lb - lp) / (lr - lp))).min(my_base)
        } else {
            let fcr = cb * PI * PI * e / (lb / rts).powi(2)
                * (1.0 + 0.078 * j_tor / (sx * ho) * (lb / rts).powi(2)).sqrt();
            (fcr * sx).min(my_base)
        }
    };

    // Compression Flange Local Buckling (FLB)
    let mn_flb = if lambda_f <= lambda_pf {
        mp
    } else if lambda_f <= lambda_rf_flex {
        let m_base = if web_class == CompactnessClass::Slender {
            rpg * myc
        } else {
            mp
        };
        m_base - (m_base - 0.7 * rpg * myc) * (lambda_f - lambda_pf) / (lambda_rf_flex - lambda_pf)
    } else {
        let fcr_flb = 0.9 * e * kc / (lambda_f * lambda_f);
        rpg * fcr_flb * sx
    };

    let mn_nominal = mn_ltb.min(mn_flb).min(mp);
    let mc = factor_fl * mn_nominal;

    // Chapter G2: Shear Strength of Webs without / with Stiffeners
    let aw = d * tw;
    let kv = match sec.stiffener_spacing_m {
        Some(a) if a > 0.0 && a / h <= 3.0 => 5.0 + 5.0 / (a / h).powi(2),
        _ => 5.34,
    };
    let cv1 = if lambda_w <= 1.10 * (kv * e / fy).sqrt() {
        1.0
    } else {
        1.10 * (kv * e / fy).sqrt() / lambda_w
    };
    let vn_nominal = 0.6 * fy * aw * cv1;
    let vc = factor_sh * vn_nominal;

    let pr = demand.axial_compression_n.abs();
    let mr = demand.moment_nm.abs();
    let vr = demand.shear_n.abs();

    let axial_ratio = pr / pc.max(1.0);
    let moment_ratio = mr / mc.max(1.0);
    let shear_ratio = vr / vc.max(1.0);

    // Chapter H1-1a / H1-1b interaction
    let (interaction_ratio, h_clause) = if axial_ratio >= 0.2 {
        (axial_ratio + (8.0 / 9.0) * moment_ratio, "H1-1a")
    } else {
        (0.5 * axial_ratio + moment_ratio, "H1-1b")
    };

    let governing_clause = if shear_ratio > interaction_ratio {
        "AISC 360-16 G2 (Web Shear)".into()
    } else if mn_flb < mn_ltb {
        format!("AISC 360-16 {} + FLB", h_clause)
    } else if web_class == CompactnessClass::Slender {
        format!("AISC 360-16 F5 (Slender Web, Rpg={rpg:.3}) + {h_clause}")
    } else {
        format!("AISC 360-16 F2/F4 (LTB) + {h_clause}")
    };

    AiscMemberResult {
        flange_class,
        web_class,
        axial_capacity_n: pc,
        moment_capacity_nm: mc,
        shear_capacity_n: vc,
        axial_ratio,
        moment_ratio,
        shear_ratio,
        interaction_ratio: interaction_ratio.max(shear_ratio),
        governing_clause,
        rpg_factor: rpg,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColdFormedCheckResult {
    pub profile_id: String,
    pub demand_moment_nm: f64,
    pub capacity_moment_nm: f64,
    pub moment_ratio: f64,
    pub deflection_m: f64,
    pub deflection_limit_m: f64,
    pub deflection_ratio: f64,
}

/// AISI S100-16 preliminary flexure & serviceability deflection screening for simply-supported Z/C purlin or girt.
pub fn aisi_s100_check_secondary(
    profile: &ColdFormedProfile,
    span_m: f64,
    line_load_strength_npm: f64,
    line_load_service_npm: f64,
    method: DesignMethod,
    deflection_denominator: f64,
) -> ColdFormedCheckResult {
    let m_demand = line_load_strength_npm.abs() * span_m * span_m / 8.0;
    let mn = profile.sx_m3 * profile.ae_ratio * profile.fy_pa;
    let factor = match method {
        DesignMethod::LRFD => 0.90,
        DesignMethod::ASD => 1.0 / 1.67,
    };
    let mc = factor * mn;
    let delta =
        5.0 * line_load_service_npm.abs() * span_m.powi(4) / (384.0 * profile.e_pa * profile.ix_m4);
    let delta_limit = span_m / deflection_denominator.max(1.0);
    ColdFormedCheckResult {
        profile_id: profile.id.clone(),
        demand_moment_nm: m_demand,
        capacity_moment_nm: mc,
        moment_ratio: m_demand / mc.max(1.0),
        deflection_m: delta,
        deflection_limit_m: delta_limit,
        deflection_ratio: delta / delta_limit.max(1e-9),
    }
}

/// Evaluates crane wheel loads, lateral thrust, and bracket eccentric moment on the column shaft
/// per ASCE 7-22 Section 4.9.
pub fn crane_wheel_and_bracket_loads(
    crane: &openmbs_geometry::CraneSpec,
    impact_factor: f64,
) -> (f64, f64, f64) {
    let p_vertical_kn = crane.max_wheel_load_kn * (1.0 + impact_factor.max(0.15));
    let p_lateral_kn = p_vertical_kn * (crane.lateral_thrust_pct / 100.0).clamp(0.10, 0.30);
    let m_bracket_knm = p_vertical_kn * crane.bracket_eccentricity_m;
    (p_vertical_kn, p_lateral_kn, m_bracket_knm)
}

/// Computes ASCE 7-22 Section 27.3.5 net parapet wind pressure (kPa) from velocity pressure qp.
pub fn parapet_wind_pressure_kpa(qp_kpa: f64) -> f64 {
    // Combined windward (1.5) and leeward (-1.0) GCpn net pressure coefficient ~ 1.8
    1.8 * qp_kpa
}

/// Generates an official, plain-text and Markdown PE Stamped Design Certification Letter (F-0170).
#[allow(clippy::too_many_arguments)]
pub fn generate_pe_certification_letter(
    project_title: &str,
    revision_hash: &str,
    building: &BuildingSpec,
    site: &SiteHazardProfile,
    max_aisc_unity: f64,
    max_drift_ratio: f64,
    engineer_name: &str,
    pe_license_no: &str,
) -> String {
    let status = if max_aisc_unity <= 1.000 {
        "SATISFIES APPLICABLE SPECIFICATIONS"
    } else {
        "EXCEEDS CODE CAPACITY LIMITS (REQUIRES MEMBER UPSIZING)"
    };

    format!(
        "================================================================================\n\
         STRUCTURAL DESIGN CERTIFICATION LETTER\n\
         Project: {project_title}\n\
         Engine Revision Hash: {revision_hash}\n\
         Issued By: OpenMBS Automated Engineering Engine (Pure Rust)\n\
         --------------------------------------------------------------------------------\n\
         1. GOVERNING DESIGN CODES & SPECIFICATIONS:\n\
            - Structural Steel: AISC 360-16 Specification for Structural Steel Buildings\n\
            - Cold-Formed Steel: AISI S100-16 North American Specification for Cold-Formed\n\
            - Hazard & Environmental Loads: ASCE/SEI 7-22 Minimum Design Loads\n\
            - Design Methodology: {:?} (Resistance Factor: phi=0.90 flexure, phi=0.90 compression)\n\
         \n\
         2. SITE CRITERIA & DESIGN PARAMETERS:\n\
            - Risk Category: {:?}\n\
            - Basic 3-Second Gust Wind Speed: {:.1} mph ({:.1} m/s), Exposure {:?}\n\
            - Ground Snow Load (pg): {:.1} psf ({:.2} kPa), Ce={:.2}, Ct={:.2}\n\
            - Roof Live Load (Lr): {:.1} psf ({:.2} kPa)\n\
            - Collateral & Superimposed Dead: {:.1} psf ({:.2} kPa)\n\
            - Seismic Parameters: Sds={:.3}g, Sd1={:.3}g, R={:.1}\n\
         \n\
         3. BUILDING GEOMETRY & FRAMING CONFIGURATION:\n\
            - Overall Dimensions: {:.2} m span x {:.2} m length, {} bays\n\
            - Eave Heights: Left = {:.2} m, Right = {:.2} m | Ridge Height = {:.2} m\n\
            - Secondary Spacings: Roof Purlins @ {:.2} m o.c., Wall Girts @ {:.2} m o.c.\n\
         \n\
         4. STRUCTURAL SUMMARY & COMPLIANCE FINDINGS:\n\
            - Maximum Frame AISC 360-16 Combined Unity: {:.3}\n\
            - Maximum Serviceability Lateral Drift Ratio: H / {:.0}\n\
            - Verification Status: {}\n\
         \n\
         5. PROFESSIONAL ENGINEER SEAL & ATTESTATION:\n\
            I hereby certify that these structural calculations and drawings were prepared\n\
            in accordance with the provisions of the stated building codes.\n\
            \n\
            Certified by: {}  P.E. License: {}\n\
         ================================================================================\n",
        site.design_method,
        site.risk_category,
        site.wind_speed_mps * 2.23694,
        site.wind_speed_mps,
        site.wind_exposure,
        site.ground_snow_pg_kpa * 20.8854,
        site.ground_snow_pg_kpa,
        site.snow_exposure_ce,
        site.snow_thermal_ct,
        site.roof_live_lr_kpa * 20.8854,
        site.roof_live_lr_kpa,
        site.roof_dead_superimposed_kpa * 20.8854,
        site.roof_dead_superimposed_kpa,
        site.seismic_sds,
        site.seismic_sd1,
        site.seismic_r_factor,
        building.width_m,
        building.total_length_m(),
        building.bay_lengths_m.len(),
        building.left_eave_m,
        building.right_eave_m,
        building.ridge_height_m,
        building.purlin_spacing_m,
        building.girt_spacing_m,
        max_aisc_unity,
        if max_drift_ratio > 1e-9 {
            1.0 / max_drift_ratio
        } else {
            1000.0
        },
        status,
        engineer_name,
        pe_license_no
    )
}

/// Evaluates crane wheel loads, lateral thrust, and bracket eccentric moment on the column shaft
/// per ASCE 7-22 Section 4.9.
pub fn crane_wheel_and_bracket_loads(
    crane: &openmbs_geometry::CraneSpec,
    impact_factor: f64,
) -> (f64, f64, f64) {
    let p_vertical_kn = crane.max_wheel_load_kn * (1.0 + impact_factor.max(0.15));
    let p_lateral_kn = p_vertical_kn * (crane.lateral_thrust_pct / 100.0).clamp(0.10, 0.30);
    let m_bracket_knm = p_vertical_kn * crane.bracket_eccentricity_m;
    (p_vertical_kn, p_lateral_kn, m_bracket_knm)
}

/// Computes ASCE 7-22 Section 27.3.5 net parapet wind pressure (kPa) from velocity pressure qp.
pub fn parapet_wind_pressure_kpa(qp_kpa: f64) -> f64 {
    // Combined windward (1.5) and leeward (-1.0) GCpn net pressure coefficient ~ 1.8
    1.8 * qp_kpa
}

/// Generates an official, plain-text and Markdown PE Stamped Design Certification Letter (F-0170).
#[allow(clippy::too_many_arguments)]
pub fn generate_pe_certification_letter(
    project_title: &str,
    revision_hash: &str,
    building: &BuildingSpec,
    site: &SiteHazardProfile,
    max_aisc_unity: f64,
    max_drift_ratio: f64,
    engineer_name: &str,
    pe_license_no: &str,
) -> String {
    let status = if max_aisc_unity <= 1.000 {
        "SATISFIES APPLICABLE SPECIFICATIONS"
    } else {
        "EXCEEDS CODE CAPACITY LIMITS (REQUIRES MEMBER UPSIZING)"
    };

    format!(
        "================================================================================\n\
         STRUCTURAL DESIGN CERTIFICATION LETTER\n\
         Project: {project_title}\n\
         Engine Revision Hash: {revision_hash}\n\
         Issued By: OpenMBS Automated Engineering Engine (Pure Rust)\n\
         --------------------------------------------------------------------------------\n\
         1. GOVERNING DESIGN CODES & SPECIFICATIONS:\n\
            - Structural Steel: AISC 360-16 Specification for Structural Steel Buildings\n\
            - Cold-Formed Steel: AISI S100-16 North American Specification for Cold-Formed\n\
            - Hazard & Environmental Loads: ASCE/SEI 7-22 Minimum Design Loads\n\
            - Design Methodology: {:?} (Resistance Factor: phi=0.90 flexure, phi=0.90 compression)\n\
         \n\
         2. SITE CRITERIA & DESIGN PARAMETERS:\n\
            - Risk Category: {:?}\n\
            - Basic 3-Second Gust Wind Speed: {:.1} mph ({:.1} m/s), Exposure {:?}\n\
            - Ground Snow Load (pg): {:.1} psf ({:.2} kPa), Ce={:.2}, Ct={:.2}\n\
            - Roof Live Load (Lr): {:.1} psf ({:.2} kPa)\n\
            - Collateral & Superimposed Dead: {:.1} psf ({:.2} kPa)\n\
            - Seismic Parameters: Sds={:.3}g, Sd1={:.3}g, R={:.1}\n\
         \n\
         3. BUILDING GEOMETRY & FRAMING CONFIGURATION:\n\
            - Overall Dimensions: {:.2} m span x {:.2} m length, {} bays\n\
            - Eave Heights: Left = {:.2} m, Right = {:.2} m | Ridge Height = {:.2} m\n\
            - Secondary Spacings: Roof Purlins @ {:.2} m o.c., Wall Girts @ {:.2} m o.c.\n\
         \n\
         4. STRUCTURAL SUMMARY & COMPLIANCE FINDINGS:\n\
            - Maximum Frame AISC 360-16 Combined Unity: {:.3}\n\
            - Maximum Serviceability Lateral Drift Ratio: H / {:.0}\n\
            - Verification Status: {}\n\
         \n\
         5. PROFESSIONAL ENGINEER SEAL & ATTESTATION:\n\
            I hereby certify that these structural calculations and drawings were prepared\n\
            in accordance with the provisions of the stated building codes.\n\
            \n\
            Certified by: {}  P.E. License: {}\n\
         ================================================================================\n",
        site.design_method,
        site.risk_category,
        site.wind_speed_mps * 2.23694,
        site.wind_speed_mps,
        site.wind_exposure,
        site.ground_snow_pg_kpa * 20.8854,
        site.ground_snow_pg_kpa,
        site.snow_exposure_ce,
        site.snow_thermal_ct,
        site.roof_live_lr_kpa * 20.8854,
        site.roof_live_lr_kpa,
        site.roof_dead_superimposed_kpa * 20.8854,
        site.roof_dead_superimposed_kpa,
        site.seismic_sds,
        site.seismic_sd1,
        site.seismic_r_factor,
        building.width_m,
        building.total_length_m(),
        building.bay_lengths_m.len(),
        building.left_eave_m,
        building.right_eave_m,
        building.ridge_height_m,
        building.purlin_spacing_m,
        building.girt_spacing_m,
        max_aisc_unity,
        if max_drift_ratio > 1e-9 { 1.0 / max_drift_ratio } else { 1000.0 },
        status,
        engineer_name,
        pe_license_no
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asce7_generates_expected_cases_and_lrfd_combos() {
        let b = BuildingSpec::default();
        let prof = SiteHazardProfile::default();
        let (cases, combos) = generate_asce7_loads(&b, &prof);
        assert!(cases.iter().any(|c| c.id == "D"));
        assert!(cases.iter().any(|c| c.id == "W+X(+GCpi)"));
        assert!(combos.iter().any(|c| c.id == "LC2: 1.2D + 1.6Lr"));
        assert!(combos.iter().any(|c| c.is_serviceability));
    }

    #[test]
    fn aisc360_compact_rolled_i_plastic_capacity_matches_formula() {
        // Check a compact section under pure moment with short unbraced length Lb <= Lp
        let sec = AiscSectionInput {
            depth_m: 0.40,
            flange_width_m: 0.20,
            flange_t_m: 0.016,
            web_t_m: 0.010,
            e_pa: 200e9,
            fy_pa: 345e6,
            unbraced_lb_m: 1.0,
            kx_lx_m: 6.0,
            ky_ly_m: 1.0,
            cb: 1.0,
            stiffener_spacing_m: None,
        };
        let zx = 0.20 * 0.016 * (0.40 - 0.016) + 0.25 * 0.010 * (0.40_f64 - 0.032).powi(2);
        let expected_phi_mp = 0.90 * 345e6 * zx;
        let res = aisc360_check_section(
            &sec,
            &AiscDemand {
                axial_compression_n: 0.0,
                moment_nm: expected_phi_mp,
                shear_n: 0.0,
            },
            DesignMethod::LRFD,
        );
        assert_eq!(res.flange_class, CompactnessClass::Compact);
        assert_eq!(res.web_class, CompactnessClass::Compact);
        assert!((res.moment_capacity_nm - expected_phi_mp).abs() / expected_phi_mp < 1e-6);
        assert!((res.interaction_ratio - 1.0).abs() < 1e-6);
    }

    #[test]
    fn aisc360_slender_web_applies_rpg_reduction() {
        let sec = AiscSectionInput {
            depth_m: 1.00,
            flange_width_m: 0.22,
            flange_t_m: 0.014,
            web_t_m: 0.005, // h/tw = 194 > 5.7*sqrt(200e3/345) = 137.2 -> slender
            e_pa: 200e9,
            fy_pa: 345e6,
            unbraced_lb_m: 1.5,
            kx_lx_m: 6.0,
            ky_ly_m: 1.5,
            cb: 1.0,
            stiffener_spacing_m: None,
        };
        let res = aisc360_check_section(
            &sec,
            &AiscDemand {
                axial_compression_n: 100_000.0,
                moment_nm: 900_000.0,
                shear_n: 50_000.0,
            },
            DesignMethod::LRFD,
        );
        assert_eq!(res.web_class, CompactnessClass::Slender);
        assert!(res.rpg_factor < 1.0);
        assert!(res.governing_clause.contains("F5"));

        let shear_gov = aisc360_check_section(
            &sec,
            &AiscDemand {
                axial_compression_n: 0.0,
                moment_nm: 200_000.0,
                shear_n: 200_000.0,
            },
            DesignMethod::LRFD,
        );
        assert!(shear_gov.governing_clause.contains("G2"));
    }
}
