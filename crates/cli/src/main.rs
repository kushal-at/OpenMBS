use openmbs_core::{peb::*, storage::ProjectFile, *};
use std::path::Path;

fn execute() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("new") if args.len() == 2 => {
            let path = Path::new(&args[1]);
            if path.exists() {
                return Err("File exists".into());
            }
            let mut f = ProjectFile::default();
            f.commit(&Project::default(), "Initial example")?;
            f.save(path)?;
            println!("Created {}", path.display());
        }
        Some("validate") if args.len() == 2 => {
            let f = ProjectFile::load(Path::new(&args[1]))?;
            println!("Valid project, {} revisions", f.revisions.len());
        }
        Some("run") if args.len() == 3 => {
            let p = ProjectFile::load(Path::new(&args[1]))?.latest()?;
            let r = process(&p)?;
            export::bundle(&p, &r, Path::new(&args[2]))?;
            println!(
                "Preliminary export: {}\nSteel {:.2} kg | {} {:.2} | Elastic yield ratio {:.3}\nProduction blocked: {} findings",
                args[2],
                r.steel_mass_kg(),
                p.rates.currency,
                r.total_minor as f64 / 100.0,
                r.governing_ratio(),
                r.blockers.len()
            );
        }
        Some("demo") if args.len() == 2 => {
            let p = Project::default();
            let r = process(&p)?;
            export::bundle(&p, &r, Path::new(&args[1]))?;
            println!("Demonstration exported to {}", args[1]);
        }
        Some("optimize") if args.len() == 5 => {
            let mut file = ProjectFile::load(Path::new(&args[1]))?;
            let mut p = file.latest()?;
            let catalog: Vec<Section> =
                serde_json::from_slice(&std::fs::read(&args[2]).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let limit = args[3].parse::<f64>().map_err(|e| e.to_string())?;
            if Path::new(&args[4]).exists() {
                return Err("Output project already exists".into());
            }
            p.section = select_section(&p, &catalog, limit)?
                .ok_or("No candidate passes preliminary constraints")?;
            file.commit(&p, "Preliminary section enumeration; not code design")?;
            file.save(Path::new(&args[4]))?;
            println!("Selected {} (preliminary only)", p.section.id);
        }
        Some("peb-new") if args.len() == 2 => {
            let path = Path::new(&args[1]);
            if path.exists() {
                return Err("File exists".into());
            }
            let proj = EngineeringProject::default();
            let bytes = serde_json::to_vec_pretty(&proj).map_err(|e| e.to_string())?;
            std::fs::write(path, bytes).map_err(|e| e.to_string())?;
            println!("Created v2 PEB engineering project: {}", path.display());
        }
        Some("peb-run") if args.len() == 3 => {
            let bytes = std::fs::read(&args[1]).map_err(|e| e.to_string())?;
            let proj: EngineeringProject =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let run = process_peb(&proj)?;
            export_peb_bundle(&proj, &run, Path::new(&args[2]))?;
            println!(
                "PEB Export: {}\nSteel {:.1} kg | {} {:.2} | AISC Unity {:.3} | Drift/Limit {:.3} | Clashes {}\nQA Findings: {}",
                args[2],
                run.steel_mass_kg(),
                proj.base_rates.currency,
                run.total_minor as f64 / 100.0,
                run.max_aisc_ratio,
                run.max_drift_ratio,
                run.detail.clashes.len(),
                run.qa_findings.len()
            );
        }
        Some("peb-demo") if args.len() == 2 => {
            let mut proj = EngineeringProject::default();
            if let Ok((c, r)) = optimize_peb_frame(&proj) {
                proj.column_section_id = c;
                proj.rafter_section_id = r;
            }
            let run = process_peb(&proj)?;
            export_peb_bundle(&proj, &run, Path::new(&args[1]))?;
            println!(
                "Full PEB demonstration exported to {} (IFC4, E1/RF1 SVG+DXF, Verify, BOMCHECK, Clashes)",
                args[1]
            );
        }
        Some("verify") if args.len() == 2 => {
            let bytes = std::fs::read(&args[1]).map_err(|e| e.to_string())?;
            let proj: EngineeringProject =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let run = process_peb(&proj)?;
            println!("{}", format_verify_report(&proj, &run));
        }
        Some("dbcheck") if args.len() == 2 => {
            let bytes = std::fs::read(&args[1]).map_err(|e| e.to_string())?;
            let proj: EngineeringProject =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            let issues = openmbs_core::inventory::dbcheck(
                &proj.database,
                proj.unit_system,
                proj.anchor_scope,
            );
            if issues.is_empty() {
                println!("DBCHECK passed: 0 issues.");
            } else {
                for i in &issues {
                    println!(
                        "[{:?}] {} {}::{}: actual='{}', expected='{}'",
                        i.severity, i.rule_id, i.table, i.field, i.actual, i.expected
                    );
                }
            }
        }
        _ => {
            println!(
                "OpenMBS — independent pure-Rust metal-building system\n\nv1 preliminary commands:\n  openmbs new PROJECT\n  openmbs validate PROJECT\n  openmbs run PROJECT NEW_OUTPUT_FOLDER\n  openmbs demo NEW_OUTPUT_FOLDER\n  openmbs optimize PROJECT CATALOG_JSON MAX_TRANSLATION_M OUTPUT_PROJECT\n\nv2 PEB (ASCE 7-22 / AISC 360-16 / Tapered / Connections / IFC4) commands:\n  openmbs peb-new PROJECT_V2\n  openmbs peb-run PROJECT_V2 NEW_OUTPUT_FOLDER\n  openmbs peb-demo NEW_OUTPUT_FOLDER\n  openmbs verify PROJECT_V2\n  openmbs dbcheck PROJECT_V2\n\nDesktop: cargo run -p openmbs-desktop"
            );
            if !args.is_empty() {
                return Err("Unknown command or invalid arguments".into());
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = execute() {
        eprintln!("OpenMBS: {e}");
        std::process::exit(1);
    }
}
