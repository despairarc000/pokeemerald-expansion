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

/// Restore a JSON project or a Map Studio game-source ZIP containing
/// mapstudio-project.json. Only the exact project file is read; no ZIP paths
/// are extracted to the filesystem.
pub fn decode_package(bytes:&[u8])->Result<World,String> {
    if bytes.len()>MAX_PROJECT_BYTES {
        return Err("Archive exceeds the 20 MiB import safety limit".into());
    }
    if !bytes.starts_with(b"PK\x03\x04") {
        return decode(bytes);
    }
    let mut zip=zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e|format!("Open Map Studio ZIP: {e}"))?;
    let mut project=zip.by_name("mapstudio-project.json")
        .map_err(|_|"ZIP does not contain mapstudio-project.json")?;
    use std::io::Read;
    let mut payload=Vec::new();
    project.by_ref().take((MAX_PROJECT_BYTES as u64)+1)
        .read_to_end(&mut payload).map_err(|e|format!("Read ZIP project: {e}"))?;
    decode(&payload)
}

/// Export a portable, editable project even when maps have unpainted cells
/// or unfinished events. A source/ROM export has stricter validation.
pub fn export_project_zip(path:&Path,world:&World)->Result<(),String>{
    world.check_integrity()?;
    if let Some(parent)=path.parent(){
        fs::create_dir_all(parent).map_err(|e|format!("Create backup directory: {e}"))?;
    }
    let project=serde_json::to_vec_pretty(&Project::new(world.clone()))
        .map_err(|e|format!("Encode project: {e}"))?;
    if project.len()>MAX_PROJECT_BYTES {
        return Err("Project exceeds the 20 MiB export safety limit".into());
    }
    let temp=PathBuf::from(format!("{}.tmp",path.display()));
    let result=(||->Result<(),String>{
        let file=fs::File::create(&temp).map_err(|e|format!("Create temporary ZIP: {e}"))?;
        let mut zip=zip::ZipWriter::new(file);
        let options=zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        use io::Write;
        zip.start_file("mapstudio-project.json",options)
            .map_err(|e|format!("Create project ZIP entry: {e}"))?;
        zip.write_all(&project).map_err(|e|format!("Write project ZIP: {e}"))?;
        let output=zip.finish().map_err(|e|format!("Complete project ZIP: {e}"))?;
        output.sync_all().map_err(|e|format!("Sync project ZIP: {e}"))?;
        Ok(())
    })();
    if let Err(err)=result{
        let _=fs::remove_file(&temp);
        return Err(err);
    }
    fs::rename(temp,path).map_err(|e|format!("Commit project ZIP: {e}"))?;
    Ok(())
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

#[cfg(test)]
mod package_import_tests {
    use super::*;
    #[test]
    fn zip_round_trip_recovers_exact_editor_project() {
        let mut world=World::new();
        world.add_map("Lake",crate::model::GridPoint::new(0,0),5,5).unwrap();
        for y in 0..5 {
            for x in 0..5 {
                world.paint_metatile(crate::model::GridPoint::new(x,y),0x0170);
            }
        }
        let path=std::env::temp_dir()
            .join(format!("mapstudio-package-import-{}.zip",std::process::id()));
        crate::export::export_zip(&world,&path).unwrap();
        let bytes=std::fs::read(&path).unwrap();
        assert_eq!(decode_package(&bytes).unwrap(),world);
        let _=std::fs::remove_file(path);
    }
    #[test]
    fn malformed_or_unrelated_zips_are_not_projects() {
        assert!(decode_package(b"PK\x03\x04bad").is_err());
        assert!(decode_package(b"[]").is_err());
    }
}

#[cfg(test)]
mod editable_project_package_tests {
    use super::*;
    #[test]
    fn unfinished_world_can_roundtrip_as_portable_package(){
        let mut world=World::new();
        let id=world.add_map("Unfinished",crate::model::GridPoint::new(0,0),7,7).unwrap();
        let p=std::env::temp_dir().join(format!(
            "mapstudio-editable-project-{}-{}.zip",std::process::id(),id));
        export_project_zip(&p,&world).unwrap();
        let bytes=fs::read(&p).unwrap();
        assert_eq!(decode_package(&bytes).unwrap(),world);
        assert!(crate::export::validate(&world).is_err());
        let _=fs::remove_file(p);
    }
}
