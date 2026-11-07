//! Offline project storage. Save atomically; no dependency on GitHub or network.
use crate::model::World;
use serde::{Serialize, Deserialize};
use std::{fs, io, path::{Path, PathBuf}};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_PROJECT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub world: World,
}

impl Project {
    pub fn new(world: World) -> Self { Self { schema_version: SCHEMA_VERSION, world } }
}

pub fn decode(bytes:&[u8]) -> Result<World,String> {
    if bytes.len()>MAX_PROJECT_BYTES {
        return Err("Project exceeds the 20 MiB import safety limit".into());
    }
    let envelope:serde_json::Value=serde_json::from_slice(bytes)
        .map_err(|e|format!("Parse project: {e}"))?;
    let version=envelope.get("schema_version").and_then(|v|v.as_u64())
        .ok_or("Project is missing a numeric schema_version")?;
    if version!=u64::from(SCHEMA_VERSION) {
        return Err(format!("Project version {} is not supported (expected {})",
            version,SCHEMA_VERSION));
    }
    let project:Project=serde_json::from_value(envelope)
        .map_err(|e|format!("Parse project: {e}"))?;
    project.world.check_integrity()?;
    Ok(project.world)
}

pub fn load(path:&Path)->Result<World,String> {
    let metadata=fs::metadata(path).map_err(|e|format!("Read project metadata: {e}"))?;
    if metadata.len()>(MAX_PROJECT_BYTES as u64) {
        return Err("Project exceeds the 20 MiB import safety limit".into());
    }
    let bytes=fs::read(path).map_err(|e|format!("Read project: {e}"))?;
    decode(&bytes)
}

/// Replaces a live project only after its contents are fully validated.
/// The caller must first preserve its previous project as a recovery snapshot.
pub fn replace_validated(path:&Path,bytes:&[u8])->Result<World,String> {
    let new_world=decode(bytes)?;
    save(path,&new_world)?;
    Ok(new_world)
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

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn decode_accepts_exported_project_bytes() {
        let w=World::sample();
        let bytes=serde_json::to_vec(&Project::new(w.clone())).unwrap();
        assert_eq!(decode(&bytes).unwrap(),w);
    }
    #[test]
    fn decode_rejects_oversized_and_corrupt_input_without_crash() {
        let oversized=vec![0u8;MAX_PROJECT_BYTES+1];
        assert!(decode(&oversized).unwrap_err().contains("20 MiB"));
        assert!(decode(b"{broken json").is_err());
    }
    #[test]
    fn invalid_import_cannot_overwrite_a_saved_project() {
        let path=std::env::temp_dir()
            .join(format!("mapstudio-restore-test-{}.json",std::process::id()));
        let world=World::sample();
        save(&path,&world).unwrap();
        assert!(replace_validated(&path,b"not json").is_err());
        assert_eq!(load(&path).unwrap(),world);
        let _=fs::remove_file(&path);
    }
    #[test]
    fn valid_import_replaces_only_after_validation() {
        let path=std::env::temp_dir()
            .join(format!("mapstudio-restore-good-{}.json",std::process::id()));
        save(&path,&World::sample()).unwrap();
        let incoming=World::new();
        let bytes=serde_json::to_vec(&Project::new(incoming.clone())).unwrap();
        assert_eq!(replace_validated(&path,&bytes).unwrap(),incoming);
        assert_eq!(load(&path).unwrap(),incoming);
        let _=fs::remove_file(&path);
    }
}
