//! Offline project storage. Save atomically; no dependency on GitHub or network.
use crate::model::World;
use serde::{Serialize, Deserialize};
use std::{fs, io, path::{Path, PathBuf}};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub world: World,
}

impl Project {
    pub fn new(world: World) -> Self { Self { schema_version: SCHEMA_VERSION, world } }
}

pub fn load(path: &Path) -> Result<World, String> {
    let bytes=fs::read(path).map_err(|e|format!("Read project: {e}"))?;
    // Verify the version before interpreting the world schema; newer document
    // formats may not be decodable with this version's World type.
    let envelope:serde_json::Value=serde_json::from_slice(&bytes)
        .map_err(|e|format!("Parse project: {e}"))?;
    let version=envelope.get("schema_version").and_then(|v|v.as_u64())
        .ok_or("Project is missing a numeric schema_version")?;
    if version != u64::from(SCHEMA_VERSION) {
        return Err(format!("Project version {} is not supported (expected {})",
            version, SCHEMA_VERSION));
    }
    let project:Project=serde_json::from_value(envelope)
        .map_err(|e|format!("Parse project: {e}"))?;
    if project.world.maps.iter().any(|m| m.cells.len() != (m.width as usize)*(m.height as usize)) {
        return Err("Invalid project: map cell lengths do not match dimensions".into());
    }
    Ok(project.world)
}

pub fn save(path: &Path, world: &World) -> Result<(), String> {
    if let Some(parent)=path.parent() {
        fs::create_dir_all(parent).map_err(|e|format!("Create project directory: {e}"))?;
    }
    let bytes=serde_json::to_vec(&Project::new(world.clone())).map_err(|e|e.to_string())?;
    // Prevent partial overwrites if Android terminates the app during a save.
    let temporary=PathBuf::from(format!("{}.tmp",path.display()));
    {
        let mut file=fs::File::create(&temporary).map_err(|e|format!("Create temporary project: {e}"))?;
        use io::Write;
        file.write_all(&bytes).map_err(|e|format!("Write project: {e}"))?;
        file.sync_all().map_err(|e|format!("Sync project: {e}"))?;
    }
    fs::rename(&temporary,path).map_err(|e|format!("Commit project: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_load_roundtrip() {
        let p=std::env::temp_dir().join(format!("mapstudio-test-{}-{}.json",std::process::id(),6));
        let w=World::sample();
        save(&p,&w).unwrap();
        assert_eq!(load(&p).unwrap(),w);
        let _=fs::remove_file(p);
    }
    #[test]
    fn mismatched_schema_is_not_silently_replaced() {
        let p=std::env::temp_dir().join(format!("mapstudio-test-{}-{}.json",std::process::id(),7));
        fs::write(&p,br#"{"schema_version":999,"world":{}}"#).unwrap();
        assert!(load(&p).unwrap_err().contains("version 999"));
        let _=fs::remove_file(p);
    }
}
