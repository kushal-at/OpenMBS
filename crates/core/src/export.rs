use crate::{Project, Result, Run, storage::atomic_write};
use sha2::{Digest, Sha256};
use std::path::Path;
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn csv(s: &str) -> String {
    let safe = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.into()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}
pub fn drawing_svg(p: &Project) -> Result<String> {
    crate::validate(p)?;
    let b = &p.building;
    let scale = (1000.0 / b.width_m).min(270.0 / b.ridge_m());
    let mut s = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 900"><rect width="1200" height="900" fill="#f7f9fc"/><g font-family="sans-serif" fill="#15283f"><text x="60" y="48" font-size="25">OPEN MBS / {}</text><text x="60" y="78" font-size="14">PRELIMINARY GENERAL ARRANGEMENT • metres • NOT FOR CONSTRUCTION</text><text x="60" y="115" font-size="18">TRANSVERSE FRAME</text>"##,
        xml(&p.name)
    );
    let pts = b.frame_points();
    for i in 0..4 {
        s += &format!(
            r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="#126e82" stroke-width="5"/>"##,
            80.0 + pts[i][0] * scale,
            420.0 - pts[i][1] * scale,
            80.0 + pts[i + 1][0] * scale,
            420.0 - pts[i + 1][1] * scale
        );
    }
    s += &format!(
        r##"<text x="80" y="450" font-size="16">Width {:.3} m / Eave {:.3} m / Ridge {:.3} m</text><text x="60" y="490" font-size="18">ROOF PLAN</text>"##,
        b.width_m,
        b.eave_m,
        b.ridge_m()
    );
    let ps = (1000.0 / b.length_m).min(260.0 / b.width_m);
    for i in 0..=b.bays {
        let x = 80.0 + i as f64 * b.bay_m() * ps;
        s += &format!(
            r##"<line x1="{x}" y1="520" x2="{x}" y2="{}" stroke="#126e82" stroke-width="2"/><text x="{x}" y="510" font-size="12">{}</text>"##,
            520.0 + b.width_m * ps,
            i + 1
        );
    }
    for y in [520.0, 520.0 + b.width_m * ps / 2.0, 520.0 + b.width_m * ps] {
        s += &format!(
            r##"<line x1="80" y1="{y}" x2="{}" y2="{y}" stroke="#126e82" stroke-width="2"/>"##,
            80.0 + b.length_m * ps
        );
    }
    s += &format!(
        r##"<text x="80" y="815" font-size="16">{} bays × {:.3} m / Length {:.3} m</text><text x="60" y="855" font-size="12">Revision {} • Centerlines only; connections, bracing, foundations and opening framing excluded.</text></g></svg>"##,
        b.bays,
        b.bay_m(),
        b.length_m,
        &crate::fingerprint(p)?[..16]
    );
    Ok(s)
}
pub fn drawing_dxf(p: &Project) -> Result<String> {
    crate::validate(p)?;
    let mut d="0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n".to_string();
    let mut line = |a: [f64; 2], b: [f64; 2], layer: &str| {
        d += &format!(
            "0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n30\n0\n11\n{}\n21\n{}\n31\n0\n",
            a[0], a[1], b[0], b[1]
        );
    };
    let b = &p.building;
    let points = b.frame_points();
    for i in 0..4 {
        line(points[i], points[i + 1], "FRAME");
    }
    let y = -b.width_m - 10.0;
    for i in 0..=b.bays {
        let x = i as f64 * b.bay_m();
        line([x, y], [x, y + b.width_m], "PLAN");
    }
    for off in [0.0, b.width_m / 2.0, b.width_m] {
        line([0.0, y + off], [b.length_m, y + off], "PLAN");
    }
    d += "0\nTEXT\n8\nNOTES\n10\n0\n20\n-3\n30\n0\n40\n0.4\n1\nPRELIMINARY - NOT FOR CONSTRUCTION - UNITS METRES\n0\nENDSEC\n0\nEOF\n";
    Ok(d)
}
/// Writes a new immutable export directory. Existing directories are rejected.
pub fn bundle(p: &Project, r: &Run, path: &Path) -> Result<()> {
    if !r.is_current(p) {
        return Err("Results are stale. Process the current revision before export.".into());
    }
    if path.exists() {
        return Err(
            "Export directory exists; choose a new folder to preserve the previous run".into(),
        );
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    let dir = temp.path();
    let mut bom = "mark,description,count,length_each_m,total_mass_kg\n".to_string();
    for l in &r.bom {
        bom += &format!(
            "{},{},{},{:.6},{:.6}\n",
            csv(&l.mark),
            csv(&l.description),
            l.count,
            l.length_m,
            l.mass_kg
        );
    }
    let mut shipping = "occurrence,piece_mark,length_m,mass_kg\n".to_string();
    for part in &r.parts {
        shipping += &format!(
            "{},{},{:.6},{:.6}\n",
            csv(&part.occurrence),
            csv(&part.mark),
            part.length_m,
            part.mass_kg
        );
    }
    let mut costs = "description,quantity,unit,rate_minor,total_minor,currency\n".to_string();
    for l in &r.costs {
        costs += &format!(
            "{},{:.3},{},{},{},{}\n",
            csv(&l.description),
            l.quantity,
            l.unit,
            l.rate_minor,
            l.total_minor,
            p.rates.currency
        );
    }
    let mut html = format!(
        "<!doctype html><html lang='en'><meta charset='utf-8'><title>OpenMBS report</title><style>body{{font:16px system-ui;max-width:1000px;margin:40px auto;color:#173045}}table{{border-collapse:collapse;width:100%;margin:20px 0}}td,th{{text-align:left;padding:10px;border-bottom:1px solid #ddd}}.flag{{background:#fff0d4;padding:20px}}img{{width:100%}}@media print{{body{{margin:12mm}}}}</style><h1>OPEN MBS / {}</h1><p>{} / {}</p><p class='flag'><strong>PRELIMINARY — NOT FOR CONSTRUCTION</strong><br>Production issuance blocked. Quantities exclude connection hardware and detailed fabrication.</p><p>Engine {} / Revision {}</p><h2>General arrangement</h2><img src='arrangement.svg' alt='Frame and roof plan'><h2>Analysis</h2><p>First-order elastic 2D prismatic frames. Positive roof pressure acts downwards on horizontal projection. Wall pressure acts in global X on the left wall. Self weight must be included in supplied loads.</p><table><tr><th>Combination</th><th>Tributary m</th><th>Max sampled translation mm</th><th>Elastic yield ratio</th></tr>",
        xml(&p.name),
        xml(&p.customer),
        xml(&p.site),
        r.engine,
        r.revision
    );
    for f in &r.frames {
        html += &format!(
            "<tr><td>{}</td><td>{:.3}</td><td>{:.3}</td><td>{:.3}</td></tr>",
            xml(&f.combination),
            f.tributary_m,
            f.max_translation_m * 1000.0,
            f.checks.iter().map(|c| c.yield_ratio).fold(0.0, f64::max)
        );
    }
    html += "</table><h2>Preliminary quantities</h2><table><tr><th>Mark</th><th>Description</th><th>Count</th><th>Mass kg</th></tr>";
    for l in &r.bom {
        html += &format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}</td></tr>",
            xml(&l.mark),
            xml(&l.description),
            l.count,
            l.mass_kg
        );
    }
    html += &format!(
        "</table><p>Roof {:.2} m² / Net wall {:.2} m² / Panel mass estimate {:.2} kg</p><h2>Cost estimate</h2><p>Rate source: {}</p><table><tr><th>Item</th><th>Quantity</th><th>Rate</th><th>Total</th></tr>",
        r.roof_area_m2,
        r.wall_area_m2,
        (r.roof_area_m2 + r.wall_area_m2) * p.building.panel_kg_per_m2,
        xml(&p.rates.source)
    );
    for l in &r.costs {
        html += &format!(
            "<tr><td>{}</td><td>{:.3} {}</td><td>{:.2}</td><td>{:.2}</td></tr>",
            xml(&l.description),
            l.quantity,
            l.unit,
            l.rate_minor as f64 / 100.0,
            l.total_minor as f64 / 100.0
        );
    }
    html += &format!(
        "</table><p>Subtotal {:.2} / Markup {:.2} / <strong>Total {} {:.2}</strong></p><h2>Release blockers</h2><ul>",
        r.subtotal_minor as f64 / 100.0,
        r.markup_minor as f64 / 100.0,
        p.rates.currency,
        r.total_minor as f64 / 100.0
    );
    for issue in &r.blockers {
        html += &format!("<li>{}</li>", xml(issue));
    }
    html += "</ul></html>";
    let files = vec![
        (
            "project.json",
            serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?,
        ),
        (
            "analysis.json",
            serde_json::to_vec_pretty(r).map_err(|e| e.to_string())?,
        ),
        ("bom.csv", bom.into_bytes()),
        ("shipping.csv", shipping.into_bytes()),
        ("costs.csv", costs.into_bytes()),
        ("arrangement.svg", drawing_svg(p)?.into_bytes()),
        ("arrangement.dxf", drawing_dxf(p)?.into_bytes()),
        ("report.html", html.into_bytes()),
    ];
    let mut hashes = std::collections::BTreeMap::new();
    for (name, bytes) in files {
        hashes.insert(name, format!("{:x}", Sha256::digest(&bytes)));
        atomic_write(&dir.join(name), &bytes)?;
    }
    atomic_write(&dir.join("manifest.json"),&serde_json::to_vec_pretty(&serde_json::json!({"engine":r.engine,"revision":r.revision,"status":"preliminary","production_ready":false,"sha256":hashes})).map_err(|e|e.to_string())?)?;
    std::fs::rename(dir, path).map_err(|e| e.to_string())?;
    Ok(())
}
