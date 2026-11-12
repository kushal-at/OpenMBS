use openmbs_core::{solver::*, storage::ProjectFile, *};
fn close(a: f64, b: f64, tol: f64) {
    assert!((a - b).abs() <= tol * b.abs().max(1.0), "{a} != {b}");
}
fn beam(fixed: bool, qy: f64) -> (Vec<Node>, Vec<Element>) {
    (
        vec![
            Node {
                x: 0.0,
                y: 0.0,
                restrained: [true, true, fixed],
            },
            Node {
                x: 6.0,
                y: 0.0,
                restrained: if fixed {
                    [false; 3]
                } else {
                    [false, true, false]
                },
            },
        ],
        vec![Element {
            a: 0,
            b: 1,
            e: 200e9,
            area: 0.01,
            inertia: 8e-5,
            qx: 0.0,
            qy,
        }],
    )
}
#[test]
fn cantilever_tip_load() {
    let (nodes, e) = beam(true, 0.0);
    let r = solve(&nodes, &e, &[[0.0; 3], [0.0, -10000.0, 0.0]]).unwrap();
    close(
        r.displacement[1][1],
        -10000.0 * 6f64.powi(3) / (3.0 * 200e9 * 8e-5),
        1e-10,
    );
    close(r.reaction[0][1], 10000.0, 1e-10);
    close(r.reaction[0][2], 60000.0, 1e-10);
}
#[test]
fn cantilever_uniform_load() {
    let (nodes, e) = beam(true, -2000.0);
    let r = solve(&nodes, &e, &[[0.0; 3]; 2]).unwrap();
    close(
        r.displacement[1][1],
        -2000.0 * 6f64.powi(4) / (8.0 * 200e9 * 8e-5),
        1e-10,
    );
    close(r.reaction[0][2], 36000.0, 1e-10);
}
#[test]
fn simply_supported_uniform_load() {
    let (nodes, e) = beam(false, -2000.0);
    let r = solve(&nodes, &e, &[[0.0; 3]; 2]).unwrap();
    close(r.reaction[0][1], 6000.0, 1e-10);
    close(r.reaction[1][1], 6000.0, 1e-10);
    close(r.end_forces[0][2], 0.0, 1e-8);
    close(r.end_forces[0][5], 0.0, 1e-8);
    close(
        r.displacement[0][2],
        -2000.0 * 6f64.powi(3) / (24.0 * 200e9 * 8e-5),
        1e-10,
    );
}
#[test]
fn axial_bar() {
    let (nodes, e) = beam(true, 0.0);
    let r = solve(&nodes, &e, &[[0.0; 3], [20000.0, 0.0, 0.0]]).unwrap();
    close(r.displacement[1][0], 20000.0 * 6.0 / (200e9 * 0.01), 1e-10);
    close(r.reaction[0][0], -20000.0, 1e-10);
}
#[test]
fn rotated_cantilever() {
    let (mut nodes, e) = beam(true, 0.0);
    nodes[1].x = 0.0;
    nodes[1].y = 6.0;
    let r = solve(&nodes, &e, &[[0.0; 3], [10000.0, 0.0, 0.0]]).unwrap();
    close(
        r.displacement[1][0],
        10000.0 * 6f64.powi(3) / (3.0 * 200e9 * 8e-5),
        1e-10,
    );
    close(r.reaction[0][2], 60000.0, 1e-10);
}
#[test]
fn mechanism_is_rejected() {
    let (mut nodes, e) = beam(true, 0.0);
    nodes[0].restrained = [false; 3];
    assert!(solve(&nodes, &e, &[[0.0; 3]; 2]).is_err());
}
#[test]
fn invalid_solver_data_is_rejected() {
    let (nodes, mut e) = beam(true, 0.0);
    e[0].inertia = f64::NAN;
    assert!(solve(&nodes, &e, &[[0.0; 3]; 2]).is_err());
}
#[test]
fn frame_global_force_and_moment_equilibrium() {
    let p = Project::default();
    let r = process(&p).unwrap();
    for f in &r.frames {
        let combo = p
            .combinations
            .iter()
            .find(|c| c.id == f.combination)
            .unwrap();
        let roof = p
            .loads
            .iter()
            .zip(&combo.factors)
            .map(|(l, c)| l.roof_down_kpa * c * 1000.0 * f.tributary_m)
            .sum::<f64>();
        let wall = p
            .loads
            .iter()
            .zip(&combo.factors)
            .map(|(l, c)| l.wall_right_kpa * c * 1000.0 * f.tributary_m)
            .sum::<f64>();
        let l = f.solution.reaction[0];
        let rr = f.solution.reaction.last().unwrap();
        close(l[1] + rr[1], roof * p.building.width_m, 1e-8);
        close(l[0] + rr[0], -wall * p.building.eave_m, 1e-8);
        close(
            l[2] + rr[2] + rr[1] * p.building.width_m,
            roof * p.building.width_m.powi(2) / 2.0 + wall * p.building.eave_m.powi(2) / 2.0,
            1e-8,
        );
    }
}
#[test]
fn pinned_bases_have_zero_moment() {
    let mut p = Project::default();
    p.building.fixed_bases = false;
    let r = process(&p).unwrap();
    for f in &r.frames {
        close(f.solution.reaction[0][2], 0.0, 1e-6);
        close(f.solution.reaction.last().unwrap()[2], 0.0, 1e-6);
    }
}
#[test]
fn linear_load_scaling() {
    let p = Project::default();
    let a = process(&p).unwrap();
    let mut p2 = p.clone();
    for c in &mut p2.combinations {
        for f in &mut c.factors {
            *f *= 2.0;
        }
    }
    let b = process(&p2).unwrap();
    close(b.governing_ratio(), 2.0 * a.governing_ratio(), 1e-8);
    close(
        b.frames[0].max_translation_m,
        2.0 * a.frames[0].max_translation_m,
        1e-8,
    );
}
#[test]
fn invalid_project_inputs() {
    let mut p = Project::default();
    p.building.width_m = f64::NAN;
    assert!(validate(&p).is_err());
    p = Project::default();
    p.combinations[0].factors.pop();
    assert!(validate(&p).is_err());
    p = Project::default();
    p.section.flange_m = 0.2;
    p.section.depth_m = 0.2;
    assert!(validate(&p).is_err());
}
#[test]
fn opening_overlap_and_bounds() {
    let mut p = Project::default();
    let o = Opening {
        id: "O1".into(),
        wall: "left".into(),
        offset_m: 1.0,
        sill_m: 0.0,
        width_m: 3.0,
        height_m: 3.0,
    };
    p.building.openings.push(o.clone());
    validate(&p).unwrap();
    let mut o2 = o;
    o2.id = "O2".into();
    p.building.openings.push(o2);
    assert!(validate(&p).is_err());
    p.building.openings.pop();
    p.building.openings[0].offset_m = 35.0;
    assert!(validate(&p).is_err());
}
#[test]
fn quantities_reconcile_and_occurrences_are_unique() {
    let p = Project::default();
    let r = process(&p).unwrap();
    close(
        r.bom.iter().map(|l| l.mass_kg).sum(),
        r.steel_mass_kg(),
        1e-12,
    );
    assert_eq!(r.parts.len(), r.bom.iter().map(|l| l.count).sum::<usize>());
    let ids: std::collections::HashSet<_> = r.parts.iter().map(|p| &p.occurrence).collect();
    assert_eq!(ids.len(), r.parts.len());
    assert_eq!(r.bom.iter().find(|l| l.mark == "C1").unwrap().count, 14);
    close(r.roof_area_m2, 2.0 * 12f64.hypot(1.2) * 36.0, 1e-12);
}
#[test]
fn costs_use_exact_minor_unit_rounding() {
    let mut p = Project::default();
    p.rates.markup_basis_points = 1500;
    let r = process(&p).unwrap();
    for line in &r.costs {
        let milli = (line.quantity * 1000.0).round() as u128;
        assert_eq!(
            line.total_minor as u128,
            (milli * line.rate_minor as u128 + 500) / 1000
        );
    }
    assert_eq!(r.total_minor, r.subtotal_minor + r.markup_minor);
    assert_eq!(
        r.markup_minor,
        ((r.subtotal_minor as u128 * 1500 + 5000) / 10000) as u64
    );
}
#[test]
fn changed_inputs_invalidate_results() {
    let mut p = Project::default();
    let r = process(&p).unwrap();
    assert!(r.is_current(&p));
    p.rates.steel_minor_per_kg += 1;
    assert!(!r.is_current(&p));
    assert!(!r.production_ready());
}
#[test]
fn persistence_roundtrip_restore_and_tamper_detection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.json");
    let mut f = ProjectFile::default();
    let p = Project::default();
    f.commit(&p, "initial").unwrap();
    f.save(&path).unwrap();
    let mut second = p.clone();
    second.building.width_m = 30.0;
    f.commit(&second, "wider").unwrap();
    f.save(&path).unwrap();
    let read = ProjectFile::load(&path).unwrap();
    assert_eq!(read.revisions.len(), 2);
    assert_eq!(read.latest().unwrap(), second);
    assert_eq!(read.revisions[0].project, p);
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["revisions"][0]["project"]["name"] = "tampered".into();
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert!(ProjectFile::load(&path).is_err());
}
#[test]
fn export_manifest_and_stale_rejection() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bundle");
    let mut p = Project {
        name: "<script>bad</script>".into(),
        ..Project::default()
    };
    let r = process(&p).unwrap();
    export::bundle(&p, &r, &path).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path.join("manifest.json")).unwrap()).unwrap();
    for (name, hash) in manifest["sha256"].as_object().unwrap() {
        assert_eq!(
            hash.as_str().unwrap(),
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(path.join(name)).unwrap())
            )
        );
    }
    let html = std::fs::read_to_string(path.join("report.html")).unwrap();
    assert!(!html.contains("<script>"));
    assert!(export::bundle(&p, &r, &path).is_err());
    p.building.length_m += 1.0;
    assert!(export::bundle(&p, &r, &dir.path().join("stale")).is_err());
}
#[test]
fn optimizer_no_feasible_candidate() {
    let p = Project::default();
    assert!(
        select_section(&p, std::slice::from_ref(&p.section), 1e-12)
            .unwrap()
            .is_none()
    );
    assert!(select_section(&p, &[], 0.1).unwrap().is_none());
}
#[test]
fn one_bay_has_only_end_frame_tributary() {
    let mut p = Project::default();
    p.building.bays = 1;
    let r = process(&p).unwrap();
    assert_eq!(r.frames.len(), p.combinations.len());
    for f in r.frames {
        close(f.tributary_m, p.building.length_m / 2.0, 1e-12);
    }
}
