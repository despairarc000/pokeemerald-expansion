//! Isolated Map Studio workspaces. Editor/ is authoritative; Compiled/ is
//! output-only. No game-source registration or ROM data is modified here.
//! The previous single autosave is copied on first run, never moved/deleted.
use crate::{model::World, storage};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::{Path,PathBuf}};
use uuid::Uuid;

pub const MANIFEST_VERSION:u32=1;

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(rename_all="snake_case")]
pub enum GameTarget { EmeraldExpansion, FireRed }

impl GameTarget {
    pub const fn label(self)->&'static str {
        match self { Self::EmeraldExpansion=>"Emerald Expansion",Self::FireRed=>"FireRed" }
    }
    pub const fn repository(self)->&'static str {
        match self {
            Self::EmeraldExpansion=>"despairarc000/pokeemerald-expansion",
            Self::FireRed=>"pret/pokefirered",
        }
    }
    /// The current generator uses Emerald headers, layout symbols and groups.
    /// FRLG artwork in an Emerald fork does NOT make it a FireRed compiler.
    pub const fn can_export(self)->bool { matches!(self,Self::EmeraldExpansion) }
}

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub manifest_version:u32,
    pub project_id:String,
    pub name:String,
    pub target:GameTarget,
    pub target_repository:String,
    /// Optional pinned source commit; empty means not yet locked to a build.
    #[serde(default)]
    pub target_commit:Option<String>,
}

impl ProjectManifest {
    pub fn validate(&self)->Result<(),String>{
        if self.manifest_version!=MANIFEST_VERSION {return Err("Unsupported project manifest version".into());}
        if Uuid::parse_str(&self.project_id).map(|u|u.to_string()).ok()
            !=Some(self.project_id.clone()){
            return Err("Invalid canonical project UUID".into());
        }
        if self.name.trim()!=self.name || self.name.is_empty() || self.name.len()>80
            || self.name.chars().any(|c|c.is_control()){
            return Err("Project name must be 1–80 characters without controls".into());
        }
        let fragments:Vec<_>=self.target_repository.split('/').collect();
        if fragments.len()!=2 || fragments.iter().any(|s|s.is_empty() || s.len()>100
            || *s=="." || *s==".." || !s.chars().all(|c|c.is_ascii_alphanumeric()
                || matches!(c,'-'|'_'|'.'))) {
            return Err("Invalid target repository owner/name".into());
        }
        if let Some(commit)=&self.target_commit {
            if commit.len()!=40 || !commit.bytes().all(|b|b.is_ascii_hexdigit()){
                return Err("Target commit must be a full 40-character SHA".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone,Debug)]
pub struct Workspace { pub root:PathBuf }

fn atomic_write(path:&Path,bytes:&[u8])->Result<(),String>{
    if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let temp=path.with_extension("pending");
    let result=(||->Result<(),String>{
        let mut f=fs::File::create(&temp).map_err(|e|e.to_string())?;
        f.write_all(bytes).map_err(|e|e.to_string())?;
        f.sync_all().map_err(|e|e.to_string())?;
        fs::rename(&temp,path).map_err(|e|e.to_string())
    })();
    if result.is_err(){let _=fs::remove_file(temp);}
    result
}
fn checked_id(id:&str)->Result<(),String>{
    let parsed=Uuid::parse_str(id).map_err(|_|"Invalid project ID")?;
    if parsed.to_string()!=id{return Err("Project ID must be a canonical UUID".into());}
    Ok(())
}

impl Workspace {
    pub fn for_legacy_save(path:&Path)->Self {
        let root=path.parent().filter(|p|!p.as_os_str().is_empty())
            .unwrap_or_else(||Path::new(".")).to_path_buf();
        Self{root}
    }
    pub fn editor_root(&self)->PathBuf{self.root.join("Editor")}
    pub fn projects_root(&self)->PathBuf{self.editor_root().join("projects")}
    pub fn compiled_root(&self)->PathBuf{self.root.join("Compiled").join("projects")}
    pub fn project_dir(&self,id:&str)->Result<PathBuf,String>{
        checked_id(id)?;
        Ok(self.projects_root().join(id))
    }
    pub fn project_path(&self,id:&str)->Result<PathBuf,String>{
        Ok(self.project_dir(id)?.join("project.json"))
    }
    pub fn manifest_path(&self,id:&str)->Result<PathBuf,String>{
        Ok(self.project_dir(id)?.join("manifest.json"))
    }
    fn active_path(&self)->PathBuf{self.editor_root().join("active-project.txt")}
    pub fn list(&self)->Result<Vec<ProjectManifest>,String>{
        let root=self.projects_root();
        if !root.exists(){return Ok(Vec::new());}
        let mut list=Vec::new();
        for item in fs::read_dir(&root).map_err(|e|e.to_string())?{
            let item=item.map_err(|e|e.to_string())?;
            if !item.file_type().map_err(|e|e.to_string())?.is_dir(){continue;}
            let id=item.file_name().to_string_lossy().to_string();
            // Unrelated or temporary folders are not workspace projects.
            if checked_id(&id).is_err(){continue;}
            list.push(self.load_manifest(&id)?);
        }
        list.sort_by(|a,b|a.name.cmp(&b.name).then(a.project_id.cmp(&b.project_id)));
        Ok(list)
    }
    pub fn load_manifest(&self,id:&str)->Result<ProjectManifest,String>{
        let path=self.manifest_path(id)?;
        let bytes=fs::read(&path).map_err(|e|format!("Read manifest for {id}: {e}"))?;
        if bytes.len()>8192{return Err("Project manifest is too large".into());}
        let manifest:ProjectManifest=serde_json::from_slice(&bytes)
            .map_err(|e|format!("Project {id} manifest is invalid: {e}"))?;
        manifest.validate()?;
        if manifest.project_id!=id{return Err("Project folder and manifest IDs differ".into());}
        Ok(manifest)
    }
    pub fn load_world(&self,id:&str)->Result<World,String>{
        self.load_manifest(id)?;
        storage::load(&self.project_path(id)?)
    }
    pub fn create(&self,name:&str,target:GameTarget,world:&World)
        ->Result<ProjectManifest,String>{
        world.check_integrity()?;
        let manifest=ProjectManifest{
            manifest_version:MANIFEST_VERSION,
            project_id:Uuid::new_v4().to_string(),
            name:name.trim().into(),
            target,target_repository:target.repository().into(),
            target_commit:None,
        };
        manifest.validate()?;
        let dir=self.project_dir(&manifest.project_id)?;
        fs::create_dir_all(self.projects_root()).map_err(|e|e.to_string())?;
        fs::create_dir(&dir).map_err(|e|e.to_string())?;
        let result=(||->Result<(),String>{
            storage::save(&self.project_path(&manifest.project_id)?,world)?;
            let data=serde_json::to_vec_pretty(&manifest).map_err(|e|e.to_string())?;
            atomic_write(&self.manifest_path(&manifest.project_id)?,&data)
        })();
        if let Err(err)=result {let _=fs::remove_dir_all(&dir);return Err(err);}
        Ok(manifest)
    }
    /// Clone an existing remote project without assigning a different UUID.
    /// It is create-only: a GitHub clone must never overwrite a local copy.
    pub fn import_existing(&self,manifest:&ProjectManifest,world:&World)
        ->Result<(),String>{
        manifest.validate()?;
        world.check_integrity()?;
        let dir=self.project_dir(&manifest.project_id)?;
        fs::create_dir_all(self.projects_root()).map_err(|e|e.to_string())?;
        fs::create_dir(&dir).map_err(|e|format!(
            "Project already exists locally or cannot be created: {e}"))?;
        let result=(||->Result<(),String>{
            storage::save(&self.project_path(&manifest.project_id)?,world)?;
            let data=serde_json::to_vec_pretty(manifest).map_err(|e|e.to_string())?;
            atomic_write(&self.manifest_path(&manifest.project_id)?,&data)
        })();
        if let Err(err)=result{
            let _=fs::remove_dir_all(&dir);
            return Err(err);
        }
        Ok(())
    }

    pub fn activate(&self,id:&str)->Result<(),String>{
        self.load_world(id)?;
        atomic_write(&self.active_path(),format!("{id}\n").as_bytes())
    }
    pub fn active_id(&self)->Result<Option<String>,String>{
        let path=self.active_path();
        if !path.exists(){return Ok(None);}
        let bytes=fs::read(path).map_err(|e|e.to_string())?;
        if bytes.len()>64{return Err("Active project selector is invalid".into());}
        let id=String::from_utf8(bytes).map_err(|_|"Invalid active project encoding")?;
        let id=id.trim();
        checked_id(id)?;
        Ok(Some(id.into()))
    }
    pub fn rename(&self,id:&str,name:&str)->Result<ProjectManifest,String>{
        let mut manifest=self.load_manifest(id)?;
        manifest.name=name.trim().into();
        manifest.validate()?;
        let data=serde_json::to_vec_pretty(&manifest).map_err(|e|e.to_string())?;
        atomic_write(&self.manifest_path(id)?,&data)?;
        Ok(manifest)
    }
    /// Establishes per-project storage on first start. Legacy JSON remains
    /// untouched, so rollback to the older APK cannot erase existing work.
    pub fn open(legacy_save:&Path)->Result<(Self,ProjectManifest,World),String>{
        let store=Self::for_legacy_save(legacy_save);
        let projects=store.list()?;
        let active=store.active_id()?;
        let id=if let Some(id)=active {
            if !projects.iter().any(|m|m.project_id==id){
                return Err("Active project does not exist; project data was not overwritten".into());
            }
            id
        } else if let Some(manifest)=projects.first(){
            manifest.project_id.clone()
        } else {
            let original=if legacy_save.exists(){
                storage::load(legacy_save).map_err(|e|
                    format!("Legacy project must be recovered before migration: {e}"))?
            }else{World::new()};
            let label=if legacy_save.exists(){"Imported previous project"}else{"Untitled World"};
            store.create(label,GameTarget::EmeraldExpansion,&original)?.project_id
        };
        let manifest=store.load_manifest(&id)?;
        let world=store.load_world(&id)?;
        store.activate(&id)?;
        Ok((store,manifest,world))
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn root(label:&str)->PathBuf{
        std::env::temp_dir().join(format!("mapstudio-workspace-{label}-{}",Uuid::new_v4()))
    }
    #[test]
    fn remote_clone_preserves_id_and_never_overwrites_local_copy(){
        let dir=root("remote-clone");
        let store=Workspace{root:dir.clone()};
        let template=ProjectManifest{
            manifest_version:MANIFEST_VERSION,
            project_id:Uuid::new_v4().to_string(),name:"Existing GitHub world".into(),
            target:GameTarget::EmeraldExpansion,
            target_repository:GameTarget::EmeraldExpansion.repository().into(),
            target_commit:None,
        };
        let mut world=World::new();
        world.add_map("Town",crate::model::GridPoint::new(1,2),8,8).unwrap();
        store.import_existing(&template,&world).unwrap();
        assert_eq!(store.load_world(&template.project_id).unwrap(),world);
        assert!(store.import_existing(&template,&World::new()).is_err());
        assert_eq!(store.load_world(&template.project_id).unwrap(),world);
        let _=fs::remove_dir_all(dir);
    }
    #[test]
    fn separate_projects_may_share_map_numbers_but_not_paths(){
        let dir=root("isolated");
        let store=Workspace{root:dir.clone()};
        let mut a=World::new();
        a.add_map("Town",crate::model::GridPoint::new(0,0),8,8).unwrap();
        let first=store.create("Emerald World",GameTarget::EmeraldExpansion,&a).unwrap();
        let second=store.create("Other GBA",GameTarget::FireRed,&a).unwrap();
        assert_ne!(first.project_id,second.project_id);
        assert_ne!(store.project_path(&first.project_id).unwrap(),
            store.project_path(&second.project_id).unwrap());
        store.activate(&first.project_id).unwrap();
        assert_eq!(store.active_id().unwrap(),Some(first.project_id.clone()));
        let mut one=store.load_world(&first.project_id).unwrap();
        one.add_map("Other",crate::model::GridPoint::new(20,0),8,8).unwrap();
        storage::save(&store.project_path(&first.project_id).unwrap(),&one).unwrap();
        assert_eq!(store.load_world(&second.project_id).unwrap().maps.len(),1);
        assert_eq!(store.list().unwrap().len(),2);
        assert!(!second.target.can_export());
        let _=fs::remove_dir_all(dir);
    }
    #[test]
    fn old_autosave_is_copied_not_replaced(){
        let dir=root("legacy");
        fs::create_dir_all(&dir).unwrap();
        let path=dir.join("map-studio-project.json");
        let mut world=World::new();
        world.add_map("Existing",crate::model::GridPoint::new(0,0),10,10).unwrap();
        storage::save(&path,&world).unwrap();
        let original=fs::read(&path).unwrap();
        let (store,manifest,loaded)=Workspace::open(&path).unwrap();
        assert_eq!(loaded.maps.len(),1);
        assert_eq!(fs::read(&path).unwrap(),original);
        assert!(store.project_path(&manifest.project_id).unwrap().exists());
        let (_,again,_)=Workspace::open(&path).unwrap();
        assert_eq!(again.project_id,manifest.project_id);
        let _=fs::remove_dir_all(dir);
    }
    #[test]
    fn broken_legacy_is_never_overwritten(){
        let dir=root("invalid");
        fs::create_dir_all(&dir).unwrap();
        let path=dir.join("map-studio-project.json");
        fs::write(&path,b"broken project").unwrap();
        assert!(Workspace::open(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(),b"broken project");
        assert!(!dir.join("Editor").exists());
        let _=fs::remove_dir_all(dir);
    }
    #[test]
    fn broken_manifest_prevents_unnoticed_project_fork(){
        let dir=root("manifest");
        let store=Workspace{root:dir.clone()};
        let p=store.create("Stable",GameTarget::EmeraldExpansion,&World::new()).unwrap();
        fs::write(store.manifest_path(&p.project_id).unwrap(),"{}").unwrap();
        assert!(Workspace::open(&dir.join("legacy.json")).is_err());
        assert_eq!(fs::read_dir(store.projects_root()).unwrap().count(),1);
        let _=fs::remove_dir_all(dir);
    }
    #[test]
    fn rejects_id_traversal_and_wrong_folder_identity(){
        let store=Workspace{root:root("names")};
        assert!(store.project_path("../../other").is_err());
        assert!(store.project_path("fake").is_err());
        assert!(store.project_path("00000000-0000-0000-0000-000000000000").is_ok());
    }
}
