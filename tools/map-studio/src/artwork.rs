//! Portable artwork packages. Source-backed Emerald exports contain the
//! actual indexed tiles, metatile definitions, attributes, palette banks and
//! an atlas preview. Arbitrary imported artwork is REFERENCE ONLY until an
//! explicit conversion and native ROM-build validation pipeline exists.
use crate::{tiles::{self,TilePair},prefabs::{self,Prefab}};
use serde::{Deserialize,Serialize};
use std::{collections::HashSet,fs,io::{Read,Write},path::{Path,PathBuf}};

pub const PACKAGE_VERSION:u32=1;
pub const MAX_PACKAGE_BYTES:usize=24*1024*1024;
const MAX_READ_BYTES:usize=10*1024*1024;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Manifest {
    pub format:String,
    pub version:u32,
    pub collection:String,
    pub engine:String,
    pub source:String,
    pub attribution:String,
    pub label:String,
    pub category:String,
    pub pair_key:String,
    pub primary_symbol:String,
    pub secondary_symbol:String,
    pub primary_count:usize,
    pub secondary_count:usize,
    pub valid_mask:Vec<u8>,
}
pub struct Package {
    pub filename:String,
    pub manifest:Manifest,
    pub atlas:Vec<u8>,
    pub prefabs:Vec<Prefab>,
    /// Generated at compile time and shipped inside the APK, not downloaded.
    pub embedded_archive:Option<&'static [u8]>,
}
impl Package{
    /// Imported byte-perfect copies of built-in pairs can reuse the native
    /// compiler resources; others must NOT paint Emerald maps by ID alone.
    pub fn native_pair(&self)->Option<&'static TilePair>{
        let pair=tiles::pair(&self.manifest.pair_key)?;
        if self.manifest.engine!="pokeemerald"
            || self.manifest.primary_symbol!=pair.primary_symbol
            || self.manifest.secondary_symbol!=pair.secondary_symbol
            || self.manifest.valid_mask.as_slice()!=pair.valid_mask.as_slice()
            || self.atlas.as_slice()!=pair.atlas {return None;}
        Some(pair)
    }
    pub fn valid(&self,id:u16)->bool{
        let n=id as usize;
        n<1024 && self.manifest.valid_mask[n/8]&(1<<(n%8))!=0
    }
}
fn zip_entry(zip:&mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
             name:&str,max:usize)->Result<Vec<u8>,String>{
    let mut file=zip.by_name(name).map_err(|_|format!("Missing {name} in artwork package"))?;
    if file.size() as usize>max{return Err(format!("{name} is too large"));}
    let mut out=Vec::new();
    file.by_ref().take((max+1) as u64)
        .read_to_end(&mut out).map_err(|e|format!("Read {name}: {e}"))?;
    if out.len()>max{return Err(format!("{name} exceeds safe size"));}
    Ok(out)
}

pub fn decode(bytes:&[u8])->Result<Package,String>{
    if bytes.len()>MAX_PACKAGE_BYTES || bytes.len()<100{
        return Err("Artwork package is empty or too large (24 MiB limit)".into());
    }
    let mut zip=zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e|format!("Open artwork ZIP: {e}"))?;
    if zip.len()>90{return Err("Artwork ZIP contains too many files".into());}
    let manifest:Manifest=serde_json::from_slice(&zip_entry(&mut zip,"manifest.json",65_536)?)
        .map_err(|e|format!("Invalid artwork manifest: {e}"))?;
    if manifest.format!="mapstudio-artwork-pack" || manifest.version!=PACKAGE_VERSION{
        return Err("Unsupported artwork package format/version".into());
    }
    if manifest.collection.trim().is_empty() || manifest.collection.len()>72
        || manifest.collection.chars().any(|c|c.is_control())
        || manifest.engine.len()>48 || manifest.category.len()>52
        || manifest.pair_key.len()>160 || manifest.label.len()>80
        || manifest.attribution.len()>1024 || manifest.source.len()>1024
        || manifest.valid_mask.len()!=128
        || manifest.primary_count>512 || manifest.secondary_count>512 {
        return Err("Invalid artwork package metadata".into());
    }
    let atlas=zip_entry(&mut zip,"atlas.png",MAX_READ_BYTES)?;
    let img=image::load_from_memory_with_format(&atlas,image::ImageFormat::Png)
        .map_err(|e|format!("Artwork atlas is not a valid PNG: {e}"))?;
    if img.width()!=256 || img.height()!=1024 {
        return Err("Artwork atlas must be a 16 by 64 sheet of 16px metatiles".into());
    }
    // A complete source package, not just a random tileset thumbnail.
    for name in ["primary/tiles.png","secondary/tiles.png",
        "primary/metatiles.bin","secondary/metatiles.bin",
        "primary/metatile_attributes.bin","secondary/metatile_attributes.bin"]{
        let bytes=zip_entry(&mut zip,name,MAX_READ_BYTES)?;
        if bytes.is_empty(){return Err(format!("Missing source data in {name}"));}
        if name.ends_with("metatiles.bin") && bytes.len()%16!=0 {
            return Err(format!("{name} has invalid native metatile block length"));
        }
    }
    let prefab_file=zip_entry(&mut zip,"prefabs.json",MAX_READ_BYTES)?;
    let prefabs:Vec<Prefab>=serde_json::from_slice(&prefab_file)
        .map_err(|e|format!("Invalid prefab definitions: {e}"))?;
    if prefabs.len()>256 || prefabs.iter().any(|p|p.width==0 || p.height==0
        || p.width>32 || p.height>32 || p.metatiles.len()>1024 || p.name.len()>96
        || p.metatiles.iter().any(|tile|tile.x>=p.width || tile.y>=p.height
            || tile.metatile_id>=1024 || tile.collision>3 || tile.elevation>15)) {
        return Err("Artwork package contains oversized prefabs".into());
    }
    Ok(Package{filename:String::new(),manifest,atlas,prefabs,embedded_archive:None})
}
pub fn filename_for(collection:&str,key:&str)->String{
    let slug=format!("{collection}-{key}").chars()
        .map(|c|if c.is_ascii_alphanumeric(){c.to_ascii_lowercase()}else{'-'})
        .take(96).collect::<String>();
    format!("MapStudio-artwork-{slug}.zip")
}

pub fn export_pair(pair:&TilePair,destination:&Path)->Result<(),String>{
    let pack=Manifest{
        format:"mapstudio-artwork-pack".into(),
        version:PACKAGE_VERSION,
        collection:pair.collection.into(),engine:"pokeemerald".into(),
        source:"despairarc000/pokeemerald-expansion source tilesets".into(),
        attribution:"Native Generation III source metatiles; prefabs from huderlem/porymap (LGPL-3.0), original Pokemon artwork rights remain with owners".into(),
        label:pair.label.into(),category:pair.category.into(),pair_key:pair.key.into(),
        primary_symbol:pair.primary_symbol.into(),secondary_symbol:pair.secondary_symbol.into(),
        primary_count:pair.primary_count,secondary_count:pair.secondary_count,
        valid_mask:pair.valid_mask.to_vec(),
    };
    let prefab_list:Vec<&Prefab>=prefabs::by_collection(pair.collection).iter()
        .filter(|p|prefabs::compatible(p,pair)).collect();
    let mut entries:Vec<(String,&[u8])>=vec![
        ("atlas.png".into(),pair.atlas),
        ("primary/tiles.png".into(),pair.raw_primary_tiles),
        ("secondary/tiles.png".into(),pair.raw_secondary_tiles),
        ("primary/metatiles.bin".into(),pair.raw_primary_metatiles),
        ("secondary/metatiles.bin".into(),pair.raw_secondary_metatiles),
        ("primary/metatile_attributes.bin".into(),pair.attr_primary),
        ("secondary/metatile_attributes.bin".into(),pair.attr_secondary),
        ("LICENSE-PORYMAP.md".into(),
            include_bytes!("../assets/PORYMAP-LICENSE.md")),
        ("ATTRIBUTION-PORYMAP.md".into(),
            include_bytes!("../assets/PORYMAP-ATTRIBUTION.md"))
    ];
    for (name,bytes) in pair.raw_files{entries.push(((*name).to_string(),bytes));}
    if entries.len()>85{return Err("Too many source resources in package".into());}
    if let Some(parent)=destination.parent(){
        fs::create_dir_all(parent).map_err(|e|e.to_string())?;
    }
    let temp=destination.with_extension("zip.tmp");
    let create=(||->Result<(),String>{
        let file=fs::File::create(&temp).map_err(|e|e.to_string())?;
        let mut zip=zip::ZipWriter::new(file);
        let options=zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name,payload) in [
            ("manifest.json",serde_json::to_vec_pretty(&pack).map_err(|e|e.to_string())?),
            ("prefabs.json",serde_json::to_vec_pretty(&prefab_list).map_err(|e|e.to_string())?)
        ]{
            zip.start_file(name,options).map_err(|e|e.to_string())?;
            zip.write_all(&payload).map_err(|e|e.to_string())?;
        }
        for (name,data) in entries{
            zip.start_file(name,options).map_err(|e|e.to_string())?;
            zip.write_all(data).map_err(|e|e.to_string())?;
        }
        let output=zip.finish().map_err(|e|e.to_string())?;
        output.sync_all().map_err(|e|e.to_string())?;
        Ok(())
    })();
    if let Err(err)=create{let _=fs::remove_file(&temp);return Err(err);}
    fs::rename(&temp,destination).map_err(|e|e.to_string())?;
    Ok(())
}

pub fn install(dir:&Path,bytes:&[u8])->Result<Package,String>{
    let mut package=decode(bytes)?;
    fs::create_dir_all(dir).map_err(|e|format!("Create artwork library: {e}"))?;
    let file=filename_for(&package.manifest.collection,&package.manifest.pair_key);
    let path=dir.join(&file);
    // Persist only after the ZIP parses, source files and metadata pass validation.
    let temp=dir.join(format!("{file}.tmp"));
    fs::write(&temp,bytes).map_err(|e|e.to_string())?;
    fs::rename(&temp,&path).map_err(|e|e.to_string())?;
    package.filename=file;
    Ok(package)
}
pub fn load_installed(dir:&Path)->Vec<Package>{
    let Ok(items)=fs::read_dir(dir) else{return Vec::new();};
    let mut packages=Vec::new();
    let mut seen=HashSet::new();
    for entry in items.flatten(){
        let Some(name)=entry.file_name().to_str().map(str::to_owned) else{continue;};
        if !name.starts_with("MapStudio-artwork-") || !name.ends_with(".zip")
            || !seen.insert(name.clone()) {continue;}
        if packages.len()>=24{break;}
        if let Ok(meta)=entry.metadata(){
            if meta.len()>MAX_PACKAGE_BYTES as u64{continue;}
        }
        let Ok(bytes)=fs::read(entry.path()) else{continue;};
        if let Ok(mut pack)=decode(&bytes){
            pack.filename=name;
            packages.push(pack);
        }
    }
    packages.sort_by(|a,b|a.filename.cmp(&b.filename));
    packages
}
/// Genuine source resources compiled into the editor. These packs do not
/// claim that a different game's metatile IDs are safe to paint in Emerald.
pub fn builtin_reference_packs()->Vec<Package>{
    let archives:&[(&str,&'static [u8])]=&[
        ("ruby_petalburg",include_bytes!(concat!(env!("OUT_DIR"),"/builtin_packs/ruby_petalburg.zip"))),
        ("ruby_mauville",include_bytes!(concat!(env!("OUT_DIR"),"/builtin_packs/ruby_mauville.zip"))),
        ("ruby_rustboro",include_bytes!(concat!(env!("OUT_DIR"),"/builtin_packs/ruby_rustboro.zip"))),
        ("oras_petalburg",include_bytes!(concat!(env!("OUT_DIR"),"/builtin_packs/oras_petalburg.zip"))),
        ("oras_mauville",include_bytes!(concat!(env!("OUT_DIR"),"/builtin_packs/oras_mauville.zip"))),
        ("oras_rustboro",include_bytes!(concat!(env!("OUT_DIR"),"/builtin_packs/oras_rustboro.zip"))),
    ];
    archives.iter().map(|(name,bytes)|{
        let mut package=decode(bytes).expect("Built-in Ruby art archive must decode");
        package.filename=format!("MapStudio-artwork-builtin-{name}.zip");
        package.embedded_archive=Some(bytes);
        package
    }).collect()
}

pub fn default_dir(save_path:Option<&Path>)->Option<PathBuf>{
    save_path.and_then(Path::parent).map(|p|p.join("artwork-packages"))
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn roundtrip_preserves_native_resources_and_prefabs(){
        let pair=tiles::pair(tiles::DEFAULT_PAIR).unwrap();
        let path=std::env::temp_dir().join(format!("mapstudio-artwork-{}.zip",std::process::id()));
        export_pair(pair,&path).unwrap();
        let bytes=fs::read(&path).unwrap();
        let pack=decode(&bytes).unwrap();
        assert!(pack.native_pair().is_some());
        assert!(!pack.prefabs.is_empty());
        assert_eq!(pack.manifest.primary_symbol,pair.primary_symbol);
        let _=fs::remove_file(&path);
    }
    #[test]
    fn original_ruby_builtins_have_real_tiles_and_prefabs(){
        let packs=builtin_reference_packs();
        assert_eq!(packs.len(),6);
        assert_eq!(packs.iter().filter(|p|p.manifest.collection=="Ruby").count(),3);
        assert_eq!(packs.iter().filter(|p|p.manifest.collection=="ORAS Style").count(),3);
        assert!(packs.iter().all(|p|!p.atlas.is_empty()
            && p.manifest.valid_mask.iter().any(|x|*x!=0)
            && p.embedded_archive.is_some()
            && p.native_pair().is_none()));
        assert!(packs.iter().filter(|p|p.manifest.collection=="Ruby")
            .any(|p|!p.prefabs.is_empty()));
    }
    #[test]
    fn zip_bomb_or_unrelated_zip_cannot_be_imported(){
        assert!(decode(&vec![0;MAX_PACKAGE_BYTES+1]).is_err());
        assert!(decode(b"not a ZIP archive").is_err());
    }
}
