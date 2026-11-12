#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use openmbs_core::{
    codes::{DesignMethod, EnclosureClass, RiskCategory, WindExposure},
    geometry::{EndwallType, WallOpening, WallSide},
    inventory::AnchorSupplyScope,
    peb::{EngineeringProject, EngineeringRun, export_peb_bundle, optimize_peb_frame, process_peb},
    storage::atomic_write,
    units::{Length, Pressure, Speed, UnitSystem},
};
use std::path::PathBuf;

struct App {
    project: EngineeringProject,
    saved_snapshot: EngineeringProject,
    run: Option<EngineeringRun>,
    path: Option<PathBuf>,
    input_tab: usize,
    output_tab: usize,
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
            input_tab: 0,
            output_tab: 0,
            message: "Ready. Edit PEB inputs or click Auto-Size / Calculate.".into(),
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
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(speed)
                .max_decimals(3),
        );
    });
}

impl App {
    fn dirty(&self) -> bool {
        self.project != self.saved_snapshot
    }

    fn may_discard(&self) -> bool {
        !self.dirty()
            || rfd::MessageDialog::new()
                .set_title("Unsaved OpenMBS changes")
                .set_description("Discard unsaved project changes? Choose No to save first.")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                == rfd::MessageDialogResult::Yes
    }

    fn process(&mut self) {
        match process_peb(&self.project) {
            Ok(r) => {
                self.message = format!(
                    "Solved ASCE 7-22 / AISC 360-16: Max Unity {:.3} | Drift/Limit {:.3} | Clashes {}",
                    r.max_aisc_ratio,
                    r.max_drift_ratio,
                    r.detail.clashes.len()
                );
                self.run = Some(r);
            }
            Err(e) => {
                self.run = None;
                self.message = e;
            }
        }
    }

    fn auto_size(&mut self) {
        match optimize_peb_frame(&self.project) {
            Ok((col, raf)) => {
                self.project.column_section_id = col.clone();
                self.project.rafter_section_id = raf;
                self.process();
                self.message = format!("Auto-sized frame section: {col}");
            }
            Err(e) => self.message = e,
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
                    self.message = format!("Saved {}", path.display());
                    self.path = Some(path);
                }
                Err(e) => self.message = e,
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
                    self.message = "Project loaded.".into();
                }
                Err(e) => self.message = e,
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
                Ok(()) => format!("Exported IFC4, SVG/DXF, Verify & BOM to {}", path.display()),
                Err(e) => e,
            };
        }
    }

    fn draw_inputs(&mut self, ui: &mut egui::Ui) {
        let sys = self.project.unit_system;
        match self.input_tab {
            0 => {
                ui.heading("Building Shape & Bays");
                ui.label("Project");
                ui.text_edit_singleline(&mut self.project.name);
                ui.label("Customer");
                ui.text_edit_singleline(&mut self.project.customer);
                ui.separator();

                let b = &mut self.project.building;
                num_m_field(ui, "Width", &mut b.width_m, sys, 2.0..=200.0);
                num_m_field(ui, "Left eave", &mut b.left_eave_m, sys, 2.0..=50.0);
                num_m_field(ui, "Right eave", &mut b.right_eave_m, sys, 2.0..=50.0);
                num_m_field(ui, "Ridge X", &mut b.ridge_x_m, sys, 0.5..=199.5);
                num_m_field(ui, "Ridge height", &mut b.ridge_height_m, sys, 2.0..=60.0);
                ui.checkbox(&mut b.fixed_bases, "Fixed column bases");

                ui.horizontal(|ui| {
                    ui.label("Endwall:");
                    ui.selectable_value(
                        &mut b.endwall_type,
                        EndwallType::PostAndBeam,
                        "Post & Beam",
                    );
                    ui.selectable_value(
                        &mut b.endwall_type,
                        EndwallType::ExpandableRigidFrame,
                        "Rigid Frame",
                    );
                });

                ui.separator();
                ui.label(format!(
                    "Bay Spacing ({} bays, total {:.2} {})",
                    b.bay_lengths_m.len(),
                    Length::from_m(b.total_length_m()).in_system(sys),
                    sys.length_unit()
                ));
                let mut remove_bay = None;
                let can_remove_bay = b.bay_lengths_m.len() > 1;
                for (i, bay_m) in b.bay_lengths_m.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let mut disp = Length::from_m(*bay_m).in_system(sys);
                        ui.label(format!("Bay {} ({})", i + 1, sys.length_unit()));
                        if ui
                            .add(
                                egui::DragValue::new(&mut disp)
                                    .range(1.0..=150.0)
                                    .speed(0.2),
                            )
                            .changed()
                        {
                            *bay_m = Length::from_system(disp, sys).m().clamp(0.5, 50.0);
                        }
                        let mut braced = b.braced_bays.contains(&i);
                        if ui.checkbox(&mut braced, "X-Brace").changed() {
                            if braced {
                                b.braced_bays.push(i);
                            } else {
                                b.braced_bays.retain(|&x| x != i);
                            }
                        }
                        if can_remove_bay && ui.small_button("x").clicked() {
                            remove_bay = Some(i);
                        }
                    });
                }
                if let Some(idx) = remove_bay {
                    b.bay_lengths_m.remove(idx);
                    b.braced_bays.retain(|&x| x < b.bay_lengths_m.len());
                }
                if b.bay_lengths_m.len() < 100 && ui.button("+ Add Bay").clicked() {
                    let last = *b.bay_lengths_m.last().unwrap_or(&6.0);
                    b.bay_lengths_m.push(last);
                }

                ui.separator();
                num_m_field(
                    ui,
                    "Purlin spacing",
                    &mut b.purlin_spacing_m,
                    sys,
                    0.3..=6.0,
                );
                num_m_field(ui, "Girt spacing", &mut b.girt_spacing_m, sys, 0.3..=6.0);
                num_m_field(
                    ui,
                    "Flange brace Lb",
                    &mut b.flange_brace_spacing_m,
                    sys,
                    0.5..=15.0,
                );

                ui.separator();
                ui.label("4-Wall Openings");
                let mut remove_op = None;
                for (i, o) in b.openings.iter_mut().enumerate() {
                    ui.push_id(i, |ui| {
                        ui.collapsing(format!("{} ({})", o.id, o.wall.as_str()), |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for (w, lbl) in [
                                    (WallSide::LeftSidewall, "Left"),
                                    (WallSide::RightSidewall, "Right"),
                                    (WallSide::FrontEndwall, "Front"),
                                    (WallSide::BackEndwall, "Back"),
                                ] {
                                    ui.selectable_value(&mut o.wall, w, lbl);
                                }
                            });
                            num_m_field(ui, "Offset", &mut o.offset_m, sys, 0.0..=900.0);
                            num_m_field(ui, "Sill", &mut o.sill_m, sys, 0.0..=50.0);
                            num_m_field(ui, "Width", &mut o.width_m, sys, 0.3..=50.0);
                            num_m_field(ui, "Height", &mut o.height_m, sys, 0.3..=40.0);
                            ui.checkbox(&mut o.shop_framed, "Shop-framed jambs & header");
                            if ui.button("Remove").clicked() {
                                remove_op = Some(i);
                            }
                        });
                    });
                }
                if let Some(idx) = remove_op {
                    b.openings.remove(idx);
                }
                if ui.button("+ Add Opening").clicked() {
                    let next = b.openings.len() + 1;
                    b.openings.push(WallOpening {
                        id: format!("D{next}"),
                        wall: WallSide::LeftSidewall,
                        offset_m: 1.0,
                        sill_m: 0.0,
                        width_m: 3.0,
                        height_m: 3.5,
                        shop_framed: true,
                    });
                }
            }
            1 => {
                ui.heading("ASCE 7-22 Code Loads");
                let h = &mut self.project.hazard;
                ui.horizontal(|ui| {
                    ui.label("Method:");
                    ui.selectable_value(&mut h.design_method, DesignMethod::LRFD, "LRFD");
                    ui.selectable_value(&mut h.design_method, DesignMethod::ASD, "ASD");
                });
                ui.horizontal(|ui| {
                    ui.label("Risk Cat:");
                    for (rc, lbl) in [
                        (RiskCategory::I, "I"),
                        (RiskCategory::II, "II"),
                        (RiskCategory::III, "III"),
                        (RiskCategory::IV, "IV"),
                    ] {
                        ui.selectable_value(&mut h.risk_category, rc, lbl);
                    }
                });
                ui.checkbox(
                    &mut self.project.second_order_pdelta,
                    "Enable 2nd-order P-Delta analysis",
                );
                ui.checkbox(
                    &mut h.auto_frame_self_weight,
                    "Auto primary steel self-weight",
                );
                ui.separator();
                num_kpa_field(
                    ui,
                    "Roof Dead + Panel",
                    &mut h.roof_dead_superimposed_kpa,
                    sys,
                    0.0..=10.0,
                );
                num_kpa_field(ui, "Collateral", &mut h.collateral_kpa, sys, 0.0..=10.0);
                num_kpa_field(ui, "Roof Live Lr", &mut h.roof_live_lr_kpa, sys, 0.0..=10.0);
                num_kpa_field(
                    ui,
                    "Ground Snow pg",
                    &mut h.ground_snow_pg_kpa,
                    sys,
                    0.0..=20.0,
                );
                ui.checkbox(&mut h.slippery_roof, "Slippery roof (snow slope factor)");
                ui.separator();
                let mut v_disp = Speed::from_mps(h.wind_speed_mps).in_system(sys);
                ui.horizontal(|ui| {
                    ui.label(format!("Wind Speed V ({})", sys.speed_unit()));
                    if ui
                        .add(
                            egui::DragValue::new(&mut v_disp)
                                .range(20.0..=250.0)
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
                    ui.label("Exposure:");
                    ui.selectable_value(&mut h.wind_exposure, WindExposure::B, "B");
                    ui.selectable_value(&mut h.wind_exposure, WindExposure::C, "C");
                    ui.selectable_value(&mut h.wind_exposure, WindExposure::D, "D");
                });
                ui.horizontal(|ui| {
                    ui.label("Enclosure:");
                    ui.selectable_value(&mut h.enclosure, EnclosureClass::Enclosed, "Enclosed");
                    ui.selectable_value(
                        &mut h.enclosure,
                        EnclosureClass::PartiallyEnclosed,
                        "Partial",
                    );
                });
                num_f64(ui, "Seismic Sds (g)", &mut h.seismic_sds, 0.0..=2.5, 0.02);
                num_f64(ui, "Seismic R", &mut h.seismic_r_factor, 1.0..=8.0, 0.25);
            }
            2 => {
                ui.heading("Sections, DB & Finishes");
                ui.label("Column Built-Up Section");
                for s in &self.project.database.built_up_sections {
                    ui.selectable_value(
                        &mut self.project.column_section_id,
                        s.id.clone(),
                        format!(
                            "{} ({:.0}->{:.0}mm)",
                            s.id,
                            s.depth_start_m * 1000.0,
                            s.depth_end_m * 1000.0
                        ),
                    );
                }
                ui.separator();
                ui.label("Rafter Built-Up Section");
                for s in &self.project.database.built_up_sections {
                    ui.selectable_value(
                        &mut self.project.rafter_section_id,
                        s.id.clone(),
                        format!(
                            "{} ({:.0}->{:.0}mm)",
                            s.id,
                            s.depth_start_m * 1000.0,
                            s.depth_end_m * 1000.0
                        ),
                    );
                }
                ui.separator();
                ui.label("Roof Purlin Profile (AISI S100)");
                for p in &self.project.database.cold_formed_profiles {
                    ui.selectable_value(&mut self.project.purlin_profile_id, p.id.clone(), &p.id);
                }
                ui.separator();
                ui.label("Anchor Bolt Supply (DM81)");
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
                ui.selectable_value(
                    &mut self.project.anchor_scope,
                    AnchorSupplyScope::TemplatesOnly,
                    "Templates Only",
                );
                ui.separator();
                ui.label("Shop Finish / DB_USER Rules");
                for f in &self.project.database.finishes {
                    ui.selectable_value(
                        &mut self.project.finish_code,
                        f.code.clone(),
                        format!("{} — {}", f.code, f.description),
                    );
                }
                ui.separator();
                ui.checkbox(
                    &mut self.project.drawing_control.suppress_welds,
                    "Drawing Control: Suppress weld symbols",
                );
            }
            _ => {
                ui.heading("Base Costing Rates");
                ui.label("Integer minor units (cents). DB_USER rules apply on finish/member.");
                let r = &mut self.project.base_rates;
                ui.text_edit_singleline(&mut r.currency);
                for (lbl, val) in [
                    ("Steel / kg (cents)", &mut r.steel_minor_per_kg),
                    ("Fabrication / kg (cents)", &mut r.fabrication_minor_per_kg),
                    ("Finish / kg (cents)", &mut r.finish_minor_per_kg),
                    ("Panels / m2 (cents)", &mut r.panel_minor_per_m2),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(lbl);
                        ui.add(egui::DragValue::new(val).range(0..=1_000_000_000u64));
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Markup (bps)");
                    ui.add(egui::DragValue::new(&mut r.markup_basis_points).range(0..=10000));
                });
            }
        }
    }

    fn draw_viewport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("3D Structural & Detailing Preview");
            ui.checkbox(&mut self.show_frames, "All frames");
            ui.checkbox(&mut self.show_purlins, "Purlins");
            ui.add(egui::Slider::new(&mut self.yaw, 0.1..=1.45).text("Orbit"));
            ui.add(egui::Slider::new(&mut self.zoom, 0.4..=2.0).text("Zoom"));
        });
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height().min(330.0)),
            egui::Sense::drag(),
        );
        if response.dragged() {
            self.yaw = (self.yaw + response.drag_delta().x * 0.004).clamp(0.1, 1.45);
        }
        let rect = response.rect;
        painter.rect_filled(rect, 8.0, Color32::from_rgb(13, 23, 36));
        let b = &self.project.building;
        if let Err(err) = b.validate() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                err,
                egui::FontId::proportional(15.0),
                Color32::LIGHT_RED,
            );
            return;
        }
        let len = b.total_length_m();
        let extent = (b.width_m + len) as f32;
        let scale = (rect.width() / extent)
            .min(rect.height() / (b.ridge_height_m as f32 + len as f32 * 0.4))
            * self.zoom
            * 0.85;
        let project = |x: f64, y: f64, z: f64| {
            let x = x as f32 - b.width_m as f32 / 2.0;
            let y = y as f32 - len as f32 / 2.0;
            Pos2::new(
                rect.center().x + (x * self.yaw.cos() - y * self.yaw.sin()) * scale,
                rect.center().y + (x * self.yaw.sin() + y * self.yaw.cos()) * scale * 0.30
                    - z as f32 * scale
                    + b.left_eave_m.max(b.right_eave_m) as f32 * scale * 0.5,
            )
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
                format!("{}", i + 1),
                egui::FontId::proportional(11.0),
                Color32::LIGHT_GRAY,
            );
        }

        // Eave & ridge longitudinal lines
        for pt in &pts {
            painter.line_segment(
                [project(pt[0], 0.0, pt[1]), project(pt[0], len, pt[1])],
                Stroke::new(1.0_f32, slate),
            );
        }

        // Purlin lines
        if self.show_purlins {
            let p_lines = (b.left_rafter_length_m() / b.purlin_spacing_m).ceil() as usize + 1;
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

        // Diagonal X-bracing (highlight orange/red if clashes exist)
        let has_clash = self
            .run
            .as_ref()
            .is_some_and(|r| !r.detail.clashes.is_empty());
        let brace_col = if has_clash {
            Color32::from_rgb(248, 113, 113)
        } else {
            Color32::from_rgb(249, 115, 22)
        };
        for &bb in &b.braced_bays {
            if bb < b.bay_lengths_m.len() {
                let y0 = ys[bb];
                let y1 = ys[bb + 1];
                for (x, h) in [(0.0, b.left_eave_m), (b.width_m, b.right_eave_m)] {
                    painter.line_segment(
                        [project(x, y0, 0.0), project(x, y1, h)],
                        Stroke::new(1.5_f32, brace_col),
                    );
                    painter.line_segment(
                        [project(x, y1, 0.0), project(x, y0, h)],
                        Stroke::new(1.5_f32, brace_col),
                    );
                }
            }
        }

        // 4-wall openings
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

        let sys = self.project.unit_system;
        painter.text(
            rect.left_top() + Vec2::new(14.0, 12.0),
            egui::Align2::LEFT_TOP,
            format!(
                "{:.1} x {:.1} {} | {} bays | Finish {}",
                Length::from_m(b.width_m).in_system(sys),
                Length::from_m(len).in_system(sys),
                sys.length_unit(),
                b.bays(),
                self.project.finish_code
            ),
            egui::FontId::proportional(13.0),
            Color32::LIGHT_GRAY,
        );
    }

    fn draw_results(&mut self, ui: &mut egui::Ui) {
        let Some(r) = &self.run else {
            ui.add_space(12.0);
            ui.heading("ASCE 7-22 + AISC 360-16 + Tapered PEB Workbench");
            ui.label("Click Calculate (or Auto-Size) to solve 2nd-order tapered frames, check connections & purlins, run DBCHECK/BOMCHECK/Clash QA, and prepare IFC4 + Shop DXF exports.");
            return;
        };

        if !r.is_current(&self.project) {
            ui.colored_label(
                Color32::YELLOW,
                "STALE RESULTS — inputs modified. Click Calculate to refresh.",
            );
        }

        ui.horizontal_wrapped(|ui| {
            for (lbl, val, col) in [
                (
                    "STEEL MASS",
                    format!("{:.0} kg", r.steel_mass_kg()),
                    Color32::WHITE,
                ),
                (
                    "ESTIMATE",
                    format!(
                        "{} {:.2}",
                        self.project.base_rates.currency,
                        r.total_minor as f64 / 100.0
                    ),
                    Color32::WHITE,
                ),
                (
                    "AISC UNITY",
                    format!("{:.3}", r.max_aisc_ratio),
                    if r.max_aisc_ratio <= 1.0 {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::LIGHT_RED
                    },
                ),
                (
                    "DRIFT / H200",
                    format!("{:.3}", r.max_drift_ratio),
                    if r.max_drift_ratio <= 1.0 {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::LIGHT_RED
                    },
                ),
                (
                    "CLASHES",
                    format!("{}", r.detail.clashes.len()),
                    if r.detail.clashes.is_empty() {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::LIGHT_RED
                    },
                ),
            ] {
                ui.group(|ui| {
                    ui.label(RichText::new(lbl).small().color(Color32::GRAY));
                    ui.label(RichText::new(val).size(19.0).color(col));
                });
            }
        });

        ui.horizontal(|ui| {
            for (idx, lbl) in [
                "Verify & Connections",
                "BOM & BOMCHECK",
                "DBCHECK & Clashes",
                "Cost Provenance",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.output_tab, idx, *lbl);
            }
        });
        ui.separator();

        match self.output_tab {
            0 => {
                ui.label(format!(
                    "Purlin {} (AISI S100): Moment ratio {:.3}, Deflection ratio {:.3}",
                    r.purlin_check.profile_id,
                    r.purlin_check.moment_ratio,
                    r.purlin_check.deflection_ratio
                ));
                for ep in &r.detail.end_plates {
                    ui.label(format!(
                        "End-Plate [{}]: {}x {}, tp={:.0}mm | bolt util {:.3}, plate util {:.3}",
                        ep.location,
                        ep.bolt_count,
                        ep.bolt_id,
                        ep.plate_thickness_m * 1000.0,
                        ep.bolt_utilization,
                        ep.plate_utilization
                    ));
                }
                for bp in &r.detail.base_plates {
                    ui.label(format!(
                        "Base Plate [{}]: {:.0}x{:.0}x{:.0}mm, {}x {} ({:?}) | anchor util {:.3}",
                        bp.location,
                        bp.plate_width_m * 1000.0,
                        bp.plate_length_m * 1000.0,
                        bp.plate_thickness_m * 1000.0,
                        bp.anchor_count,
                        bp.anchor_id,
                        bp.anchor_scope,
                        bp.anchor_utilization
                    ));
                }
                ui.separator();
                for f in r.frames.iter().filter(|f| !f.is_serviceability) {
                    ui.collapsing(
                        format!(
                            "{} | {} (P-Delta iters={})",
                            f.frame_label, f.combination, f.solution.second_order_iterations
                        ),
                        |ui| {
                            for c in &f.member_checks {
                                ui.label(format!(
                                    "{} @xi={:.2}: N={:.1} kN, V={:.1} kN, M={:.1} kNm | Unity={:.3} [{}]",
                                    c.member,
                                    c.governing_xi,
                                    c.axial_n / 1000.0,
                                    c.shear_n / 1000.0,
                                    c.moment_nm / 1000.0,
                                    c.aisc.interaction_ratio,
                                    c.aisc.governing_clause
                                ));
                            }
                        },
                    );
                }
            }
            1 => {
                egui::Grid::new("bom_grid").striped(true).show(ui, |ui| {
                    for h in ["Mark", "Category", "Finish", "Qty", "Length m", "Total kg"] {
                        ui.strong(h);
                    }
                    ui.end_row();
                    for b in &r.bom {
                        ui.label(&b.mark);
                        ui.label(&b.category);
                        ui.label(&b.finish_code);
                        ui.label(b.count.to_string());
                        ui.label(format!("{:.3}", b.length_each_m));
                        ui.label(format!("{:.1}", b.total_mass_kg));
                        ui.end_row();
                    }
                });
                ui.separator();
                ui.strong("BOMCHECK Location Reconciliation");
                for bc in &r.detail.bomcheck {
                    ui.label(format!(
                        "[{}] {} | {} | qty={} | {:.1} kg | {}",
                        bc.status,
                        bc.category,
                        bc.mark,
                        bc.count,
                        bc.total_mass_kg,
                        bc.location_tag
                    ));
                }
            }
            2 => {
                ui.strong("3D Geometric Clash Detection");
                if r.detail.clashes.is_empty() {
                    ui.colored_label(Color32::LIGHT_GREEN, "0 clashes detected.");
                } else {
                    for cl in &r.detail.clashes {
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            format!(
                                "{} [{}]: {} <-> {} — {}",
                                cl.clash_id, cl.group_key, cl.part_a, cl.part_b, cl.description
                            ),
                        );
                    }
                }
                ui.separator();
                ui.strong("QA Release Findings");
                for q in &r.qa_findings {
                    ui.label(format!("• {q}"));
                }
            }
            _ => {
                for c in &r.costs {
                    ui.label(format!(
                        "{}: {:.2} {} x {:.2} = {:.2} [{}]",
                        c.description,
                        c.quantity,
                        c.unit,
                        c.rate_minor as f64 / 100.0,
                        c.total_minor as f64 / 100.0,
                        c.rule_trace
                    ));
                }
                ui.label(format!(
                    "Subtotal {:.2} + Markup {:.2} = Total {} {:.2}",
                    r.subtotal_minor as f64 / 100.0,
                    r.markup_minor as f64 / 100.0,
                    self.project.base_rates.currency,
                    r.total_minor as f64 / 100.0
                ));
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
            "OpenMBS PEB Workbench — {}{}",
            self.project.name,
            if self.dirty() { " *" } else { "" }
        )));

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("OPEN MBS")
                        .size(22.0)
                        .strong()
                        .color(Color32::from_rgb(56, 189, 248)),
                );
                if ui.button("New").clicked() && self.may_discard() {
                    *self = Self::default();
                }
                if ui.button("Open").clicked() {
                    self.open();
                }
                if ui.button("Save").clicked() {
                    self.save(false);
                }
                if ui.button("Save As").clicked() {
                    self.save(true);
                }
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo"))
                    .clicked()
                    && let Some(p) = self.undo.pop()
                {
                    self.redo.push(self.project.clone());
                    self.project = p;
                }
                if ui
                    .add_enabled(!self.redo.is_empty(), egui::Button::new("Redo"))
                    .clicked()
                    && let Some(p) = self.redo.pop()
                {
                    self.undo.push(self.project.clone());
                    self.project = p;
                }
                ui.separator();
                let next_unit_lbl = match self.project.unit_system {
                    UnitSystem::ImperialUS => "Units: US (ft/psf)",
                    UnitSystem::MetricSI => "Units: SI (m/kPa)",
                };
                if ui.button(next_unit_lbl).clicked() {
                    let next = match self.project.unit_system {
                        UnitSystem::ImperialUS => UnitSystem::MetricSI,
                        UnitSystem::MetricSI => UnitSystem::ImperialUS,
                    };
                    self.project.unit_system = next;
                    self.project.drawing_control.unit_system = next;
                }
                if ui.button("Auto-Size").clicked() {
                    self.auto_size();
                }
                if ui
                    .button(RichText::new("Calculate").color(Color32::LIGHT_GREEN))
                    .clicked()
                {
                    self.process();
                }
                if ui.button("Export Bundle (IFC/DXF)").clicked() {
                    self.export();
                }
            });
            ui.add_space(5.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    Color32::from_rgb(245, 158, 11),
                    "ENGINEERING APPROVAL REQUIRED BEFORE CONSTRUCTION",
                );
                ui.separator();
                ui.label(&self.message);
            });
        });

        let before = self.project.clone();
        egui::SidePanel::left("inputs")
            .resizable(true)
            .default_width(350.0)
            .min_width(300.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (i, lbl) in ["Shape", "Loads", "Members/DB", "Costs"].iter().enumerate() {
                        ui.selectable_value(&mut self.input_tab, i, *lbl);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| self.draw_inputs(ui));
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
                ui.colored_label(Color32::LIGHT_RED, err);
            }
            self.draw_viewport(ui);
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("results_scroll")
                .show(ui, |ui| self.draw_results(ui));
        });

        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.save(false);
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1480.0, 960.0])
            .with_min_inner_size([1080.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "OpenMBS PEB Workbench",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<App>::default())
        }),
    )
}
