//! The mascot's ordinary local chat, separate from voice features.
use std::path::{Path, PathBuf};
use crate::inbox::ThreadSummary;
pub fn folder(home: &Path) -> PathBuf { home.join("ThreadbearThreads").join("Bear") }
pub fn prepare(home:&Path)->std::io::Result<PathBuf> {
    let folder=folder(home);
    crate::private_file::dir(&folder)?;
    crate::private_file::write_if_absent(&folder.join("CLAUDE.md"),include_bytes!("bear_chat/instructions.md"))?;
    Ok(folder)
}
pub fn newest<'a>(home:&Path,threads:impl Iterator<Item=&'a ThreadSummary>)->Option<&'a ThreadSummary> {
    let wanted=folder(home);
    threads.filter(|t| !t.is_remote() && t.archived_at.is_none() && {
        let path=t.cwd.strip_prefix("~/").map(|rest|home.join(rest)).unwrap_or_else(||PathBuf::from(&t.cwd));
        normalized(&path)==normalized(&wanted)
    }).max_by_key(|t|chrono::DateTime::parse_from_rfc3339(&t.last_activity_at).map(|d|d.timestamp_millis()).unwrap_or_default())
}
fn normalized(path:&Path)->PathBuf {
    let mut out=PathBuf::new();
    for part in path.components() {
        match part {std::path::Component::CurDir=>{},std::path::Component::ParentDir=>{out.pop();},_=>out.push(part.as_os_str())}
    }
    out
}
#[cfg(test)]
mod tests {
    use super::{folder,prepare,newest};
    use crate::{inbox::ThreadSummary,private_file};
    #[test]
    fn prepares_once_and_keeps_the_users_instructions() {
        let home=std::env::temp_dir().join(format!("tb-bear-{}",std::process::id()));
        let _=std::fs::remove_dir_all(&home);
        let path=prepare(&home).unwrap();
        assert_eq!(path,folder(&home));
        assert!(std::fs::read_to_string(path.join("CLAUDE.md")).unwrap().contains("Remember"));
        private_file::write(&path.join("CLAUDE.md"),b"My own bear instructions.").unwrap();
        prepare(&home).unwrap();
        assert_eq!(std::fs::read_to_string(path.join("CLAUDE.md")).unwrap(),"My own bear instructions.");
        std::fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn picks_newest_local_unarchived_chat_in_the_folder() {
        let home=PathBuf::from("/tmp/bear-user");
        let old=ThreadSummary{id:"old".into(),cwd:folder(&home).to_string_lossy().into_owned(),last_activity_at:"2026-11-14T12:00:00Z".into(),..Default::default()};
        let new=ThreadSummary{id:"new".into(),cwd:"~/ThreadbearThreads/Bear".into(),last_activity_at:"2026-11-14T13:00:00Z".into(),..old.clone()};
        let archived=ThreadSummary{id:"archived".into(),archived_at:Some("2026-11-14T14:00:00Z".into()),last_activity_at:"2026-11-14T14:00:00Z".into(),..new.clone()};
        let remote=ThreadSummary{id:"remote".into(),runner_id:Some("other".into()),archived_at:None,..archived.clone()};
        let threads=[old,new,archived,remote];
        assert_eq!(newest(&home,threads.iter()).map(|t|t.id.as_str()),Some("new"));
    }
    use std::path::PathBuf;
}
