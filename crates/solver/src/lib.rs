//! 2D Euler-Bernoulli finite-element frame solver supporting prismatic and
//! linearly tapered I-sections, end moment releases, and P-Delta geometric stiffness.
//! Canonical SI units: m, N, rad, Pa.

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub x: f64,
    pub y: f64,
    /// Translational X, translational Y, in-plane Z rotation restraints.
    pub restrained: [bool; 3],
}

/// Cross-section stiffness definition along element coordinate xi in [0, 1].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SectionStiffness {
    /// Constant prismatic E, A, I along element length.
    Prismatic { e: f64, area: f64, inertia: f64 },
    /// Linearly tapered doubly-symmetric I-section from node `a` (xi=0) to node `b` (xi=1).
    TaperedI {
        e: f64,
        depth_a: f64,
        depth_b: f64,
        flange_width_a: f64,
        flange_width_b: f64,
        flange_t_a: f64,
        flange_t_b: f64,
        web_t_a: f64,
        web_t_b: f64,
    },
}

impl SectionStiffness {
    pub fn sample(&self, xi: f64) -> (f64, f64, f64) {
        match *self {
            Self::Prismatic { e, area, inertia } => (e, area, inertia),
            Self::TaperedI {
                e,
                depth_a,
                depth_b,
                flange_width_a,
                flange_width_b,
                flange_t_a,
                flange_t_b,
                web_t_a,
                web_t_b,
            } => {
                let d = depth_a * (1.0 - xi) + depth_b * xi;
                let bf = flange_width_a * (1.0 - xi) + flange_width_b * xi;
                let tf = flange_t_a * (1.0 - xi) + flange_t_b * xi;
                let tw = web_t_a * (1.0 - xi) + web_t_b * xi;
                let hw = d - 2.0 * tf;
                let area = 2.0 * bf * tf + hw * tw;
                let inertia = (bf * d.powi(3) - (bf - tw) * hw.powi(3)) / 12.0;
                (e, area, inertia)
            }
        }
    }

    pub fn validate(&self) -> Result<()> {
        match *self {
            Self::Prismatic { e, area, inertia } => {
                if [e, area, inertia]
                    .iter()
                    .any(|v| !v.is_finite() || *v <= 0.0)
                {
                    return Err("Invalid prismatic section stiffness properties".into());
                }
            }
            Self::TaperedI {
                e,
                depth_a,
                depth_b,
                flange_width_a,
                flange_width_b,
                flange_t_a,
                flange_t_b,
                web_t_a,
                web_t_b,
            } => {
                for v in [
                    e,
                    depth_a,
                    depth_b,
                    flange_width_a,
                    flange_width_b,
                    flange_t_a,
                    flange_t_b,
                    web_t_a,
                    web_t_b,
                ] {
                    if !v.is_finite() || v <= 0.0 {
                        return Err("Non-positive or non-finite tapered section dimension".into());
                    }
                }
                if 2.0 * flange_t_a >= depth_a
                    || 2.0 * flange_t_b >= depth_b
                    || web_t_a >= flange_width_a
                    || web_t_b >= flange_width_b
                {
                    return Err("Tapered section thickness exceeds bounding geometry".into());
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameElement {
    pub a: usize,
    pub b: usize,
    pub section: SectionStiffness,
    /// Global distributed line load X (N/m along element length).
    pub qx: f64,
    /// Global distributed line load Y (N/m along element length).
    pub qy: f64,
    /// Release in-plane moment (hinge) at start node `a`.
    pub release_a_mz: bool,
    /// Release in-plane moment (hinge) at end node `b`.
    pub release_b_mz: bool,
}

impl FrameElement {
    pub fn prismatic(
        a: usize,
        b: usize,
        e: f64,
        area: f64,
        inertia: f64,
        qx: f64,
        qy: f64,
    ) -> Self {
        Self {
            a,
            b,
            section: SectionStiffness::Prismatic { e, area, inertia },
            qx,
            qy,
            release_a_mz: false,
            release_b_mz: false,
        }
    }
}

/// Compatibility element type matching OpenMBS v0.1 prismatic solver calls.
#[derive(Clone, Debug)]
pub struct Element {
    pub a: usize,
    pub b: usize,
    pub e: f64,
    pub area: f64,
    pub inertia: f64,
    pub qx: f64,
    pub qy: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Solution {
    pub displacement: Vec<[f64; 3]>,
    pub reaction: Vec<[f64; 3]>,
    pub end_forces: Vec<[f64; 6]>,
    pub residual_relative: f64,
    pub second_order_iterations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StationForce {
    pub xi: f64,
    pub x_local: f64,
    pub axial_n: f64,
    pub shear_n: f64,
    pub moment_nm: f64,
}

type M6 = [[f64; 6]; 6];

/// 5-point Gauss-Legendre points and weights mapped to xi in [0, 1].
const GAUSS5: [(f64, f64); 5] = [
    (0.5 * (1.0 - 0.906179845938664), 0.5 * 0.23692688505618908),
    (0.5 * (1.0 - 0.5384693101056831), 0.5 * 0.47862867049936647),
    (0.5, 0.5 * 0.5688888888888889),
    (0.5 * (1.0 + 0.5384693101056831), 0.5 * 0.47862867049936647),
    (0.5 * (1.0 + 0.906179845938664), 0.5 * 0.23692688505618908),
];

fn rotation_matrix(c: f64, s: f64) -> M6 {
    let mut t = [[0.0; 6]; 6];
    for o in [0, 3] {
        t[o][o] = c;
        t[o][o + 1] = s;
        t[o + 1][o] = -s;
        t[o + 1][o + 1] = c;
        t[o + 2][o + 2] = 1.0;
    }
    t
}

fn local_matrices(nodes: &[Node], elem: &FrameElement) -> (f64, M6, M6, [f64; 6]) {
    let dx = nodes[elem.b].x - nodes[elem.a].x;
    let dy = nodes[elem.b].y - nodes[elem.a].y;
    let l = dx.hypot(dy);
    let c = dx / l;
    let s = dy / l;
    let t = rotation_matrix(c, s);

    let mut k_local = [[0.0; 6]; 6];
    match elem.section {
        SectionStiffness::Prismatic { e, area, inertia } => {
            let a = e * area / l;
            let b = 12.0 * e * inertia / l.powi(3);
            let d = 6.0 * e * inertia / l.powi(2);
            let f = 4.0 * e * inertia / l;
            let g = f / 2.0;
            k_local = [
                [a, 0.0, 0.0, -a, 0.0, 0.0],
                [0.0, b, d, 0.0, -b, d],
                [0.0, d, f, 0.0, -d, g],
                [-a, 0.0, 0.0, a, 0.0, 0.0],
                [0.0, -b, -d, 0.0, b, -d],
                [0.0, d, g, 0.0, -d, f],
            ];
        }
        SectionStiffness::TaperedI { .. } => {
            // Exact Gauss-Legendre quadrature of B(x)^T D(x) B(x) dx over element length.
            for (xi, w) in GAUSS5 {
                let (e, area, inertia) = elem.section.sample(xi);
                let ea_dx = e * area * w * l;
                let ei_dx = e * inertia * w * l;

                // Axial strain-displacement: [-1/L, 0, 0, 1/L, 0, 0]
                let ba = [-1.0 / l, 0.0, 0.0, 1.0 / l, 0.0, 0.0];
                // Curvature-displacement d^2v/dx^2 in terms of xi in [0, 1]:
                // d^2 N_v1 / dx^2 = (12*xi - 6) / L^2
                // d^2 N_th1 / dx^2 = (6*xi - 4) / L
                // d^2 N_v2 / dx^2 = (6 - 12*xi) / L^2
                // d^2 N_th2 / dx^2 = (6*xi - 2) / L
                let bb = [
                    0.0,
                    (12.0 * xi - 6.0) / (l * l),
                    (6.0 * xi - 4.0) / l,
                    0.0,
                    (6.0 - 12.0 * xi) / (l * l),
                    (6.0 * xi - 2.0) / l,
                ];

                for i in 0..6 {
                    for j in 0..6 {
                        k_local[i][j] += ba[i] * ba[j] * ea_dx + bb[i] * bb[j] * ei_dx;
                    }
                }
            }
        }
    }

    let qx = c * elem.qx + s * elem.qy;
    let qy = -s * elem.qx + c * elem.qy;
    let mut f_eq = [
        qx * l / 2.0,
        qy * l / 2.0,
        qy * l * l / 12.0,
        qx * l / 2.0,
        qy * l / 2.0,
        -qy * l * l / 12.0,
    ];

    // Static condensation of released end moments.
    let mut released = vec![];
    if elem.release_a_mz {
        released.push(2);
    }
    if elem.release_b_mz {
        released.push(5);
    }
    if released.len() == 1 {
        let r = released[0];
        let k_rr = k_local[r][r];
        let f_r = f_eq[r];
        for i in 0..6 {
            if i != r {
                f_eq[i] -= k_local[i][r] / k_rr * f_r;
                for j in 0..6 {
                    if j != r {
                        k_local[i][j] -= k_local[i][r] * k_local[r][j] / k_rr;
                    }
                }
            }
        }
        f_eq[r] = 0.0;
        k_local[r] = [0.0; 6];
        for row in &mut k_local {
            row[r] = 0.0;
        }
    } else if released.len() == 2 {
        // Condense both rotational DOFs [2, 5] simultaneously.
        let (a11, a12, a22) = (k_local[2][2], k_local[2][5], k_local[5][5]);
        let det = a11 * a22 - a12 * a12;
        let inv = [[a22 / det, -a12 / det], [-a12 / det, a11 / det]];
        let fr = [f_eq[2], f_eq[5]];
        for i in [0, 1, 3, 4] {
            let kr_i = [k_local[i][2], k_local[i][5]];
            let inv_fr = [
                inv[0][0] * fr[0] + inv[0][1] * fr[1],
                inv[1][0] * fr[0] + inv[1][1] * fr[1],
            ];
            f_eq[i] -= kr_i[0] * inv_fr[0] + kr_i[1] * inv_fr[1];
            for j in [0, 1, 3, 4] {
                let kr_j = [k_local[2][j], k_local[5][j]];
                let inv_krj = [
                    inv[0][0] * kr_j[0] + inv[0][1] * kr_j[1],
                    inv[1][0] * kr_j[0] + inv[1][1] * kr_j[1],
                ];
                k_local[i][j] -= kr_i[0] * inv_krj[0] + kr_i[1] * inv_krj[1];
            }
        }
        for r in [2, 5] {
            f_eq[r] = 0.0;
            k_local[r] = [0.0; 6];
            for row in &mut k_local {
                row[r] = 0.0;
            }
        }
    }

    (l, k_local, t, f_eq)
}

/// Exact local Hermitian geometric stiffness matrix for axial force `p_tension` (N, tension + / compression -).
fn local_geometric_matrix(l: f64, p_tension: f64) -> M6 {
    let f = p_tension / (30.0 * l);
    let l2 = l * l;
    [
        [0.0; 6],
        [0.0, 36.0 * f, 3.0 * l * f, 0.0, -36.0 * f, 3.0 * l * f],
        [0.0, 3.0 * l * f, 4.0 * l2 * f, 0.0, -3.0 * l * f, -l2 * f],
        [0.0; 6],
        [0.0, -36.0 * f, -3.0 * l * f, 0.0, 36.0 * f, -3.0 * l * f],
        [0.0, 3.0 * l * f, -l2 * f, 0.0, -3.0 * l * f, 4.0 * l2 * f],
    ]
}

fn indices(a: usize, b: usize) -> [usize; 6] {
    [a * 3, a * 3 + 1, a * 3 + 2, b * 3, b * 3 + 1, b * 3 + 2]
}

fn validate_inputs(
    nodes: &[Node],
    elements: &[FrameElement],
    point_loads: &[[f64; 3]],
) -> Result<()> {
    if nodes.len() < 2
        || nodes.len() > 400
        || elements.is_empty()
        || point_loads.len() != nodes.len()
    {
        return Err("Invalid analysis dimensions".into());
    }
    if nodes.iter().any(|n| !n.x.is_finite() || !n.y.is_finite())
        || point_loads.iter().flatten().any(|x| !x.is_finite())
    {
        return Err("Non-finite node or load".into());
    }
    for e in elements {
        if e.a >= nodes.len() || e.b >= nodes.len() || e.a == e.b {
            return Err("Invalid analysis element connectivity".into());
        }
        e.section.validate()?;
        if !e.qx.is_finite()
            || !e.qy.is_finite()
            || (nodes[e.b].x - nodes[e.a].x).hypot(nodes[e.b].y - nodes[e.a].y) < 1e-6
        {
            return Err("Invalid analysis element geometry or distributed load".into());
        }
    }
    Ok(())
}

fn solve_system(k: &[Vec<f64>], f: &[f64], nodes: &[Node]) -> Result<(Vec<f64>, Vec<f64>, f64)> {
    let n = nodes.len() * 3;
    let free: Vec<usize> = (0..n).filter(|i| !nodes[i / 3].restrained[i % 3]).collect();
    let m = free.len();
    if m == 0 {
        return Err("Model has no free degrees of freedom".into());
    }
    let scale: Vec<f64> = free.iter().map(|&i| k[i][i].abs().sqrt()).collect();
    if scale.iter().any(|s| !s.is_finite() || *s < 1e-12) {
        return Err("Unconnected or unstable degree of freedom".into());
    }
    let mut a = vec![vec![0.0; m + 1]; m];
    for i in 0..m {
        for j in 0..m {
            a[i][j] = k[free[i]][free[j]] / scale[i] / scale[j];
        }
        a[i][m] = f[free[i]] / scale[i];
    }
    for col in 0..m {
        let pivot = (col..m)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .unwrap();
        if a[pivot][col].abs() < 1e-12 {
            return Err("Unstable or ill-conditioned frame; check restraints".into());
        }
        a.swap(col, pivot);
        let pivot_row = a[col].clone();
        for row in a.iter_mut().skip(col + 1) {
            let ratio = row[col] / pivot_row[col];
            for (value, pivot_value) in row.iter_mut().zip(&pivot_row).skip(col) {
                *value -= ratio * pivot_value;
            }
        }
    }
    let mut x = vec![0.0; m];
    for i in (0..m).rev() {
        x[i] = (a[i][m] - (i + 1..m).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i];
    }
    let mut u = vec![0.0; n];
    for i in 0..m {
        u[free[i]] = x[i] / scale[i];
    }
    let r: Vec<f64> = (0..n)
        .map(|i| k[i].iter().zip(&u).map(|(kv, uv)| kv * uv).sum::<f64>() - f[i])
        .collect();
    let residual = free.iter().map(|&i| r[i].abs()).fold(0.0, f64::max)
        / f.iter().map(|v| v.abs()).fold(1.0, f64::max);
    if u.iter().chain(r.iter()).any(|v| !v.is_finite()) || residual > 1e-7 {
        return Err("Analysis failed equilibrium or finite-result check".into());
    }
    Ok((u, r, residual))
}

/// First-order linear elastic frame solve.
pub fn solve_frames(
    nodes: &[Node],
    elements: &[FrameElement],
    point_loads: &[[f64; 3]],
) -> Result<Solution> {
    solve_with_options(nodes, elements, point_loads, false)
}

/// Iterative second-order (P-Delta / P-delta) elastic frame solve using geometric stiffness.
pub fn solve_frames_pdelta(
    nodes: &[Node],
    elements: &[FrameElement],
    point_loads: &[[f64; 3]],
) -> Result<Solution> {
    solve_with_options(nodes, elements, point_loads, true)
}

pub fn solve_with_options(
    nodes: &[Node],
    elements: &[FrameElement],
    point_loads: &[[f64; 3]],
    second_order: bool,
) -> Result<Solution> {
    validate_inputs(nodes, elements, point_loads)?;
    let n = nodes.len() * 3;
    let mut k_mat = vec![vec![0.0; n]; n];
    let mut f: Vec<f64> = point_loads.iter().flatten().copied().collect();
    let mut cached = Vec::with_capacity(elements.len());

    for e in elements {
        let (l, local, t, load) = local_matrices(nodes, e);
        let ix = indices(e.a, e.b);
        for i in 0..6 {
            for a in 0..6 {
                f[ix[i]] += t[a][i] * load[a];
            }
            for j in 0..6 {
                for a in 0..6 {
                    for b in 0..6 {
                        k_mat[ix[i]][ix[j]] += t[a][i] * local[a][b] * t[b][j];
                    }
                }
            }
        }
        cached.push((l, local, t, load, ix));
    }

    let (mut u, mut r, mut residual) = solve_system(&k_mat, &f, nodes)?;
    let mut iterations = 1;

    if second_order {
        let mut converged = false;
        for iter in 2..=30 {
            let mut k_total = k_mat.clone();
            for (l, local, t, load, ix) in &cached {
                let mut v = [0.0; 6];
                for i in 0..6 {
                    v[i] = (0..6).map(|j| t[i][j] * u[ix[j]]).sum();
                }
                let q0 = (0..6).map(|j| local[0][j] * v[j]).sum::<f64>() - load[0];
                let q3 = (0..6).map(|j| local[3][j] * v[j]).sum::<f64>() - load[3];
                // q[0] is outward along -x_local at node a (so tension at a is -q[0]),
                // q[3] is along +x_local at node b (so tension at b is +q[3]).
                let p_tension = 0.5 * (-q0 + q3);
                let kg = local_geometric_matrix(*l, p_tension);
                for i in 0..6 {
                    for j in 0..6 {
                        for a in 0..6 {
                            for b in 0..6 {
                                k_total[ix[i]][ix[j]] += t[a][i] * kg[a][b] * t[b][j];
                            }
                        }
                    }
                }
            }
            let (u_next, r_next, res_next) = solve_system(&k_total, &f, nodes)?;
            let du = u_next
                .iter()
                .zip(&u)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f64::max);
            let unorm = u_next.iter().map(|v| v.abs()).fold(1.0, f64::max);
            u = u_next;
            r = r_next;
            residual = res_next;
            iterations = iter;
            if du / unorm < 1e-9 {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err(
                "Second-order P-Delta analysis failed to converge (near elastic buckling)".into(),
            );
        }
    }

    let end_forces = cached
        .iter()
        .map(|(_, local, t, load, ix)| {
            let mut v = [0.0; 6];
            for i in 0..6 {
                v[i] = (0..6).map(|j| t[i][j] * u[ix[j]]).sum();
            }
            let mut q = [0.0; 6];
            for i in 0..6 {
                q[i] = (0..6).map(|j| local[i][j] * v[j]).sum::<f64>() - load[i];
            }
            q
        })
        .collect();

    Ok(Solution {
        displacement: u.chunks(3).map(|v| [v[0], v[1], v[2]]).collect(),
        reaction: r.chunks(3).map(|v| [v[0], v[1], v[2]]).collect(),
        end_forces,
        residual_relative: residual,
        second_order_iterations: iterations,
    })
}

/// Compatibility entry point matching OpenMBS v0.1 API.
pub fn solve(nodes: &[Node], elements: &[Element], point_loads: &[[f64; 3]]) -> Result<Solution> {
    let mapped: Vec<FrameElement> = elements
        .iter()
        .map(|e| FrameElement::prismatic(e.a, e.b, e.e, e.area, e.inertia, e.qx, e.qy))
        .collect();
    solve_frames(nodes, &mapped, point_loads)
}

/// Sample internal axial (N), shear (V), and moment (M) along an element in `samples >= 2` points.
pub fn element_station_forces(
    nodes: &[Node],
    elem: &FrameElement,
    end_forces: &[f64; 6],
    samples: usize,
) -> Vec<StationForce> {
    let count = samples.max(2);
    let dx = nodes[elem.b].x - nodes[elem.a].x;
    let dy = nodes[elem.b].y - nodes[elem.a].y;
    let l = dx.hypot(dy);
    let c = dx / l;
    let s = dy / l;
    let qx = c * elem.qx + s * elem.qy;
    let qy = -s * elem.qx + c * elem.qy;

    (0..count)
        .map(|i| {
            let xi = i as f64 / (count - 1) as f64;
            let x = xi * l;
            // Convention: end_forces[0..2] at a are [Fx_a, Fy_a, Mz_a] applied BY node ON member.
            // Tension is positive (+), shear (+y at left face = Fy_a + qy*x), sagging moment (+M = -Mz_a + Fy_a*x + qy*x^2/2).
            let axial_n = -(end_forces[0] + qx * x);
            let shear_n = end_forces[1] + qy * x;
            let moment_nm = -end_forces[2] + end_forces[1] * x + 0.5 * qy * x * x;
            StationForce {
                xi,
                x_local: x,
                axial_n,
                shear_n,
                moment_nm,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol * b.abs().max(1.0), "{a} != {b}");
    }

    #[test]
    fn tapered_cantilever_matches_prismatic_limit_and_exact_integral() {
        // When taper start == end, TaperedI stiffness must match Prismatic exactly.
        let s_taper = SectionStiffness::TaperedI {
            e: 200e9,
            depth_a: 0.6,
            depth_b: 0.6,
            flange_width_a: 0.22,
            flange_width_b: 0.22,
            flange_t_a: 0.012,
            flange_t_b: 0.012,
            web_t_a: 0.008,
            web_t_b: 0.008,
        };
        let (_, area, inertia) = s_taper.sample(0.0);
        let nodes = vec![
            Node {
                x: 0.0,
                y: 0.0,
                restrained: [true, true, true],
            },
            Node {
                x: 6.0,
                y: 0.0,
                restrained: [false, false, false],
            },
        ];
        let p_res = solve_frames(
            &nodes,
            &[FrameElement::prismatic(
                0, 1, 200e9, area, inertia, 0.0, 0.0,
            )],
            &[[0.0; 3], [0.0, -25000.0, 0.0]],
        )
        .unwrap();
        let t_elem = FrameElement {
            a: 0,
            b: 1,
            section: s_taper,
            qx: 0.0,
            qy: 0.0,
            release_a_mz: false,
            release_b_mz: false,
        };
        let t_res = solve_frames(&nodes, &[t_elem], &[[0.0; 3], [0.0, -25000.0, 0.0]]).unwrap();
        close(t_res.displacement[1][1], p_res.displacement[1][1], 1e-12);
        close(t_res.reaction[0][2], p_res.reaction[0][2], 1e-12);
    }

    #[test]
    fn multi_element_tapered_cantilever_converges_to_numerical_integral() {
        // Tapered web depth 0.80 m at fixed root -> 0.40 m at tip over L = 6 m with tip load P = -20 kN.
        // Analytical virtual work tip deflection delta = int_0^L P*x^2 / (E*I(x)) dx (x measured from tip).
        let sec = SectionStiffness::TaperedI {
            e: 200e9,
            depth_a: 0.80,
            depth_b: 0.40,
            flange_width_a: 0.20,
            flange_width_b: 0.20,
            flange_t_a: 0.012,
            flange_t_b: 0.012,
            web_t_a: 0.008,
            web_t_b: 0.008,
        };
        let l = 6.0;
        let p = 20_000.0;
        // High-accuracy Simpson / midpoint reference integral over 1000 slices:
        let mut exact_delta = 0.0;
        let n_ref = 1000;
        let dx = l / n_ref as f64;
        for i in 0..n_ref {
            let x = (i as f64 + 0.5) * dx; // distance from root a
            let xi = x / l;
            let (e, _, inertia) = sec.sample(xi);
            let m_x = p * (l - x);
            exact_delta += (m_x * (l - x)) / (e * inertia) * dx;
        }

        // 8-element tapered FE mesh:
        let n_el = 8;
        let mut nodes = Vec::new();
        let mut elems = Vec::new();
        for i in 0..=n_el {
            nodes.push(Node {
                x: l * i as f64 / n_el as f64,
                y: 0.0,
                restrained: if i == 0 { [true; 3] } else { [false; 3] },
            });
        }
        for i in 0..n_el {
            let xi0 = i as f64 / n_el as f64;
            let xi1 = (i + 1) as f64 / n_el as f64;
            let d0 = 0.80 * (1.0 - xi0) + 0.40 * xi0;
            let d1 = 0.80 * (1.0 - xi1) + 0.40 * xi1;
            elems.push(FrameElement {
                a: i,
                b: i + 1,
                section: SectionStiffness::TaperedI {
                    e: 200e9,
                    depth_a: d0,
                    depth_b: d1,
                    flange_width_a: 0.20,
                    flange_width_b: 0.20,
                    flange_t_a: 0.012,
                    flange_t_b: 0.012,
                    web_t_a: 0.008,
                    web_t_b: 0.008,
                },
                qx: 0.0,
                qy: 0.0,
                release_a_mz: false,
                release_b_mz: false,
            });
        }
        let mut loads = vec![[0.0; 3]; n_el + 1];
        loads[n_el] = [0.0, -p, 0.0];
        let sol = solve_frames(&nodes, &elems, &loads).unwrap();
        close(-sol.displacement[n_el][1], exact_delta, 1e-4);
        close(sol.reaction[0][2], p * l, 1e-10);
    }

    #[test]
    fn moment_release_produces_zero_end_moment() {
        let nodes = vec![
            Node {
                x: 0.0,
                y: 0.0,
                restrained: [true, true, true],
            },
            Node {
                x: 3.0,
                y: 0.0,
                restrained: [false, false, false],
            },
            Node {
                x: 6.0,
                y: 0.0,
                restrained: [true, true, true],
            },
        ];
        // Two-element 6m beam fixed at both ends, moment-released at node 0 -> propped cantilever:
        // R_0y = 3/8 w L, M_0 = 0, R_2y = 5/8 w L, M_2 = -w L^2 / 8
        let mut e0 = FrameElement::prismatic(0, 1, 200e9, 0.01, 8e-5, 0.0, -4000.0);
        e0.release_a_mz = true;
        let e1 = FrameElement::prismatic(1, 2, 200e9, 0.01, 8e-5, 0.0, -4000.0);
        let sol = solve_frames(&nodes, &[e0, e1], &[[0.0; 3]; 3]).unwrap();
        close(sol.end_forces[0][2], 0.0, 1e-8);
        close(sol.reaction[0][2], 0.0, 1e-8);
        close(sol.reaction[0][1], 3.0 / 8.0 * 4000.0 * 6.0, 1e-8);
        close(sol.reaction[2][1], 5.0 / 8.0 * 4000.0 * 6.0, 1e-8);
        close(sol.reaction[2][2], -4000.0 * 36.0 / 8.0, 1e-8);
    }

    #[test]
    fn second_order_pdelta_magnifies_cantilever_sway() {
        // Column height 6 m, P_cr = pi^2 E I / (4 L^2). Apply lateral H = 10 kN and axial compression P = 0.25 P_cr.
        // 4 elements capture continuous P-delta curvature accuracy cleanly.
        let e = 200e9;
        let inertia = 8e-5;
        let l = 6.0;
        let p_cr = std::f64::consts::PI.powi(2) * e * inertia / (4.0 * l * l);
        let p_axial = 0.25 * p_cr;
        let h_lat = 10_000.0;
        let n_el = 4;
        let nodes: Vec<Node> = (0..=n_el)
            .map(|i| Node {
                x: 0.0,
                y: l * i as f64 / n_el as f64,
                restrained: if i == 0 { [true; 3] } else { [false; 3] },
            })
            .collect();
        let elems: Vec<FrameElement> = (0..n_el)
            .map(|i| FrameElement::prismatic(i, i + 1, e, 0.01, inertia, 0.0, 0.0))
            .collect();
        let mut loads = vec![[0.0; 3]; n_el + 1];
        loads[n_el] = [h_lat, -p_axial, 0.0];

        let first = solve_frames(&nodes, &elems, &loads).unwrap();
        let second = solve_frames_pdelta(&nodes, &elems, &loads).unwrap();
        let amp = second.displacement[n_el][0] / first.displacement[n_el][0];
        // Theoretical magnification ~ 1 / (1 - 0.25) = 1.3333; cantilevered tip-loaded tangent factor is slightly higher (~1.36)
        assert!(amp > 1.30 && amp < 1.42, "Unexpected P-delta amp {amp}");
        assert!(second.second_order_iterations >= 2);
    }
}
