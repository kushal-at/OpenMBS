//! Explicit engineering units with canonical SI storage and lossless/verified
//! conversions between Metric SI and US Imperial unit systems.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Display and authoring unit convention selected for a project or UI session.
/// Internal structural calculations and persistence remain in canonical SI
/// (m, N, Pa, kg, rad) unless explicitly converted at I/O boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum UnitSystem {
    #[default]
    MetricSI,
    ImperialUS,
}

impl UnitSystem {
    pub fn label(self) -> &'static str {
        match self {
            Self::MetricSI => "SI Metric (m, mm, kN, kPa, kg)",
            Self::ImperialUS => "US Imperial (ft, in, kip, psf, lb)",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::MetricSI => "SI",
            Self::ImperialUS => "US",
        }
    }

    pub fn length_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "m",
            Self::ImperialUS => "ft",
        }
    }

    pub fn small_length_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "mm",
            Self::ImperialUS => "in",
        }
    }

    pub fn force_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "kN",
            Self::ImperialUS => "kip",
        }
    }

    pub fn moment_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "kN*m",
            Self::ImperialUS => "kip*ft",
        }
    }

    pub fn pressure_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "kPa",
            Self::ImperialUS => "psf",
        }
    }

    pub fn stress_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "MPa",
            Self::ImperialUS => "ksi",
        }
    }

    pub fn mass_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "kg",
            Self::ImperialUS => "lb",
        }
    }

    pub fn area_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "m2",
            Self::ImperialUS => "ft2",
        }
    }

    pub fn speed_unit(self) -> &'static str {
        match self {
            Self::MetricSI => "m/s",
            Self::ImperialUS => "mph",
        }
    }
}

/// Exact NIST/international conversion constants.
pub const M_PER_FT: f64 = 0.3048;
pub const M_PER_IN: f64 = 0.0254;
pub const MM_PER_IN: f64 = 25.4;
pub const IN_PER_FT: f64 = 12.0;
pub const KG_PER_LB: f64 = 0.45359237;
pub const G_STD: f64 = 9.80665;
pub const N_PER_LBF: f64 = KG_PER_LB * G_STD; // 4.4482216152605 N
pub const N_PER_KIP: f64 = N_PER_LBF * 1000.0; // 4448.2216152605 N
pub const PA_PER_PSF: f64 = N_PER_LBF / (M_PER_FT * M_PER_FT); // 47.88025898033584 Pa
pub const PA_PER_PSI: f64 = N_PER_LBF / (M_PER_IN * M_PER_IN); // 6894.757293168361 Pa
pub const PA_PER_KSI: f64 = PA_PER_PSI * 1000.0;
pub const NM_PER_KIP_FT: f64 = N_PER_KIP * M_PER_FT;
pub const MPS_PER_MPH: f64 = 0.44704;

/// Length in metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Length(pub f64);

impl Length {
    pub fn from_m(m: f64) -> Self {
        Self(m)
    }
    pub fn from_mm(mm: f64) -> Self {
        Self(mm / 1000.0)
    }
    pub fn from_ft(ft: f64) -> Self {
        Self(ft * M_PER_FT)
    }
    pub fn from_in(inches: f64) -> Self {
        Self(inches * M_PER_IN)
    }
    pub fn from_system(value: f64, sys: UnitSystem) -> Self {
        match sys {
            UnitSystem::MetricSI => Self::from_m(value),
            UnitSystem::ImperialUS => Self::from_ft(value),
        }
    }
    pub fn from_small_system(value: f64, sys: UnitSystem) -> Self {
        match sys {
            UnitSystem::MetricSI => Self::from_mm(value),
            UnitSystem::ImperialUS => Self::from_in(value),
        }
    }
    pub fn m(self) -> f64 {
        self.0
    }
    pub fn mm(self) -> f64 {
        self.0 * 1000.0
    }
    pub fn ft(self) -> f64 {
        self.0 / M_PER_FT
    }
    pub fn inches(self) -> f64 {
        self.0 / M_PER_IN
    }
    pub fn in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.m(),
            UnitSystem::ImperialUS => self.ft(),
        }
    }
    pub fn in_small_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.mm(),
            UnitSystem::ImperialUS => self.inches(),
        }
    }
}

/// Force in Newtons.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Force(pub f64);

impl Force {
    pub fn from_n(n: f64) -> Self {
        Self(n)
    }
    pub fn from_kn(kn: f64) -> Self {
        Self(kn * 1000.0)
    }
    pub fn from_kip(kip: f64) -> Self {
        Self(kip * N_PER_KIP)
    }
    pub fn n(self) -> f64 {
        self.0
    }
    pub fn kn(self) -> f64 {
        self.0 / 1000.0
    }
    pub fn kip(self) -> f64 {
        self.0 / N_PER_KIP
    }
    pub fn in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.kn(),
            UnitSystem::ImperialUS => self.kip(),
        }
    }
    pub fn from_system(value: f64, sys: UnitSystem) -> Self {
        match sys {
            UnitSystem::MetricSI => Self::from_kn(value),
            UnitSystem::ImperialUS => Self::from_kip(value),
        }
    }
}

/// Pressure or distributed area load in Pascals (N/m^2).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Pressure(pub f64);

impl Pressure {
    pub fn from_pa(pa: f64) -> Self {
        Self(pa)
    }
    pub fn from_kpa(kpa: f64) -> Self {
        Self(kpa * 1000.0)
    }
    pub fn from_mpa(mpa: f64) -> Self {
        Self(mpa * 1e6)
    }
    pub fn from_psf(psf: f64) -> Self {
        Self(psf * PA_PER_PSF)
    }
    pub fn from_ksi(ksi: f64) -> Self {
        Self(ksi * PA_PER_KSI)
    }
    pub fn pa(self) -> f64 {
        self.0
    }
    pub fn kpa(self) -> f64 {
        self.0 / 1000.0
    }
    pub fn mpa(self) -> f64 {
        self.0 / 1e6
    }
    pub fn psf(self) -> f64 {
        self.0 / PA_PER_PSF
    }
    pub fn ksi(self) -> f64 {
        self.0 / PA_PER_KSI
    }
    pub fn pressure_in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.kpa(),
            UnitSystem::ImperialUS => self.psf(),
        }
    }
    pub fn from_pressure_system(value: f64, sys: UnitSystem) -> Self {
        match sys {
            UnitSystem::MetricSI => Self::from_kpa(value),
            UnitSystem::ImperialUS => Self::from_psf(value),
        }
    }
    pub fn stress_in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.mpa(),
            UnitSystem::ImperialUS => self.ksi(),
        }
    }
    pub fn from_stress_system(value: f64, sys: UnitSystem) -> Self {
        match sys {
            UnitSystem::MetricSI => Self::from_mpa(value),
            UnitSystem::ImperialUS => Self::from_ksi(value),
        }
    }
}

/// Bending moment in N*m.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Moment(pub f64);

impl Moment {
    pub fn from_nm(nm: f64) -> Self {
        Self(nm)
    }
    pub fn from_knm(knm: f64) -> Self {
        Self(knm * 1000.0)
    }
    pub fn from_kip_ft(kip_ft: f64) -> Self {
        Self(kip_ft * NM_PER_KIP_FT)
    }
    pub fn nm(self) -> f64 {
        self.0
    }
    pub fn knm(self) -> f64 {
        self.0 / 1000.0
    }
    pub fn kip_ft(self) -> f64 {
        self.0 / NM_PER_KIP_FT
    }
    pub fn in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.knm(),
            UnitSystem::ImperialUS => self.kip_ft(),
        }
    }
}

/// Mass in kilograms.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Mass(pub f64);

impl Mass {
    pub fn from_kg(kg: f64) -> Self {
        Self(kg)
    }
    pub fn from_lb(lb: f64) -> Self {
        Self(lb * KG_PER_LB)
    }
    pub fn kg(self) -> f64 {
        self.0
    }
    pub fn lb(self) -> f64 {
        self.0 / KG_PER_LB
    }
    pub fn in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.kg(),
            UnitSystem::ImperialUS => self.lb(),
        }
    }
}

/// Wind/velocity in m/s.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Default)]
pub struct Speed(pub f64);

impl Speed {
    pub fn from_mps(mps: f64) -> Self {
        Self(mps)
    }
    pub fn from_mph(mph: f64) -> Self {
        Self(mph * MPS_PER_MPH)
    }
    pub fn mps(self) -> f64 {
        self.0
    }
    pub fn mph(self) -> f64 {
        self.0 / MPS_PER_MPH
    }
    pub fn in_system(self, sys: UnitSystem) -> f64 {
        match sys {
            UnitSystem::MetricSI => self.mps(),
            UnitSystem::ImperialUS => self.mph(),
        }
    }
}

/// Format an architectural feet-inches string (e.g. `20'-6.5"`).
pub fn format_ft_in(len: Length) -> String {
    let total_in = len.inches();
    let sign = if total_in < 0.0 { "-" } else { "" };
    let abs_in = total_in.abs();
    let mut ft = (abs_in / 12.0).floor() as i64;
    let mut rem_in = abs_in - (ft as f64) * 12.0;
    if (rem_in - 12.0).abs() < 1e-4 {
        ft += 1;
        rem_in = 0.0;
    }
    format!("{sign}{ft}'-{rem_in:.2}\"")
}

/// Format length in active unit system.
pub fn format_length(len: Length, sys: UnitSystem, decimals: usize) -> String {
    let val = len.in_system(sys);
    let unit = sys.length_unit();
    format!("{val:.prec$} {unit}", prec = decimals)
}

impl fmt::Display for UnitSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.short_label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol * b.abs().max(1.0), "{a} != {b}");
    }

    #[test]
    fn length_roundtrip() {
        let l = Length::from_ft(80.0);
        close(l.m(), 24.384, 1e-12);
        close(l.inches(), 960.0, 1e-10);
        close(Length::from_in(1.0).mm(), 25.4, 1e-12);
    }

    #[test]
    fn pressure_and_stress_conversions() {
        // 1 ksi = 6.894757 MPa; 20 psf = 0.957605 kPa
        close(Pressure::from_ksi(50.0).mpa(), 344.737864658418, 1e-9);
        close(Pressure::from_psf(20.0).kpa(), 0.9576051796, 1e-9);
        close(Pressure::from_kpa(1.0).psf(), 20.885434233, 1e-8);
    }

    #[test]
    fn force_and_moment_conversions() {
        close(Force::from_kip(10.0).kn(), 44.4822161526, 1e-9);
        close(Moment::from_kip_ft(100.0).knm(), 135.581794833, 1e-9);
    }

    #[test]
    fn architectural_ft_in_formatting() {
        let l = Length::from_in(246.5); // 20'-6.5"
        assert_eq!(format_ft_in(l), "20'-6.50\"");
    }
}
