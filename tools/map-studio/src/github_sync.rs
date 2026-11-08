//! Opt-in GitHub Contents API transport for one Map Studio project.
//! HTTPS only; no server, no webhook, no GitHub credentials embedded in APK.
//! Access token is passed in memory and MUST NEVER be written to disk.
use crate::{projects::ProjectManifest, storage};
use base64::{Engine as _,engine::general_purpose::STANDARD};
use reqwest::{blocking::{Client,RequestBuilder},StatusCode};
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};
use sha2::{Digest,Sha256};
use std::{fs,io::Write,path::{Path,PathBuf},time::Duration};

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubLink {
    pub repository:String,
    pub branch:String,
    #[serde(default)]
    pub auto_pull:bool,
}
impl GithubLink{
    pub fn validate(&self)->Result<(),String>{
        let parts:Vec<_>=self.repository.split('/').collect();
        if parts.len()!=2 || parts.iter().any(|p|p.is_empty() || p.len()>100
            || *p=="." || *p==".." || !p.bytes().all(|b|b.is_ascii_alphanumeric()
                || matches!(b,b'-'|b'_'|b'.'))){
            return Err("GitHub repository must be owner/repo".into());
        }
        if self.branch.is_empty() || self.branch.len()>120
            || self.branch.contains("..") || self.branch.starts_with('/')
            || self.branch.ends_with('/')
            || self.branch.contains("//")
            || !self.branch.bytes().all(|b|b.is_ascii_alphanumeric()
                || matches!(b,b'-'|b'_'|b'.'|b'/')){
            return Err("Invalid GitHub branch name".into());
        }
        Ok(())
    }
}

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncCursor {
    pub project_id:String,
    pub repository:String,
    pub branch:String,
    /// Git blob SHA used as GitHub's optimistic concurrency precondition.
    pub remote_sha:String,
    /// SHA-256 of the complete local project.json at last successful sync.
    pub local_hash:String,
    #[serde(default)]
    pub remote_etag:Option<String>,
}
impl SyncCursor {
    fn matches(&self,project:&ProjectManifest,link:&GithubLink)->bool{
        self.project_id==project.project_id
            && self.repository==link.repository && self.branch==link.branch
            && self.remote_sha.len()==40
            && self.remote_sha.bytes().all(|b|b.is_ascii_hexdigit())
            && self.local_hash.len()==64
            && self.local_hash.bytes().all(|b|b.is_ascii_hexdigit())
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SyncAction{Pull,Publish}
#[derive(Debug)]
pub enum SyncOutcome {
    Unchanged{cursor:Option<SyncCursor>,message:String},
    Incoming{bytes:Vec<u8>,cursor:SyncCursor},
    Published{cursor:SyncCursor},
}
pub struct SyncReply{
    pub project_id:String,
    pub repository:String,
    pub branch:String,
    pub local_hash:String,
    pub outcome:Result<SyncOutcome,String>,
}
struct RemoteFile{sha:String,bytes:Vec<u8>,etag:Option<String>}
enum RemoteResponse{File(RemoteFile),Missing,NotModified}
fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
/// Hash normalized world data, not the indentation used by an AI editor.
pub fn hash_project(bytes:&[u8])->String{
    // Malformed project inputs are rejected by storage::decode before sync;
    // hashing their raw bytes keeps this utility total for UI diagnostics.
    match storage::decode(bytes).and_then(|world|
        serde_json::to_vec(&storage::Project::new(world)).map_err(|e|e.to_string()))
    {
        Ok(canonical)=>digest(&canonical),
        Err(_)=>digest(bytes),
    }
}
fn cursor_path(project_file:&Path)->PathBuf{project_file.with_file_name("sync.json")}
fn link_path(project_file:&Path)->PathBuf{project_file.with_file_name("github-link.json")}
fn safe_write(path:&Path,bytes:&[u8])->Result<(),String>{
    let temp=path.with_extension("pending");
    let result=(||->Result<(),String>{
        let mut file=fs::File::create(&temp).map_err(|e|e.to_string())?;
        file.write_all(bytes).map_err(|e|e.to_string())?;
        file.sync_all().map_err(|e|e.to_string())?;
        fs::rename(&temp,path).map_err(|e|e.to_string())
    })();
    if result.is_err(){let _=fs::remove_file(temp);}
    result
}
pub fn load_link(project_file:&Path)->Result<Option<GithubLink>,String>{
    let path=link_path(project_file);
    if !path.exists(){return Ok(None);}
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    if bytes.len()>2048{return Err("GitHub link metadata too large".into());}
    let link:GithubLink=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    link.validate()?;
    Ok(Some(link))
}
pub fn save_link(project_file:&Path,link:&GithubLink)->Result<(),String>{
    link.validate()?;
    safe_write(&link_path(project_file),
        &serde_json::to_vec(link).map_err(|e|e.to_string())?)
}
pub fn load_cursor(project_file:&Path)->Result<Option<SyncCursor>,String>{
    let path=cursor_path(project_file);
    if !path.exists(){return Ok(None);}
    let bytes=fs::read(path).map_err(|e|e.to_string())?;
    if bytes.len()>2048{return Err("GitHub cursor metadata too large".into());}
    let cursor:SyncCursor=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
    Ok(Some(cursor))
}
pub fn save_cursor(project_file:&Path,cursor:&SyncCursor)->Result<(),String>{
    safe_write(&cursor_path(project_file),
        &serde_json::to_vec(cursor).map_err(|e|e.to_string())?)
}
fn request(client:&Client,token:&str,url:&str)->RequestBuilder{
    let req=client.get(url)
        .header("Accept","application/vnd.github+json")
        .header("User-Agent","MapStudio-ProjectSync/1")
        .header("X-GitHub-Api-Version","2022-11-28");
    if token.trim().is_empty(){req}else{req.bearer_auth(token.trim())}
}
fn url_for(link:&GithubLink,path:&str)->Result<String,String>{
    link.validate()?;
    // The only variable file path is a validated UUID and fixed file name.
    let mut url=reqwest::Url::parse(&format!("https://api.github.com/repos/{}/contents/{path}",
        link.repository)).map_err(|e|e.to_string())?;
    url.query_pairs_mut().append_pair("ref",&link.branch);
    Ok(url.to_string())
}
fn fetch_blob(client:&Client,link:&GithubLink,token:&str,sha:&str,max:usize)
    ->Result<Vec<u8>,String>{
    let url=format!("https://api.github.com/repos/{}/git/blobs/{sha}",
        link.repository);
    let response=request(client,token,&url).send()
        .map_err(|e|format!("Read Git blob: {e}"))?;
    if !response.status().is_success(){
        return Err(format!("GitHub blob request failed with HTTP {}",response.status()));
    }
    let value:Value=response.json().map_err(|e|e.to_string())?;
    decode_content(&value,max)
}
fn decode_content(v:&Value,max:usize)->Result<Vec<u8>,String>{
    let content=v["content"].as_str().ok_or("Missing GitHub file contents")?;
    if content.len()>((max*4/3)+8192){
        return Err("GitHub project exceeds the allowed transfer size".into());
    }
    let compact:String=content.chars().filter(|c|!c.is_ascii_whitespace()).collect();
    let data=STANDARD.decode(compact).map_err(|_|"Invalid GitHub base64 data")?;
    if data.len()>max{return Err("Downloaded project exceeds the allowed size".into());}
    Ok(data)
}
fn get_file(client:&Client,link:&GithubLink,token:&str,path:&str,
    max:usize,conditional_etag:Option<&str>)->Result<RemoteResponse,String>{
    let url=url_for(link,path)?;
    let mut req=request(client,token,&url);
    if let Some(etag)=conditional_etag{
        if !etag.is_empty() && etag.len()<200 && !etag.contains('\n'){
            req=req.header(reqwest::header::IF_NONE_MATCH,etag);
        }
    }
    let response=req.send().map_err(|e|format!("GitHub request failed: {e}"))?;
    if response.status()==StatusCode::NOT_MODIFIED{return Ok(RemoteResponse::NotModified);}
    if response.status()==StatusCode::NOT_FOUND{return Ok(RemoteResponse::Missing);}
    if !response.status().is_success(){
        return Err(format!("GitHub request failed with HTTP {}",response.status()));
    }
    let etag=response.headers().get(reqwest::header::ETAG)
        .and_then(|v|v.to_str().ok()).filter(|e|e.len()<200)
        .map(str::to_string);
    let v:Value=response.json().map_err(|e|format!("Parse GitHub reply: {e}"))?;
    if v["type"].as_str()!=Some("file"){return Err("GitHub path is not a regular file".into());}
    let sha=v["sha"].as_str().ok_or("GitHub file lacks SHA")?.to_string();
    if sha.len()!=40 || !sha.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err("GitHub returned an invalid file SHA".into());
    }
    let size=v["size"].as_u64().ok_or("GitHub file lacks byte size")?;
    if size>max as u64{return Err("GitHub file exceeds Map Studio limit".into());}
    let data=if v["encoding"].as_str()==Some("base64"){
        decode_content(&v,max)?
    }else{
        fetch_blob(client,link,token,&sha,max)?
    };
    Ok(RemoteResponse::File(RemoteFile{sha,bytes:data,etag}))
}
fn put_file(client:&Client,link:&GithubLink,token:&str,path:&str,
    contents:&[u8],old_sha:Option<&str>)->Result<String,String>{
    if token.trim().is_empty(){
        return Err("Publishing requires a fine-grained GitHub token with Contents write access".into());
    }
    let url=url_for(link,path)?;
    let mut payload=json!({
        "message":format!("Map Studio: update {path}"),
        "content":STANDARD.encode(contents),
        "branch":link.branch
    });
    if let Some(sha)=old_sha{payload["sha"]=json!(sha);}
    let resp=client.put(url)
        .header("Accept","application/vnd.github+json")
        .header("User-Agent","MapStudio-ProjectSync/1")
        .header("X-GitHub-Api-Version","2022-11-28")
        .bearer_auth(token.trim())
        .json(&payload).send()
        .map_err(|e|format!("Publish project file: {e}"))?;
    if !resp.status().is_success(){
        return Err(format!("GitHub rejected the update (HTTP {}). Local project was not replaced.",
            resp.status()));
    }
    let v:Value=resp.json().map_err(|e|e.to_string())?;
    let sha=v["content"]["sha"].as_str()
        .ok_or("GitHub did not return the new file SHA")?;
    Ok(sha.into())
}
fn cursor(project:&ProjectManifest,link:&GithubLink,sha:String,hash:String,
    remote_etag:Option<String>)->SyncCursor{
    SyncCursor{
        project_id:project.project_id.clone(),repository:link.repository.clone(),
        branch:link.branch.clone(),remote_sha:sha,local_hash:hash,remote_etag
    }
}

// Pure conflict decisions are deliberately separate from HTTP I/O so that
// simultaneous edits can be tested without publishing throwaway projects.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum PullDecision { Current, LocalPending, ApplyRemote, Paired }

fn assess_pull(base:Option<&SyncCursor>,remote_sha:&str,
    local_hash:&str,remote_hash:&str)->Result<PullDecision,String>{
    if let Some(b)=base{
        if remote_sha==b.remote_sha {
            return Ok(if local_hash==b.local_hash{
                PullDecision::Current
            }else{PullDecision::LocalPending});
        }
        if local_hash!=b.local_hash {
            return Err("Conflict: both the phone and GitHub changed. Export a backup and resolve manually.".into());
        }
    } else if remote_hash!=local_hash {
        return Err("Remote project differs from the phone's unpaired copy. Import or reconcile it before syncing.".into());
    }
    Ok(if remote_hash==local_hash{PullDecision::Paired}
       else{PullDecision::ApplyRemote})
}
fn assess_publish(base:Option<&SyncCursor>,remote_sha:&str,
    local_hash:&str,remote_hash:&str)->Result<bool,String>{
    if let Some(b)=base {
        if remote_sha!=b.remote_sha {
            if remote_hash==local_hash{return Ok(false);}
            return Err("Conflict: GitHub changed since the last sync. Pull/review before publishing.".into());
        }
    }else if remote_hash!=local_hash{
        return Err("GitHub contains a different unpaired project; refusing to overwrite it.".into());
    }
    Ok(remote_hash!=local_hash)
}

pub fn perform(action:SyncAction,link:GithubLink,token:String,
    project:ProjectManifest,local_bytes:Vec<u8>,baseline:Option<SyncCursor>)
    ->Result<SyncOutcome,String>{
    link.validate()?;
    project.validate()?;
    let local_world=storage::decode(&local_bytes)?;
    local_world.check_integrity()?;
    let hash=hash_project(&local_bytes);
    let client=Client::builder().timeout(Duration::from_secs(25))
        .build().map_err(|e|e.to_string())?;
    let prefix=format!("Editor/projects/{}",project.project_id);
    let remote_manifest=match get_file(&client,&link,&token,
        &format!("{prefix}/manifest.json"),8192,None)?{
        RemoteResponse::File(file)=>Some(file),
        RemoteResponse::Missing=>None,
        RemoteResponse::NotModified=>return Err("Unexpected cached manifest response".into()),
    };
    if let Some(rm)=&remote_manifest{
        let remote_project:ProjectManifest=serde_json::from_slice(&rm.bytes)
            .map_err(|_|"Remote project manifest is invalid")?;
        remote_project.validate()?;
        if remote_project!=project{
            return Err("GitHub project ID, target or manifest differs. Refusing cross-project sync.".into());
        }
    }
    let base=baseline.filter(|b|b.matches(&project,&link));
    let conditional=if action==SyncAction::Pull{
        base.as_ref().and_then(|b|b.remote_etag.as_deref())
    }else{None};
    let remote=match get_file(&client,&link,&token,
        &format!("{prefix}/project.json"),storage::MAX_PROJECT_BYTES,conditional)?{
        RemoteResponse::File(file)=>Some(file),
        RemoteResponse::Missing=>None,
        RemoteResponse::NotModified=>{
            let pending=base.as_ref().is_some_and(|b|b.local_hash!=hash);
            return Ok(SyncOutcome::Unchanged{cursor:None,message:
                if pending{"Local changes are waiting to be published"}
                else{"Already synchronized (no GitHub changes)"}.into()});
        }
    };
    if remote.is_some() && remote_manifest.is_none(){
        return Err("Remote project has no manifest; refusing to sync".into());
    }
    match action {
        SyncAction::Pull=>{
            let Some(remote)=remote else{
                return Ok(SyncOutcome::Unchanged{
                    cursor:None,message:"No remote project published yet".into()
                });
            };
            let remote_hash=hash_project(&remote.bytes);
            let decision=assess_pull(base.as_ref(),&remote.sha,&hash,&remote_hash)?;
            match decision {
                PullDecision::Current => return Ok(SyncOutcome::Unchanged{
                    cursor:Some(cursor(&project,&link,remote.sha,remote_hash,remote.etag)),
                    message:"Already synchronized".into()
                }),
                PullDecision::LocalPending => return Ok(SyncOutcome::Unchanged{
                    cursor:None,message:"Local edits are waiting to be published".into()
                }),
                PullDecision::ApplyRemote | PullDecision::Paired => {}
            }
            let _world=storage::decode(&remote.bytes)?;
            let new_cursor=cursor(&project,&link,remote.sha,remote_hash,remote.etag);
            if decision==PullDecision::Paired{
                return Ok(SyncOutcome::Unchanged{
                    cursor:Some(new_cursor),message:"Paired with matching GitHub project".into(),
                });
            }
            Ok(SyncOutcome::Incoming{bytes:remote.bytes,cursor:new_cursor})
        }
        SyncAction::Publish=>{
            if token.trim().is_empty(){
                return Err("Publishing requires a fine-grained GitHub token (Contents write)".into());
            }
            match remote{
                None=>{
                    if remote_manifest.is_none(){
                        let manifest_bytes=serde_json::to_vec_pretty(&project)
                            .map_err(|e|e.to_string())?;
                        put_file(&client,&link,&token,&format!("{prefix}/manifest.json"),
                            &manifest_bytes,None)?;
                    }
                    let sha=put_file(&client,&link,&token,
                        &format!("{prefix}/project.json"),&local_bytes,None)?;
                    Ok(SyncOutcome::Published{cursor:cursor(&project,&link,sha,hash,None)})
                }
                Some(remote)=>{
                    let remote_hash=hash_project(&remote.bytes);
                    if !assess_publish(base.as_ref(),&remote.sha,&hash,&remote_hash)? {
                        return Ok(SyncOutcome::Unchanged{
                            cursor:Some(cursor(&project,&link,remote.sha,hash,remote.etag)),
                            message:"Already current on GitHub".into(),
                        });
                    }
                    let sha=put_file(&client,&link,&token,
                        &format!("{prefix}/project.json"),&local_bytes,Some(&remote.sha))?;
                    Ok(SyncOutcome::Published{cursor:cursor(&project,&link,sha,hash,None)})
                }
            }
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn simulate_remote_edit_phone_edit_and_first_pairing(){
        let hash_a="a".repeat(64);
        let hash_b="b".repeat(64);
        let hash_c="c".repeat(64);
        let baseline=SyncCursor{
            project_id:uuid::Uuid::new_v4().to_string(),
            repository:"owner/repo".into(),branch:"main".into(),
            remote_sha:"1".repeat(40),local_hash:hash_a.clone(),remote_etag:None,
        };
        assert_eq!(assess_pull(Some(&baseline),&baseline.remote_sha,
            &hash_a,&hash_a).unwrap(),PullDecision::Current);
        assert_eq!(assess_pull(Some(&baseline),&baseline.remote_sha,
            &hash_b,&hash_a).unwrap(),PullDecision::LocalPending);
        assert_eq!(assess_pull(Some(&baseline),&"2".repeat(40),
            &hash_a,&hash_b).unwrap(),PullDecision::ApplyRemote);
        assert!(assess_pull(Some(&baseline),&"2".repeat(40),
            &hash_c,&hash_b).unwrap_err().contains("Conflict"));
        assert_eq!(assess_pull(None,&"2".repeat(40),
            &hash_a,&hash_a).unwrap(),PullDecision::Paired);
        assert!(assess_pull(None,&"2".repeat(40),
            &hash_a,&hash_b).unwrap_err().contains("unpaired"));
        assert_eq!(assess_publish(Some(&baseline),&baseline.remote_sha,
            &hash_b,&hash_a).unwrap(),true);
        assert!(assess_publish(Some(&baseline),&"2".repeat(40),
            &hash_b,&hash_c).unwrap_err().contains("Conflict"));
        assert!(!assess_publish(Some(&baseline),&"2".repeat(40),
            &hash_b,&hash_b).unwrap());
        assert!(assess_publish(None,&"2".repeat(40),
            &hash_b,&hash_a).unwrap_err().contains("unpaired"));
    }
    #[test]
    fn same_world_with_different_json_format_has_same_sync_hash(){
        let world=crate::model::World::new();
        let compact=serde_json::to_vec(&storage::Project::new(world.clone())).unwrap();
        let pretty=serde_json::to_vec_pretty(&storage::Project::new(world)).unwrap();
        assert_ne!(compact,pretty);
        assert_eq!(hash_project(&compact),hash_project(&pretty));
        assert_eq!(hash_project(&compact).len(),64);
    }
    #[test]
    fn actual_map_edit_changes_canonical_hash(){
        let mut world=crate::model::World::new();
        let before=serde_json::to_vec(&storage::Project::new(world.clone())).unwrap();
        world.add_map("New Area",crate::model::GridPoint::new(0,0),8,8).unwrap();
        let after=serde_json::to_vec_pretty(&storage::Project::new(world)).unwrap();
        assert_ne!(hash_project(&before),hash_project(&after));
    }
    #[test]
    fn validates_remote_addresses_without_allowing_other_hosts(){
        let good=GithubLink{
            repository:"despairarc000/map-projects".into(),branch:"develop/maps".into(),
            auto_pull:true,
        };
        assert!(good.validate().is_ok());
        assert!(url_for(&good,"Editor/projects/example/project.json").unwrap()
            .starts_with("https://api.github.com/repos/"));
        for repo in ["https://evil.example/repo","../other","owner/repo/other","x//y"]{
            let mut test=good.clone();test.repository=repo.into();
            assert!(test.validate().is_err());
        }
        let mut test=good.clone();test.branch="../main".into();
        assert!(test.validate().is_err());
    }
    #[test]
    fn credentials_never_enter_persisted_metadata(){
        let link=GithubLink{
            repository:"example/maps".into(),branch:"main".into(),auto_pull:false,
        };
        let data=serde_json::to_string(&link).unwrap();
        assert!(!data.contains("token"));
    }
    #[test]
    fn cursor_scope_is_project_and_repo_specific(){
        let project=ProjectManifest{
            manifest_version:1,project_id:uuid::Uuid::new_v4().to_string(),
            name:"Test".into(),target:crate::projects::GameTarget::EmeraldExpansion,
            target_repository:"despairarc000/pokeemerald-expansion".into(),
            target_commit:None,
        };
        let link=GithubLink{
            repository:"example/map-projects".into(),branch:"main".into(),auto_pull:false,
        };
        let c=cursor(&project,&link,"f".repeat(40),"a".repeat(64),None);
        assert!(c.matches(&project,&link));
        let mut different=link.clone();different.branch="other".into();
        assert!(!c.matches(&project,&different));
    }
}
