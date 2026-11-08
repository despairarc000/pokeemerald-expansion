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
    pub local_hash:String,
    pub outcome:Result<SyncOutcome,String>,
}
struct RemoteFile{sha:String,bytes:Vec<u8>}
fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
pub fn hash_project(bytes:&[u8])->String{digest(bytes)}
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
fn get_file(client:&Client,link:&GithubLink,token:&str,path:&str,max:usize)
    ->Result<Option<RemoteFile>,String>{
    let url=url_for(link,path)?;
    let response=request(client,token,&url).send()
        .map_err(|e|format!("GitHub request failed: {e}"))?;
    if response.status()==StatusCode::NOT_FOUND{return Ok(None);}
    if !response.status().is_success(){
        return Err(format!("GitHub request failed with HTTP {}",response.status()));
    }
    let v:Value=response.json().map_err(|e|format!("Parse GitHub reply: {e}"))?;
    if v["type"].as_str()!=Some("file"){return Err("GitHub path is not a regular file".into());}
    let sha=v["sha"].as_str().ok_or("GitHub file lacks SHA")?.to_string();
    if sha.len()!=40 || !sha.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err("GitHub returned an invalid file SHA".into());
    }
    let size=v["size"].as_u64().ok_or("GitHub file lacks byte size")?;
    if size>max as u64 {return Err("GitHub file exceeds Map Studio limit".into());}
    let data=if v["encoding"].as_str()==Some("base64"){
        decode_content(&v,max)?
    }else{
        // GitHub's Contents API omits inline Base64 for large files.
        fetch_blob(client,link,token,&sha,max)?
    };
    Ok(Some(RemoteFile{sha,bytes:data}))
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
fn cursor(project:&ProjectManifest,link:&GithubLink,sha:String,hash:String)
    ->SyncCursor{
    SyncCursor{
        project_id:project.project_id.clone(),repository:link.repository.clone(),
        branch:link.branch.clone(),remote_sha:sha,local_hash:hash,
    }
}

pub fn perform(action:SyncAction,link:GithubLink,token:String,
    project:ProjectManifest,local_bytes:Vec<u8>,baseline:Option<SyncCursor>)
    ->Result<SyncOutcome,String>{
    link.validate()?;
    project.validate()?;
    let local_world=storage::decode(&local_bytes)?;
    local_world.check_integrity()?;
    let hash=digest(&local_bytes);
    let client=Client::builder().timeout(Duration::from_secs(25))
        .build().map_err(|e|e.to_string())?;
    let prefix=format!("Editor/projects/{}",project.project_id);
    let remote_manifest=get_file(&client,&link,&token,
        &format!("{prefix}/manifest.json"),8192)?;
    if let Some(rm)=&remote_manifest{
        let remote_project:ProjectManifest=serde_json::from_slice(&rm.bytes)
            .map_err(|_|"Remote project manifest is invalid")?;
        remote_project.validate()?;
        if remote_project!=project{
            return Err("GitHub project ID, target or manifest differs. Refusing cross-project sync.".into());
        }
    }
    let remote=get_file(&client,&link,&token,
        &format!("{prefix}/project.json"),storage::MAX_PROJECT_BYTES)?;
    if remote.is_some() && remote_manifest.is_none(){
        return Err("Remote project has no manifest; refusing to sync".into());
    }
    let base=baseline.filter(|b|b.matches(&project,&link));
    match action {
        SyncAction::Pull=>{
            let Some(remote)=remote else{
                return Ok(SyncOutcome::Unchanged{
                    cursor:None,message:"No remote project published yet".into()
                });
            };
            if let Some(b)=&base{
                if remote.sha==b.remote_sha{
                    return Ok(SyncOutcome::Unchanged{
                        cursor:None,
                        message:if hash!=b.local_hash{
                            "Local edits are waiting to be published".into()
                        }else{"Already synchronized".into()},
                    });
                }
                if hash!=b.local_hash{
                    return Err("Conflict: both the phone and GitHub changed. Export a backup and resolve manually.".into());
                }
            }else if digest(&remote.bytes)!=hash{
                return Err("Remote project differs from the phone's unpaired copy. Import or reconcile it before syncing.".into());
            }
            let _world=storage::decode(&remote.bytes)?;
            let new_cursor=cursor(&project,&link,remote.sha,digest(&remote.bytes));
            if digest(&remote.bytes)==hash{
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
                    Ok(SyncOutcome::Published{cursor:cursor(&project,&link,sha,hash)})
                }
                Some(remote)=>{
                    let remote_hash=digest(&remote.bytes);
                    if let Some(b)=&base {
                        if remote.sha!=b.remote_sha{
                            if remote_hash==hash{
                                return Ok(SyncOutcome::Unchanged{
                                    cursor:Some(cursor(&project,&link,remote.sha,hash)),
                                    message:"Already current on GitHub".into(),
                                });
                            }
                            return Err("Conflict: GitHub changed since the last sync. Pull/review before publishing.".into());
                        }
                    }else if remote_hash!=hash{
                        return Err("GitHub contains a different unpaired project; refusing to overwrite it.".into());
                    }
                    if remote_hash==hash{
                        return Ok(SyncOutcome::Unchanged{
                            cursor:Some(cursor(&project,&link,remote.sha,hash)),
                            message:"Already current on GitHub".into(),
                        });
                    }
                    let sha=put_file(&client,&link,&token,
                        &format!("{prefix}/project.json"),&local_bytes,Some(&remote.sha))?;
                    Ok(SyncOutcome::Published{cursor:cursor(&project,&link,sha,hash)})
                }
            }
        }
    }
}

#[cfg(test)]
mod tests{
    use super::*;
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
        let c=cursor(&project,&link,"f".repeat(40),"a".repeat(64));
        assert!(c.matches(&project,&link));
        let mut different=link.clone();different.branch="other".into();
        assert!(!c.matches(&project,&different));
    }
}
