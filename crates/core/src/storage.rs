use crate::{ENGINE, Project, Result, fingerprint, validate};
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub hash: String,
    pub engine: String,
    pub note: String,
    pub project: Project,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFile {
    pub format: String,
    pub revisions: Vec<Revision>,
}
impl Default for ProjectFile {
    fn default() -> Self {
        Self {
            format: "openmbs-project-v1".into(),
            revisions: vec![],
        }
    }
}
impl ProjectFile {
    pub fn commit(&mut self, p: &Project, note: &str) -> Result<()> {
        validate(p)?;
        let hash = fingerprint(p)?;
        if self.revisions.last().is_none_or(|r| r.hash != hash) {
            self.revisions.push(Revision {
                hash,
                engine: ENGINE.into(),
                note: note.into(),
                project: p.clone(),
            });
        }
        Ok(())
    }
    pub fn latest(&self) -> Result<Project> {
        self.revisions
            .last()
            .map(|r| r.project.clone())
            .ok_or("Project has no revisions".into())
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.verify()?;
        atomic_write(
            path,
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let file: Self = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
        file.verify()?;
        Ok(file)
    }
    fn verify(&self) -> Result<()> {
        if self.format != "openmbs-project-v1" || self.revisions.is_empty() {
            return Err("Unsupported or empty project file".into());
        }
        for r in &self.revisions {
            if r.hash != fingerprint(&r.project)? {
                return Err("Revision content hash mismatch".into());
            }
            if r.engine != ENGINE {
                return Err(
                    "Project requires a different engine version; explicit migration is required"
                        .into(),
                );
            }
        }
        Ok(())
    }
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
