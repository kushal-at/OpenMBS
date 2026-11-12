//! Independent pure-Rust metal-building engineering, detailing, drafting, and QA engine.
pub mod export;
pub mod model;
pub mod peb;
pub mod pipeline;
pub mod solver;
pub mod storage;

pub use model::*;
pub use openmbs_codes as codes;
pub use openmbs_detailing as detailing;
pub use openmbs_drawings as drawings;
pub use openmbs_geometry as geometry;
pub use openmbs_inventory as inventory;
pub use openmbs_units as units;
pub use peb::*;
pub use pipeline::*;

pub type Result<T> = std::result::Result<T, String>;
pub const ENGINE: &str = "openmbs-0.1.0";
pub const PEB_ENGINE: &str = "openmbs-peb-0.2.0";
