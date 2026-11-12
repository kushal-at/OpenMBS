#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use openmbs_core::{
    codes::{
        DesignMethod, EnclosureClass, RiskCategory, WindExposure, generate_pe_certification_letter,
    },
    geometry::{
        CanopySpec, CraneSpec, EndwallType, LeanToSpec, ParapetSpec, WallOpening, WallSide,
    },
    inventory::{AnchorSupplyScope, dbcheck},
    peb::{
        EngineeringProject, EngineeringRun, export_peb_bundle, format_verify_report,
        optimize_peb_frame, process_peb,
    },
    storage::atomic_write,
    units::{Length, Pressure, Speed, UnitSystem},
};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MbsTab {
    Shape,
    BaySpacing,
    CodesLoads,
    RigidFrames,
    Openings,
    Extensions,
    Inventory,
    Costing,
    DrawingSheets,
    VerifyReports,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewportMode {
    Isometric3D,
    Elevation2D,
    RoofPlan2D,
}

struct App {
    project: EngineeringProject,
    saved_snapshot: EngineeringProject,
    run: Option<EngineeringRun>,
    path: Option<PathBuf>,
    active_tab: MbsTab,
    view_mode: ViewportMode,
    message: String,
    undo: Vec<EngineeringProject>,
    redo: Vec<EngineeringProject>,
    yaw: f32,
    zoom: f32,
    show_frames: bool,
    show_purlins: bool,
}

impl Default for App {
    fn default() -> Self {
        let project = EngineeringProject::default();
        Self {
            saved_snapshot: project.clone(),
            project,
            run: None,
            path: None,
            active_tab: MbsTab::Shape,
            view_mode: ViewportMode::Isometric3D,
            message: "OpenMBS ready. Configure building tabs or click Auto-Size / Calculate."
                .into(),
            undo: vec![],
            redo: vec![],
            yaw: 0.62,
            zoom: 1.0,
            show_frames: true,
            show_purlins: true,
        }
    }
}

fn num_m_field(
    ui: &mut egui::Ui,
    label: &str,
    value_m: &mut f64,
    sys: UnitSystem,
    range_m: std::ops::RangeInclusive<f64>,
) {
    let mut disp = Length::from_m(*value_m).in_system(sys);
    let lo = Length::from_m(*range_m.start()).in_system(sys);
    let hi = Length::from_m(*range_m.end()).in_system(sys);
    ui.horizontal(|ui| {
        ui.label(format!("{label} ({})", sys.length_unit()));
        if ui
            .add(
                egui::DragValue::new(&mut disp)
                    .range(lo..=hi)
                    .speed(0.1)
                    .max_decimals(3),
            )
            .changed()
        {
            *value_m = Length::from_system(disp, sys).m();
        }
    });
}

fn num_kpa_field(
    ui: &mut egui::Ui,
    label: &str,
    value_kpa: &mut f64,
    sys: UnitSystem,
    range_kpa: std::ops::RangeInclusive<f64>,
) {
    let mut disp = Pressure::from_kpa(*value_kpa).pressure_in_system(sys);
    let lo = Pressure::from_kpa(*range_kpa.start()).pressure_in_system(sys);
    let hi = Pressure::from_kpa(*range_kpa.end()).pressure_in_system(sys);
    ui.horizontal(|ui| {
        ui.label(format!("{label} ({})", sys.pressure_unit()));
        if ui
            .add(
                egui::DragValue::new(&mut disp)
                    .range(lo..=hi)
                    .speed(0.2)
                    .max_decimals(2),
            )
            .changed()
        {
            *value_kpa = Pressure::from_pressure_system(disp, sys).kpa();
        }
    });
}

fn num_f64(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val).range(range).speed(speed));
    });
}

impl App {
    fn dirty(&self) -> bool {
        self.project != self.saved_snapshot
    }

    fn may_discard(&mut self) -> bool {
        if !self.dirty() {
            return true;
        }
        let confirm = rfd::MessageDialog::new()
            .set_title("Unsaved Changes")
            .set_description("You have unsaved changes. Discard them?")
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        confirm == rfd::MessageDialogResult::Yes
    }

    fn process(&mut self) {
        match process_peb(&self.project) {
            Ok(run) => {
                let max_ratio = run.max_aisc_ratio;
                let max_drift = run.max_drift_ratio;
                self.run = Some(run);
                self.message = format!(
                    "Calculated: Max AISC Unity = {:.3} | Drift = H/{:.0}",
                    max_ratio,
                    if max_drift > 1e-9 {
                        1.0 / max_drift
                    } else {
                        1000.0
                    }
                );
            }
            Err(e) => {
                self.message = format!("Calculation Error: {e}");
            }
        }
    }

    fn auto_size(&mut self) {
        match optimize_peb_frame(&self.project) {
            Ok((col, raf)) => {
                self.project.column_section_id = col.clone();
                self.project.rafter_section_id = raf;
                self.process();
                self.message = format!("Auto-sized frame sections: {col}");
            }
            Err(e) => self.message = format!("Auto-Size Error: {e}"),
        }
    }

    fn save(&mut self, as_new: bool) {
        let path = if as_new || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("OpenMBS PEB Project", &["json"])
                .set_file_name("project.peb.openmbs.json")
                .save_file()
        } else {
            self.path.clone()
        };
        if let Some(path) = path {
            let res = serde_json::to_vec_pretty(&self.project)
                .map_err(|e| e.to_string())
                .and_then(|bytes| atomic_write(&path, &bytes));
            match res {
                Ok(()) => {
                    self.saved_snapshot = self.project.clone();
                    self.message = format!("Saved to {}", path.display());
                    self.path = Some(path);
                }
                Err(e) => self.message = format!("Save Error: {e}"),
            }
        }
    }

    fn open(&mut self) {
        if !self.may_discard() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("OpenMBS Project", &["json"])
            .pick_file()
        {
            let loaded = std::fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    if let Ok(v2) = serde_json::from_slice::<EngineeringProject>(&bytes) {
                        Ok(v2)
                    } else {
                        let v1 = openmbs_core::storage::ProjectFile::load(&path)?.latest()?;
                        Ok(EngineeringProject::from_v1(&v1))
                    }
                });
            match loaded {
                Ok(p) => {
                    self.undo.clear();
                    self.redo.clear();
                    self.project = p.clone();
                    self.saved_snapshot = p;
                    self.path = Some(path);
                    self.run = None;
                    self.message = "Project loaded successfully.".into();
                }
                Err(e) => self.message = format!("Open Error: {e}"),
            }
        }
    }

    fn export(&mut self) {
        if !self
            .run
            .as_ref()
            .is_some_and(|r| r.is_current(&self.project))
        {
            self.process();
        }
        if let Some(r) = &self.run
            && r.is_current(&self.project)
            && let Some(parent) = rfd::FileDialog::new()
                .set_title("Choose parent folder for OpenMBS export bundle")
                .pick_folder()
        {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let path = parent.join(format!("openmbs-peb-{}-{stamp}", &r.revision[..8]));
            self.message = match export_peb_bundle(&self.project, r, &path) {
                Ok(()) => format!(
                    "Exported IFC4, DXF, SVGs, Verify & BOM to {}",
                    path.display()
                ),
                Err(e) => format!("Export Error: {e}"),
            };
        }
    }

    fn draw_tab_content(&mut self, ui: &mut egui::Ui) {
        let sys = self.project.unit_system;
        match self.active_tab {
            MbsTab::Shape => {
                ui.heading("Shape (UI-0002: Building Geometry)");
                ui.label("Project Name:");
                ui.text_edit_singleline(&mut self.project.name);
                ui.label("Customer / Account:");
                ui.text_edit_singleline(&mut self.project.customer);
                ui.separator();

                ui.label("Quick Geometry Presets:");
                ui.horizontal(|ui| {
                    if ui.button("Standard 80x120 ft").clicked() {
                        self.project.building.width_m = 24.384;
                        self.project.building.bay_lengths_m = vec![6.096; 6];
                        self.project.building.left_eave_m = 6.096;
                        self.project.building.right_eave_m = 6.096;
                        self.project.building.ridge_x_m = 12.192;
                        self.project.building.ridge_height_m = 7.112;
                    }
                    if ui.button("Single Slope 60x100 ft").clicked() {
                        self.project.building.width_m = 18.288;
                        self.project.building.bay_lengths_m = vec![6.096; 5];
                        self.project.building.left_eave_m = 4.876;
                        self.project.building.right_eave_m = 6.705;
                        self.project.building.ridge_x_m = 18.288;
                        self.project.building.ridge_height_m = 6.705;
                    }
                });
                ui.separator();

                let b = &mut self.project.building;
                num_m_field(
                    ui,
                    "Building Width (Transverse Span)",
                    &mut b.width_m,
                    sys,
                    2.0..=200.0,
                );
                num_m_field(
                    ui,
                    "Left Sidewall Eave Height",
                    &mut b.left_eave_m,
                    sys,
                    2.0..=50.0,
                );
                num_m_field(
                    ui,
                    "Right Sidewall Eave Height",
                    &mut b.right_eave_m,
                    sys,
                    2.0..=50.0,
                );
                num_m_field(
                    ui,
                    "Peak Offset (Ridge X from Left)",
                    &mut b.ridge_x_m,
                    sys,
                    0.5..=199.5,
                );
                num_m_field(
                    ui,
                    "Peak / Ridge Elevation",
                    &mut b.ridge_height_m,
                    sys,
                    2.0..=60.0,
                );

                let slope = b.left_slope();
                ui.label(format!(
                    "Calculated Roof Pitch: {:.2}:12 ({:.1} deg)",
                    slope * 12.0,
                    slope.atan().to_degrees()
                ));

                ui.separator();
                ui.checkbox(
                    &mut b.fixed_bases,
                    "Fixed Column Bases (unchecked = Pinned)",
                );

                ui.horizontal(|ui| {
                    ui.label("Endwall Framing Type:");
                    ui.selectable_value(
                        &mut b.endwall_type,
                        EndwallType::PostAndBeam,
                        "Post & Beam (Bearing)",
                    );
                    ui.selectable_value(
                        &mut b.endwall_type,
                        EndwallType::ExpandableRigidFrame,
                        "Expandable Rigid Frame",
                    );
                });
                ui.add(
                    egui::Slider::new(&mut b.endwall_post_bays, 1..=12).text("Endwall Post Bays"),
                );
            }

            MbsTab::BaySpacing => {
                ui.heading("Bay Spacing (UI-0003 & UI-0004)");
                let b = &mut self.project.building;
                let total_len = b.total_length_m();
                ui.label(format!(
                    "Total Building Length: {:.2} {} across {} bays",
                    Length::from_m(total_len).in_system(sys),
                    sys.length_unit(),
                    b.bay_lengths_m.len()
                ));
                ui.separator();

                ui.label("Roof Bay Spacing Table:");
                egui::Grid::new("bay_table").striped(true).show(ui, |ui| {
                    ui.strong("Bay #");
                    ui.strong(format!("Length ({})", sys.length_unit()));
                    ui.strong("X-Brace");
                    ui.strong("Action");
                    ui.end_row();

                    let mut remove_idx = None;
                    let can_remove = b.bay_lengths_m.len() > 1;
                    for (i, bay_len) in b.bay_lengths_m.iter_mut().enumerate() {
                        ui.label(format!("Bay {}", i + 1));
                        let mut disp = Length::from_m(*bay_len).in_system(sys);
                        if ui
                            .add(
                                egui::DragValue::new(&mut disp)
                                    .range(1.0..=150.0)
                                    .speed(0.2),
                            )
                            .changed()
                        {
                            *bay_len = Length::from_system(disp, sys).m().clamp(0.5, 50.0);
                        }

                        let mut is_braced = b.braced_bays.contains(&i);
                        if ui.checkbox(&mut is_braced, "").changed() {
                            if is_braced {
                                b.braced_bays.push(i);
                            } else {
                                b.braced_bays.retain(|&x| x != i);
                            }
                        }

                        if can_remove && ui.small_button("Delete").clicked() {
                            remove_idx = Some(i);
                        }
                        ui.end_row();
                    }

                    if let Some(idx) = remove_idx {
                        b.bay_lengths_m.remove(idx);
                        b.braced_bays.retain(|&x| x < b.bay_lengths_m.len());
                    }
                });

                if b.bay_lengths_m.len() < 100 && ui.button("+ Insert Bay").clicked() {
                    let last = *b.bay_lengths_m.last().unwrap_or(&6.0);
                    b.bay_lengths_m.push(last);
                }

                ui.separator();
                ui.heading("Secondary Framing Layout");
                num_m_field(
                    ui,
                    "Roof Purlin Spacing o.c.",
                    &mut b.purlin_spacing_m,
                    sys,
                    0.3..=4.0,
                );
                num_m_field(
                    ui,
                    "Purlin Lap Length Allowance",
                    &mut b.purlin_lap_m,
                    sys,
                    0.0..=2.0,
                );
                num_m_field(
                    ui,
                    "Sidewall Girt Spacing o.c.",
                    &mut b.girt_spacing_m,
                    sys,
                    0.3..=4.0,
                );
                num_m_field(
                    ui,
                    "Flange Brace Unbraced Length Lb",
                    &mut b.flange_brace_spacing_m,
                    sys,
                    0.5..=10.0,
                );
            }

            MbsTab::CodesLoads => {
                ui.heading("Code, Loads (UI-0006)");
                let h = &mut self.project.hazard;

                ui.horizontal(|ui| {
                    ui.label("Design Method:");
                    ui.selectable_value(
                        &mut h.design_method,
                        DesignMethod::LRFD,
                        "LRFD (AISC 360-16)",
                    );
                    ui.selectable_value(
                        &mut h.design_method,
                        DesignMethod::ASD,
                        "ASD (AISC 360-16)",
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Risk Category:");
                    ui.selectable_value(&mut h.risk_category, RiskCategory::I, "I (Ag)");
                    ui.selectable_value(&mut h.risk_category, RiskCategory::II, "II (Standard)");
                    ui.selectable_value(&mut h.risk_category, RiskCategory::III, "III (Assembly)");
                    ui.selectable_value(&mut h.risk_category, RiskCategory::IV, "IV (Essential)");
                });

                ui.checkbox(
                    &mut self.project.second_order_pdelta,
                    "Iterative 2nd-Order P-Delta Analysis",
                );
                ui.checkbox(
                    &mut h.auto_frame_self_weight,
                    "Automatic Frame Self-Weight Addition",
                );

                ui.separator();
                ui.strong("Environmental & Hazard Loads (ASCE 7-22):");
                let mut v_disp = Speed::from_mps(h.wind_speed_mps).in_system(sys);
                ui.horizontal(|ui| {
                    ui.label(format!("Basic Wind Speed V ({})", sys.speed_unit()));
                    if ui
                        .add(
                            egui::DragValue::new(&mut v_disp)
                                .range(30.0..=250.0)
                                .speed(1.0),
                        )
                        .changed()
                    {
                        h.wind_speed_mps = match sys {
                            UnitSystem::MetricSI => v_disp,
                            UnitSystem::ImperialUS => Speed::from_mph(v_disp).mps(),
                        };
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Terrain Exposure:");
                    ui.selectable_value(
                        &mut h.wind_exposure,
                        WindExposure::B,
                        "B (Urban/Suburban)",
                    );
                    ui.selectable_value(&mut h.wind_exposure, WindExposure::C, "C (Open Terrain)");
                    ui.selectable_value(&mut h.wind_exposure, WindExposure::D, "D (Flat Coast)");
                });

                ui.horizontal(|ui| {
                    ui.label("Building Enclosure:");
                    ui.selectable_value(
                        &mut h.enclosure,
                        EnclosureClass::Enclosed,
                        "Enclosed (GCpi = +/-0.18)",
                    );
                    ui.selectable_value(
                        &mut h.enclosure,
                        EnclosureClass::PartiallyEnclosed,
                        "Partially Enclosed (GCpi = +/-0.55)",
                    );
                });

                num_kpa_field(
                    ui,
                    "Ground Snow Load pg",
                    &mut h.ground_snow_pg_kpa,
                    sys,
                    0.0..=15.0,
                );
                ui.checkbox(&mut h.slippery_roof, "Slippery Roof Surface (Cs reduction)");
                num_kpa_field(
                    ui,
                    "Roof Live Load Lr",
                    &mut h.roof_live_lr_kpa,
                    sys,
                    0.0..=5.0,
                );
                num_kpa_field(
                    ui,
                    "Superimposed Dead Load",
                    &mut h.roof_dead_superimposed_kpa,
                    sys,
                    0.0..=5.0,
                );
                num_kpa_field(
                    ui,
                    "Collateral Load (Ceilings/Pipes)",
                    &mut h.collateral_kpa,
                    sys,
                    0.0..=5.0,
                );

                ui.separator();
                ui.strong("Seismic Equivalent Lateral Force Parameters:");
                num_f64(
                    ui,
                    "Spectral Acceleration Sds (g)",
                    &mut h.seismic_sds,
                    0.0..=3.0,
                    0.05,
                );
                num_f64(
                    ui,
                    "Spectral Acceleration Sd1 (g)",
                    &mut h.seismic_sd1,
                    0.0..=1.5,
                    0.02,
                );
                num_f64(
                    ui,
                    "Response Modification Factor R",
                    &mut h.seismic_r_factor,
                    1.0..=8.0,
                    0.5,
                );
            }

            MbsTab::RigidFrames => {
                ui.heading("Rigid Frames (UI-0005)");
                ui.label("Select Frame Sections from Manufacturer Inventory:");

                ui.horizontal(|ui| {
                    ui.label("Column Shape:");
                    egui::ComboBox::from_id_salt("col_combo")
                        .selected_text(&self.project.column_section_id)
                        .show_ui(ui, |ui| {
                            for s in &self.project.database.built_up_sections {
                                ui.selectable_value(
                                    &mut self.project.column_section_id,
                                    s.id.clone(),
                                    format!(
                                        "{} (d={:.0}->{:.0}mm)",
                                        s.id,
                                        s.depth_start_m * 1000.0,
                                        s.depth_end_m * 1000.0
                                    ),
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Rafter Shape:");
                    egui::ComboBox::from_id_salt("raf_combo")
                        .selected_text(&self.project.rafter_section_id)
                        .show_ui(ui, |ui| {
                            for s in &self.project.database.built_up_sections {
                                ui.selectable_value(
                                    &mut self.project.rafter_section_id,
                                    s.id.clone(),
                                    format!(
                                        "{} (d={:.0}->{:.0}mm)",
                                        s.id,
                                        s.depth_start_m * 1000.0,
                                        s.depth_end_m * 1000.0
                                    ),
                                );
                            }
                        });
                });

                ui.separator();
                ui.strong("Connection Detailing Standards:");
                ui.label("• Eave Knee & Ridge Splice: AISC Design Guide 4 Bolted Moment End-Plate");
                ui.label("• Column Base Plate: AISC Design Guide 1 Base Plate & Anchor Rods");

                ui.horizontal(|ui| {
                    ui.label("Anchor Bolt Supply Scope (DM81):");
                    ui.selectable_value(
                        &mut self.project.anchor_scope,
                        AnchorSupplyScope::ByManufacturer,
                        "By Manufacturer",
                    );
                    ui.selectable_value(
                        &mut self.project.anchor_scope,
                        AnchorSupplyScope::ByOthers,
                        "By Others",
                    );
                });

                ui.separator();
                if ui
                    .button(
                        RichText::new("⚡ Run Discrete Frame Auto-Sizer")
                            .color(Color32::from_rgb(56, 189, 248)),
                    )
                    .clicked()
                {
                    self.auto_size();
                }
            }

            MbsTab::Openings => {
                ui.heading("4-Wall Openings (UI-0007 / UI-0008)");
                ui.label("Standard Wall Openings with Automatic Girt Trimming:");

                ui.label("Quick Preset Add:");
                ui.horizontal(|ui| {
                    if ui.button("+ 12x14 Overhead Door").clicked() {
                        let id = format!("OHD{}", self.project.building.openings.len() + 1);
                        self.project.building.openings.push(WallOpening {
                            id,
                            wall: WallSide::LeftSidewall,
                            offset_m: 6.0,
                            sill_m: 0.0,
                            width_m: 3.658,
                            height_m: 4.267,
                            shop_framed: true,
                        });
                    }
                    if ui.button("+ 3x7 Walk Door").clicked() {
                        let id = format!("MAN{}", self.project.building.openings.len() + 1);
                        self.project.building.openings.push(WallOpening {
                            id,
                            wall: WallSide::LeftSidewall,
                            offset_m: 2.0,
                            sill_m: 0.0,
                            width_m: 0.914,
                            height_m: 2.134,
                            shop_framed: true,
                        });
                    }
                    if ui.button("+ 4x3 Window").clicked() {
                        let id = format!("WIN{}", self.project.building.openings.len() + 1);
                        self.project.building.openings.push(WallOpening {
                            id,
                            wall: WallSide::FrontEndwall,
                            offset_m: 3.0,
                            sill_m: 1.2,
                            width_m: 1.219,
                            height_m: 0.914,
                            shop_framed: false,
                        });
                    }
                });

                ui.separator();
                let b = &mut self.project.building;
                let mut remove_op = None;
                egui::Grid::new("openings_grid")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("ID");
                        ui.strong("Wall Side");
                        ui.strong(format!("Offset ({})", sys.length_unit()));
                        ui.strong(format!("Width ({})", sys.length_unit()));
                        ui.strong(format!("Height ({})", sys.length_unit()));
                        ui.strong("Shop Framed");
                        ui.strong("Action");
                        ui.end_row();

                        for (i, o) in b.openings.iter_mut().enumerate() {
                            ui.text_edit_singleline(&mut o.id);

                            egui::ComboBox::from_id_salt(format!("wall_{i}"))
                                .selected_text(o.wall.as_str())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut o.wall,
                                        WallSide::LeftSidewall,
                                        "Left Sidewall",
                                    );
                                    ui.selectable_value(
                                        &mut o.wall,
                                        WallSide::RightSidewall,
                                        "Right Sidewall",
                                    );
                                    ui.selectable_value(
                                        &mut o.wall,
                                        WallSide::FrontEndwall,
                                        "Front Endwall",
                                    );
                                    ui.selectable_value(
                                        &mut o.wall,
                                        WallSide::BackEndwall,
                                        "Back Endwall",
                                    );
                                });

                            let mut off = Length::from_m(o.offset_m).in_system(sys);
                            if ui.add(egui::DragValue::new(&mut off).speed(0.1)).changed() {
                                o.offset_m = Length::from_system(off, sys).m();
                            }
                            let mut w = Length::from_m(o.width_m).in_system(sys);
                            if ui.add(egui::DragValue::new(&mut w).speed(0.1)).changed() {
                                o.width_m = Length::from_system(w, sys).m();
                            }
                            let mut h = Length::from_m(o.height_m).in_system(sys);
                            if ui.add(egui::DragValue::new(&mut h).speed(0.1)).changed() {
                                o.height_m = Length::from_system(h, sys).m();
                            }

                            ui.checkbox(&mut o.shop_framed, "");

                            if ui.small_button("Delete").clicked() {
                                remove_op = Some(i);
                            }
                            ui.end_row();
                        }
                    });

                if let Some(idx) = remove_op {
                    b.openings.remove(idx);
                }
            }

            MbsTab::Extensions => {
                ui.heading("Extensions, Cranes & Attachments");
                let b = &mut self.project.building;

                ui.collapsing("Lean-To Attachments (F-0004)", |ui| {
                    if ui.button("+ Add Lean-To").clicked() {
                        b.lean_tos.push(LeanToSpec {
                            id: format!("LT{}", b.lean_tos.len() + 1),
                            wall: WallSide::LeftSidewall,
                            span_m: 6.0,
                            eave_height_m: 4.5,
                            slope: 0.08,
                            bay_start: 0,
                            bay_count: b.bay_lengths_m.len().min(4),
                        });
                    }
                    let mut remove_lt = None;
                    for (i, lt) in b.lean_tos.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&lt.id);
                            num_m_field(ui, "Span", &mut lt.span_m, sys, 1.0..=30.0);
                            num_m_field(ui, "Eave", &mut lt.eave_height_m, sys, 1.0..=20.0);
                            if ui.button("Remove").clicked() {
                                remove_lt = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = remove_lt {
                        b.lean_tos.remove(idx);
                    }
                });

                ui.collapsing("Canopies & Overhangs (F-0001 / F-0112)", |ui| {
                    if ui.button("+ Add Canopy").clicked() {
                        b.canopies.push(CanopySpec {
                            id: format!("CAN{}", b.canopies.len() + 1),
                            wall: WallSide::LeftSidewall,
                            projection_m: 2.5,
                            height_m: b.left_eave_m,
                            bay_start: 0,
                            bay_count: b.bay_lengths_m.len(),
                        });
                    }
                    let mut remove_can = None;
                    for (i, can) in b.canopies.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&can.id);
                            num_m_field(ui, "Projection", &mut can.projection_m, sys, 0.5..=10.0);
                            if ui.button("Remove").clicked() {
                                remove_can = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = remove_can {
                        b.canopies.remove(idx);
                    }
                });

                ui.collapsing("Overhead Bridge Cranes (F-0118 / F-0119)", |ui| {
                    if ui.button("+ Add Overhead Crane").clicked() {
                        b.cranes.push(CraneSpec {
                            id: format!("CRN{}", b.cranes.len() + 1),
                            capacity_metric_tons: 10.0,
                            runway_height_m: 5.0,
                            bracket_eccentricity_m: 0.45,
                            wheel_base_m: 3.5,
                            max_wheel_load_kn: 85.0,
                            lateral_thrust_pct: 20.0,
                        });
                    }
                    let mut remove_crn = None;
                    for (i, cr) in b.cranes.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(&cr.id);
                            ui.add(
                                egui::DragValue::new(&mut cr.capacity_metric_tons)
                                    .prefix("Tons: ")
                                    .range(1.0..=100.0),
                            );
                            num_m_field(
                                ui,
                                "Runway Height",
                                &mut cr.runway_height_m,
                                sys,
                                1.0..=30.0,
                            );
                            if ui.button("Remove").clicked() {
                                remove_crn = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = remove_crn {
                        b.cranes.remove(idx);
                    }
                });

                ui.collapsing("Parapet Wall System (F-0075 / F-0114)", |ui| {
                    let mut has_parapet = b.parapet.is_some();
                    if ui
                        .checkbox(&mut has_parapet, "Enable Perimeter Parapets")
                        .changed()
                    {
                        if has_parapet {
                            b.parapet = Some(ParapetSpec {
                                height_m: 1.2,
                                sidewalls: true,
                                endwalls: true,
                            });
                        } else {
                            b.parapet = None;
                        }
                    }
                    if let Some(ref mut p) = b.parapet {
                        num_m_field(
                            ui,
                            "Parapet Height above Eave",
                            &mut p.height_m,
                            sys,
                            0.3..=5.0,
                        );
                        ui.checkbox(&mut p.sidewalls, "Sidewall Parapets");
                        ui.checkbox(&mut p.endwalls, "Endwall Parapets");
                    }
                });
            }

            MbsTab::Inventory => {
                ui.heading("Company Database & Inventory");
                ui.label("Shop Finish / DB_USER Cost Rules:");
                for f in &self.project.database.finishes {
                    ui.selectable_value(
                        &mut self.project.finish_code,
                        f.code.clone(),
                        format!("{} — {}", f.code, f.description),
                    );
                }

                ui.separator();
                ui.strong("Inventory Catalogs:");
                ui.label(format!(
                    "• {} Built-Up 3-Plate Frame Sections",
                    self.project.database.built_up_sections.len()
                ));
                ui.label(format!(
                    "• {} Cold-Formed Secondary Profiles",
                    self.project.database.cold_formed_profiles.len()
                ));
                ui.label(format!(
                    "• {} Structural Bolt Specs",
                    self.project.database.bolts.len()
                ));
                ui.label(format!(
                    "• {} Anchor Rod Specs",
                    self.project.database.anchor_rods.len()
                ));

                ui.separator();
                if ui.button("Run DBCHECK Database Integrity Audit").clicked() {
                    let issues = dbcheck(
                        &self.project.database,
                        self.project.unit_system,
                        self.project.anchor_scope,
                    );
                    if issues.is_empty() {
                        self.message =
                            "DBCHECK Audit: 0 issues found. Database is 100% valid.".into();
                    } else {
                        self.message = format!("DBCHECK Audit: {} issues found.", issues.len());
                    }
                }
            }

            MbsTab::Costing => {
                ui.heading("Costing & Bill of Materials");
                let Some(r) = &self.run else {
                    ui.label("Click Calculate to run the pricing and costing engine.");
                    return;
                };

                egui::Grid::new("cost_grid").striped(true).show(ui, |ui| {
                    ui.strong("Description");
                    ui.strong("Quantity");
                    ui.strong("Unit");
                    ui.strong("Rate");
                    ui.strong("Total");
                    ui.strong("Rule Trace");
                    ui.end_row();

                    for c in &r.costs {
                        ui.label(&c.description);
                        ui.label(format!("{:.2}", c.quantity));
                        ui.label(&c.unit);
                        ui.label(format!("{:.2}", c.rate_minor as f64 / 100.0));
                        ui.label(format!("{:.2}", c.total_minor as f64 / 100.0));
                        ui.label(&c.rule_trace);
                        ui.end_row();
                    }
                });

                ui.separator();
                ui.heading(format!(
                    "Total Project Cost: {} {:.2} (incl. markup)",
                    self.project.base_rates.currency,
                    r.total_minor as f64 / 100.0
                ));
            }

            MbsTab::DrawingSheets => {
                ui.heading("Drawings & Interoperability CAD/BIM");
                ui.label("Fabrication and Erection Drawings:");
                ui.label("• Sheet E1: Foundation Anchor Bolt & Reaction Plan");
                ui.label("• Sheet RF1: Rigid Frame Shop Assembly with Marks & Splices");
                ui.label("• 1:1 CNC Fabrication DXF Profile (shop_RF1.dxf)");
                ui.label("• ISO-10303-21 IFC4 STEP Structural BIM Model (model.ifc)");

                ui.separator();
                if ui
                    .button(
                        RichText::new("📂 Export Complete Deliverables Bundle")
                            .size(16.0)
                            .color(Color32::from_rgb(56, 189, 248)),
                    )
                    .clicked()
                {
                    self.export();
                }
            }

            MbsTab::VerifyReports => {
                ui.heading("Design Verification Reports (MBS Verify)");
                let Some(r) = &self.run else {
                    ui.label("Click Calculate to generate verification reports.");
                    return;
                };

                let verify_text = format_verify_report(&self.project, r);
                ui.horizontal(|ui| {
                    if ui.button("Copy Verification Report").clicked() {
                        ui.ctx().copy_text(verify_text.clone());
                        self.message = "Copied MBS Verify report to clipboard.".into();
                    }
                    if ui.button("Copy PE Certification Letter").clicked() {
                        let cert = generate_pe_certification_letter(
                            &self.project.name,
                            &r.revision,
                            &self.project.building,
                            &self.project.hazard,
                            r.max_aisc_ratio,
                            r.max_drift_ratio,
                            "Structural Engineer of Record",
                            "PE-54912-US",
                        );
                        ui.ctx().copy_text(cert);
                        self.message = "Copied PE Certification Letter to clipboard.".into();
                    }
                });

                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| {
                        ui.monospace(&verify_text);
                    });
            }
        }
    }

    fn draw_viewport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Viewport Mode:").strong());
            ui.selectable_value(
                &mut self.view_mode,
                ViewportMode::Isometric3D,
                "3D BIM Isometric",
            );
            ui.selectable_value(
                &mut self.view_mode,
                ViewportMode::Elevation2D,
                "2D Transverse Elevation",
            );
            ui.selectable_value(
                &mut self.view_mode,
                ViewportMode::RoofPlan2D,
                "2D Roof & Bay Plan",
            );
            ui.separator();
            ui.checkbox(&mut self.show_frames, "Frames");
            ui.checkbox(&mut self.show_purlins, "Purlins/Girts");
            ui.add(egui::Slider::new(&mut self.zoom, 0.4..=3.0).text("Zoom"));
            ui.add(
                egui::Slider::new(&mut self.yaw, -std::f32::consts::PI..=std::f32::consts::PI)
                    .text("Orbit"),
            );
        });

        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), 460.0),
            egui::Sense::click_and_drag(),
        );

        if response.dragged() {
            self.yaw += response.drag_delta().x * 0.01;
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_rgb(15, 23, 42));

        let b = &self.project.building;
        let len = b.total_length_m();
        let center = rect.center();

        match self.view_mode {
            ViewportMode::Isometric3D => {
                let s = (rect.height() / (b.width_m.max(len) as f32 * 1.5)) * self.zoom;
                let cos_y = self.yaw.cos();
                let sin_y = self.yaw.sin();
                let cos_p = 0.5f32;
                let sin_p = 0.866f32;

                let project = |x: f64, y: f64, z: f64| -> Pos2 {
                    let xc = (x - b.width_m / 2.0) as f32;
                    let yc = (y - len / 2.0) as f32;
                    let zc = z as f32;

                    let rx = xc * cos_y - yc * sin_y;
                    let ry = xc * sin_y + yc * cos_y;
                    let px = rx;
                    let py = -zc * cos_p + ry * sin_p;
                    Pos2::new(center.x + px * s, center.y + py * s)
                };

                let cyan = Color32::from_rgb(56, 189, 248);
                let slate = Color32::from_rgb(71, 85, 105);
                let pts = b.frame_points_xz();
                let ys = b.frame_y_positions();

                for (i, &y) in ys.iter().enumerate() {
                    if !self.show_frames && i != 0 && i + 1 != ys.len() {
                        continue;
                    }
                    for j in 0..4 {
                        painter.line_segment(
                            [
                                project(pts[j][0], y, pts[j][1]),
                                project(pts[j + 1][0], y, pts[j + 1][1]),
                            ],
                            Stroke::new(2.0_f32, cyan),
                        );
                    }
                    painter.text(
                        project(0.0, y, 0.0) + Vec2::new(-8.0, 12.0),
                        egui::Align2::CENTER_CENTER,
                        format!("FL{}", i + 1),
                        egui::FontId::proportional(11.0),
                        Color32::LIGHT_GRAY,
                    );
                }

                for pt in &pts {
                    painter.line_segment(
                        [project(pt[0], 0.0, pt[1]), project(pt[0], len, pt[1])],
                        Stroke::new(1.0_f32, slate),
                    );
                }

                if self.show_purlins {
                    let p_lines =
                        (b.left_rafter_length_m() / b.purlin_spacing_m).ceil() as usize + 1;
                    for (p0, p1) in [(pts[1], pts[2]), (pts[3], pts[2])] {
                        for row in 1..p_lines.saturating_sub(1) {
                            let t = row as f64 / (p_lines - 1) as f64;
                            let x = p0[0] * (1.0 - t) + p1[0] * t;
                            let z = p0[1] * (1.0 - t) + p1[1] * t;
                            painter.line_segment(
                                [project(x, 0.0, z), project(x, len, z)],
                                Stroke::new(0.8_f32, Color32::from_rgb(51, 90, 120)),
                            );
                        }
                    }
                }

                for &bb in &b.braced_bays {
                    if bb < b.bay_lengths_m.len() {
                        let y0 = ys[bb];
                        let y1 = ys[bb + 1];
                        for (x, h) in [(0.0, b.left_eave_m), (b.width_m, b.right_eave_m)] {
                            painter.line_segment(
                                [project(x, y0, 0.0), project(x, y1, h)],
                                Stroke::new(1.5_f32, Color32::from_rgb(249, 115, 22)),
                            );
                            painter.line_segment(
                                [project(x, y1, 0.0), project(x, y0, h)],
                                Stroke::new(1.5_f32, Color32::from_rgb(249, 115, 22)),
                            );
                        }
                    }
                }

                for o in &b.openings {
                    let corners = match o.wall {
                        WallSide::LeftSidewall => [
                            project(0.0, o.offset_m, o.sill_m),
                            project(0.0, o.offset_m + o.width_m, o.sill_m),
                            project(0.0, o.offset_m + o.width_m, o.sill_m + o.height_m),
                            project(0.0, o.offset_m, o.sill_m + o.height_m),
                        ],
                        WallSide::RightSidewall => [
                            project(b.width_m, o.offset_m, o.sill_m),
                            project(b.width_m, o.offset_m + o.width_m, o.sill_m),
                            project(b.width_m, o.offset_m + o.width_m, o.sill_m + o.height_m),
                            project(b.width_m, o.offset_m, o.sill_m + o.height_m),
                        ],
                        WallSide::FrontEndwall => [
                            project(o.offset_m, 0.0, o.sill_m),
                            project(o.offset_m + o.width_m, 0.0, o.sill_m),
                            project(o.offset_m + o.width_m, 0.0, o.sill_m + o.height_m),
                            project(o.offset_m, 0.0, o.sill_m + o.height_m),
                        ],
                        WallSide::BackEndwall => [
                            project(o.offset_m, len, o.sill_m),
                            project(o.offset_m + o.width_m, len, o.sill_m),
                            project(o.offset_m + o.width_m, len, o.sill_m + o.height_m),
                            project(o.offset_m, len, o.sill_m + o.height_m),
                        ],
                    };
                    for j in 0..4 {
                        painter.line_segment(
                            [corners[j], corners[(j + 1) % 4]],
                            Stroke::new(2.0_f32, Color32::GOLD),
                        );
                    }
                }
            }

            ViewportMode::Elevation2D => {
                let s = (rect.width() * 0.75 / b.width_m as f32)
                    .min(rect.height() * 0.75 / b.ridge_height_m as f32)
                    * self.zoom;
                let ox = center.x - (b.width_m as f32 * s) / 2.0;
                let oy = center.y + (b.ridge_height_m as f32 * s) / 2.0;

                let to_screen = |x: f64, z: f64| -> Pos2 {
                    Pos2::new(ox + (x as f32) * s, oy - (z as f32) * s)
                };

                let pts = b.frame_points_xz();
                for j in 0..4 {
                    painter.line_segment(
                        [
                            to_screen(pts[j][0], pts[j][1]),
                            to_screen(pts[j + 1][0], pts[j + 1][1]),
                        ],
                        Stroke::new(3.0_f32, Color32::from_rgb(56, 189, 248)),
                    );
                }

                painter.text(
                    to_screen(0.0, b.left_eave_m / 2.0) + Vec2::new(-30.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.1}m", b.left_eave_m),
                    egui::FontId::proportional(12.0),
                    Color32::LIGHT_GRAY,
                );
                painter.text(
                    to_screen(b.width_m, b.right_eave_m / 2.0) + Vec2::new(30.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.1}m", b.right_eave_m),
                    egui::FontId::proportional(12.0),
                    Color32::LIGHT_GRAY,
                );
                painter.text(
                    to_screen(b.ridge_x_m, b.ridge_height_m) + Vec2::new(0.0, -15.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("Peak: {:.2}m", b.ridge_height_m),
                    egui::FontId::proportional(13.0),
                    Color32::WHITE,
                );
                painter.text(
                    to_screen(b.width_m / 2.0, 0.0) + Vec2::new(0.0, 20.0),
                    egui::Align2::CENTER_TOP,
                    format!("Span: {:.2}m", b.width_m),
                    egui::FontId::proportional(14.0),
                    Color32::from_rgb(56, 189, 248),
                );
            }

            ViewportMode::RoofPlan2D => {
                let s = (rect.width() * 0.75 / len as f32)
                    .min(rect.height() * 0.75 / b.width_m as f32)
                    * self.zoom;
                let ox = center.x - (len as f32 * s) / 2.0;
                let oy = center.y - (b.width_m as f32 * s) / 2.0;

                let to_screen = |y: f64, x: f64| -> Pos2 {
                    Pos2::new(ox + (y as f32) * s, oy + (x as f32) * s)
                };

                let ys = b.frame_y_positions();
                for (i, &y) in ys.iter().enumerate() {
                    painter.line_segment(
                        [to_screen(y, 0.0), to_screen(y, b.width_m)],
                        Stroke::new(2.0_f32, Color32::from_rgb(56, 189, 248)),
                    );
                    painter.text(
                        to_screen(y, -0.5),
                        egui::Align2::CENTER_BOTTOM,
                        format!("FL{}", i + 1),
                        egui::FontId::proportional(11.0),
                        Color32::LIGHT_GRAY,
                    );
                }

                for &bb in &b.braced_bays {
                    if bb < b.bay_lengths_m.len() {
                        let y0 = ys[bb];
                        let y1 = ys[bb + 1];
                        painter.line_segment(
                            [to_screen(y0, 0.0), to_screen(y1, b.width_m)],
                            Stroke::new(1.5_f32, Color32::from_rgb(249, 115, 22)),
                        );
                        painter.line_segment(
                            [to_screen(y1, 0.0), to_screen(y0, b.width_m)],
                            Stroke::new(1.5_f32, Color32::from_rgb(249, 115, 22)),
                        );
                    }
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.may_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "OpenMBS v0.2.0 — {} [{}]",
            self.project.name,
            if self.dirty() {
                "Unsaved Changes *"
            } else {
                "Saved"
            }
        )));

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("OPEN MBS")
                        .size(20.0)
                        .strong()
                        .color(Color32::from_rgb(56, 189, 248)),
                );
                ui.separator();
                if ui.button("New Job").clicked() && self.may_discard() {
                    *self = Self::default();
                }
                if ui.button("Open Job").clicked() {
                    self.open();
                }
                if ui.button("Save Job").clicked() {
                    self.save(false);
                }
                if ui.button("Save As...").clicked() {
                    self.save(true);
                }
                ui.separator();
                let next_unit = match self.project.unit_system {
                    UnitSystem::ImperialUS => "Units: US Imperial (ft/psf)",
                    UnitSystem::MetricSI => "Units: SI Metric (m/kPa)",
                };
                if ui
                    .button(RichText::new(next_unit).color(Color32::GOLD))
                    .clicked()
                {
                    let next = match self.project.unit_system {
                        UnitSystem::ImperialUS => UnitSystem::MetricSI,
                        UnitSystem::MetricSI => UnitSystem::ImperialUS,
                    };
                    self.project.unit_system = next;
                    self.project.drawing_control.unit_system = next;
                }
                if ui
                    .button(RichText::new("⚡ Auto-Size").color(Color32::from_rgb(56, 189, 248)))
                    .clicked()
                {
                    self.auto_size();
                }
                if ui
                    .button(
                        RichText::new("▶ Run Design (Calculate)")
                            .color(Color32::LIGHT_GREEN)
                            .strong(),
                    )
                    .clicked()
                {
                    self.process();
                }
                if ui.button("Export Bundle (IFC4/CAD)").clicked() {
                    self.export();
                }
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::top("mbs_ribbon_tabs").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (tab, name) in [
                    (MbsTab::Shape, "1. Shape"),
                    (MbsTab::BaySpacing, "2. Bay Spacing"),
                    (MbsTab::CodesLoads, "3. Codes & Loads"),
                    (MbsTab::RigidFrames, "4. Rigid Frames"),
                    (MbsTab::Openings, "5. 4-Wall Openings"),
                    (MbsTab::Extensions, "6. Extensions & Cranes"),
                    (MbsTab::Inventory, "7. Inventory & DB"),
                    (MbsTab::Costing, "8. Costing"),
                    (MbsTab::DrawingSheets, "9. CAD/BIM Drawings"),
                    (MbsTab::VerifyReports, "10. Verify Reports"),
                ] {
                    ui.selectable_value(
                        &mut self.active_tab,
                        tab,
                        RichText::new(name).size(13.5).strong(),
                    );
                }
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(r) = &self.run {
                    if r.max_aisc_ratio <= 1.0 {
                        ui.colored_label(
                            Color32::LIGHT_GREEN,
                            format!("✔ PASSED (AISC Unity = {:.3})", r.max_aisc_ratio),
                        );
                    } else {
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            format!("✘ OVERSTRESSED (AISC Unity = {:.3})", r.max_aisc_ratio),
                        );
                    }
                } else {
                    ui.colored_label(Color32::GRAY, "⚪ Not Yet Calculated");
                }
                ui.separator();
                ui.label(&self.message);
            });
        });

        let before = self.project.clone();
        egui::SidePanel::left("input_side_panel")
            .resizable(true)
            .default_width(420.0)
            .min_width(340.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.draw_tab_content(ui);
                });
            });

        if before != self.project {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Err(err) = self.project.building.validate() {
                ui.colored_label(Color32::LIGHT_RED, format!("⚠ Validation Warning: {err}"));
            }
            self.draw_viewport(ui);
        });

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save(false);
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1520.0, 980.0])
            .with_min_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OpenMBS Workbench",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<App>::default())
        }),
    )
}
