//! Web addresses in inline code, distinct from file paths and commands.
pub const ENDINGS: &[&str]=&["com","org","net","dev","app","io","ai","co","run","site","so","me","xyz","tech","cloud","page","pages","link","gg","fm","tv","news","blog","us","uk","de","fr","ca","au"];
pub fn url(code:&str)->Option<String> {
    let text=code.trim();
    if text.is_empty() || text.chars().count()>=500 || text.chars().any(|c|c.is_whitespace()||matches!(c,'`'|'<'|'>'|'"')) {return None;}
    let lower=text.to_ascii_lowercase();
    if lower.starts_with("https://")||lower.starts_with("http://") {
        let authority=text.split_once("://")?.1.split('/').next()?;
        return (!authority.is_empty()).then(||text.to_owned());
    }
    let host_port=text.split('/').next()?;
    let host=if let Some((host,port))=host_port.rsplit_once(':') {
        if port.is_empty()||!port.bytes().all(|b|b.is_ascii_digit()) {return None;}
        host
    } else {host_port};
    let labels:Vec<&str>=host.split('.').collect();
    if labels.len()<2 || !ENDINGS.contains(&labels.last()?.to_ascii_lowercase().as_str()) {return None;}
    if labels.iter().any(|label|label.is_empty()||!label.as_bytes()[0].is_ascii_alphanumeric()||!label.as_bytes()[label.len()-1].is_ascii_alphanumeric()||!label.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-')) {return None;}
    Some(format!("https://{text}"))
}
#[cfg(test)]
mod tests {
    use super::url;
    #[test]
    fn web_addresses_in_code() {
        for (text,wanted) in [("github.com/waynesutton/threadbear","https://github.com/waynesutton/threadbear"),("https://docs.convex.dev/home","https://docs.convex.dev/home"),("threadbear.mikecann.app","https://threadbear.mikecann.app"),("http://localhost:5173","http://localhost:5173"),("www.example.co.uk/path","https://www.example.co.uk/path")] {assert_eq!(url(text).as_deref(),Some(wanted));}
    }
    #[test]
    fn files_commands_and_unsafe_destinations_stay_code() {
        for text in ["deploy.sh","readme.md","package.json","fd68ea4","npm run build","foo.bar","x.ts","localhost:5173","obj.method()","javascript:alert(1)","https://x.y/<script>","example.com:bad","https://x.y/\"bad"] {assert_eq!(url(text),None,"{text}");}
    }
}
