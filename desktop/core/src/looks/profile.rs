//! The public profile page, `<site>/profile` (docs/protocol.md, "Profile
//! page"), a port of the Swift app's ProfilePage.swift: what the Librarian
//! page's Customize panel lets you put on it. A site is one person's, so
//! there's no handle. The runner keeps it (profile.json) and sends only the
//! parts turned on to your Convex backend; every part starts off.

use super::{github_user_clean, LibrarianBlock, LibrarianLook, Place};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProfilePart {
    Picture,
    Background,
    Activity,
    Github,
    Numbers,
    Projects,
    MemberSince,
    Changelog,
}

impl ProfilePart {
    pub const ALL: [ProfilePart; 8] = [
        ProfilePart::Picture,
        ProfilePart::Background,
        ProfilePart::Activity,
        ProfilePart::Github,
        ProfilePart::Numbers,
        ProfilePart::Projects,
        ProfilePart::MemberSince,
        ProfilePart::Changelog,
    ];

    pub fn raw(self) -> &'static str {
        match self {
            ProfilePart::Picture => "picture",
            ProfilePart::Background => "background",
            ProfilePart::Activity => "activity",
            ProfilePart::Github => "github",
            ProfilePart::Numbers => "numbers",
            ProfilePart::Projects => "projects",
            ProfilePart::MemberSince => "memberSince",
            ProfilePart::Changelog => "changelog",
        }
    }

    pub fn from_raw(raw: &str) -> Option<ProfilePart> {
        ProfilePart::ALL.into_iter().find(|p| p.raw() == raw)
    }

    pub fn title(self) -> &'static str {
        match self {
            ProfilePart::Picture => "Picture",
            ProfilePart::Background => "Background photo",
            ProfilePart::Activity => "Threadbear activity",
            ProfilePart::Github => "GitHub activity",
            ProfilePart::Numbers => "Numbers",
            ProfilePart::Projects => "Projects",
            ProfilePart::MemberSince => "Using Threadbear since",
            ProfilePart::Changelog => "Changelog",
        }
    }

    pub fn tip(self) -> &'static str {
        match self {
            ProfilePart::Picture => "The Librarian page's picture: the bear or your own",
            ProfilePart::Background => "The Librarian page's background photo, behind the top of your page",
            ProfilePart::Activity => "A year of days, shaded by how many threads you started each day. Threads in a private folder aren't counted",
            ProfilePart::Github => "A year of your public GitHub contributions, for the user name in GitHub activity above",
            ProfilePart::Numbers => "The numbers you pick below. Counts only, never what a thread said",
            ProfilePart::Projects => "The projects you pick below: title and thread count only, never goals, state, notes or folders",
            ProfilePart::MemberSince => "The month of your first thread",
            ProfilePart::Changelog => "Your public commits to your Threadbear repo on GitHub, newest first, in a section at the bottom that opens when clicked. The visitor's browser reads them from GitHub, so only public commits ever show",
        }
    }
}

/// The parts Customize doesn't offer. The background photo is held back for
/// now (Wayne, 8 Nov): the runner and site still know it, so putting it back
/// is taking it off here.
pub const HELD_BACK: &[ProfilePart] = &[ProfilePart::Background];

/// The parts Customize offers.
pub fn offered() -> Vec<ProfilePart> {
    ProfilePart::ALL.into_iter().filter(|p| !HELD_BACK.contains(p)).collect()
}

/// The page's numbers, as the runner works them out.
pub const NUMBER_CHOICES: [(&str, &str); 5] = [
    ("threads", "Threads this year"),
    ("activeDays", "Active days"),
    ("streak", "Longest streak"),
    ("projects", "Projects"),
    ("models", "Models used"),
];

pub const LINKS_MAX: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ProfileLink {
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfilePageSettings {
    pub published: bool,
    /// Visitors to the site's front page see it too. Only while published.
    pub home: bool,
    pub name: String,
    pub bio: String,
    pub links: Vec<ProfileLink>,
    pub show: Vec<ProfilePart>,
    pub numbers: Vec<String>,
    pub project_ids: Vec<String>,
    pub order: Vec<String>,
    pub picture_path: Option<String>,
    pub background_path: Option<String>,
    pub background_dim: Option<f64>,
    pub github_user: Option<String>,
    /// The changelog's GitHub repository, owner/name. None means
    /// `<GitHub user>/threadbear`, a fork's usual name (`changelog_repo_for`).
    pub changelog_repo: Option<String>,
}

impl Default for ProfilePageSettings {
    fn default() -> Self {
        ProfilePageSettings {
            published: false,
            home: false,
            name: String::new(),
            bio: String::new(),
            links: Vec::new(),
            show: Vec::new(),
            numbers: vec!["threads".into(), "activeDays".into(), "streak".into()],
            project_ids: Vec::new(),
            order: Vec::new(),
            picture_path: None,
            background_path: None,
            background_dim: None,
            github_user: None,
            changelog_repo: None,
        }
    }
}

fn strings(v: &Value) -> Option<Vec<String>> {
    v.as_array()?.iter().map(|s| s.as_str().map(str::to_owned)).collect()
}

impl ProfilePageSettings {
    pub fn shows(&self, part: ProfilePart) -> bool {
        self.show.contains(&part)
    }

    pub fn set_shown(&mut self, on: bool, part: ProfilePart) {
        if on {
            if !self.show.contains(&part) {
                self.show.push(part);
            }
        } else {
            self.show.retain(|p| *p != part);
        }
    }

    /// The repository the changelog lists, for this GitHub user: the one
    /// typed, or their fork's usual name. None when there's no user or
    /// what's typed isn't owner/name.
    pub fn changelog_repo_for(&self, github_user: &str) -> Option<String> {
        let typed = self.changelog_repo.as_deref().unwrap_or_default();
        let clean = clean_repo(typed);
        if !clean.is_empty() {
            return Some(clean);
        }
        if !typed.trim().is_empty() {
            return None;
        }
        let user = github_user_clean(github_user);
        (!user.is_empty()).then(|| format!("{user}/threadbear"))
    }

    /// What's sent: links still being typed (no good address yet) wait, and
    /// the front page needs the page published. A part held back is never
    /// sent, even if an older build turned it on.
    pub fn sendable(&self) -> ProfilePageSettings {
        let mut s = self.clone();
        s.links.retain(|l| link_problem(&l.url).is_none());
        s.home = s.published && s.home;
        s.show.retain(|p| !HELD_BACK.contains(p));
        if HELD_BACK.contains(&ProfilePart::Background) {
            s.background_path = None;
            s.background_dim = None;
        }
        s
    }

    /// Lenient per field, as the Swift decoder: a `handle` from before 9 Nov
    /// is ignored (a site is one person's), and a part from a newer build is
    /// dropped, not fatal.
    pub fn decode(v: &Value) -> ProfilePageSettings {
        let string = |key: &str| v[key].as_str().unwrap_or_default().to_owned();
        let optional = |key: &str| v[key].as_str().map(str::to_owned);
        let links = v["links"]
            .as_array()
            .and_then(|list| list.iter().map(|l| Some(ProfileLink { title: l["title"].as_str()?.to_owned(), url: l["url"].as_str()?.to_owned() })).collect::<Option<Vec<_>>>())
            .unwrap_or_default();
        ProfilePageSettings {
            published: v["published"].as_bool().unwrap_or(false),
            home: v["home"].as_bool().unwrap_or(false),
            name: string("name"),
            bio: string("bio"),
            links,
            show: strings(&v["show"]).unwrap_or_default().iter().filter_map(|s| ProfilePart::from_raw(s)).collect(),
            numbers: strings(&v["numbers"]).unwrap_or_default(),
            project_ids: strings(&v["projectIds"]).unwrap_or_default(),
            order: strings(&v["order"]).unwrap_or_default(),
            picture_path: optional("picturePath"),
            background_path: optional("backgroundPath"),
            background_dim: v["backgroundDim"].as_f64(),
            github_user: optional("githubUser"),
            changelog_repo: optional("changelogRepo"),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut out = Map::new();
        out.insert("published".into(), json!(self.published));
        out.insert("home".into(), json!(self.home));
        out.insert("name".into(), json!(self.name));
        out.insert("bio".into(), json!(self.bio));
        out.insert("links".into(), Value::Array(self.links.iter().map(|l| json!({ "title": l.title, "url": l.url })).collect()));
        out.insert("show".into(), json!(self.show.iter().map(|p| p.raw()).collect::<Vec<_>>()));
        out.insert("numbers".into(), json!(self.numbers));
        out.insert("projectIds".into(), json!(self.project_ids));
        out.insert("order".into(), json!(self.order));
        let mut put = |key: &str, value: Option<Value>| {
            if let Some(value) = value {
                out.insert(key.into(), value);
            }
        };
        put("picturePath", self.picture_path.as_ref().map(|s| json!(s)));
        put("backgroundPath", self.background_path.as_ref().map(|s| json!(s)));
        put("backgroundDim", self.background_dim.map(|d| json!(d)));
        put("githubUser", self.github_user.as_ref().map(|s| json!(s)));
        put("changelogRepo", self.changelog_repo.as_ref().map(|s| json!(s)));
        Value::Object(out)
    }
}

/// owner/name as typed or pasted (a github.com link, a trailing .git), or
/// empty when it isn't one.
pub fn clean_repo(typed: &str) -> String {
    let mut s = typed.trim().to_owned();
    if let Some(at) = s.to_ascii_lowercase().find("github.com/") {
        s = s[at + "github.com/".len()..].to_owned();
    }
    if let Some(rest) = s.strip_suffix(".git") {
        s = rest.to_owned();
    }
    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).take(2).collect();
    let ok = parts.len() == 2
        && github_user_clean(parts[0]) == parts[0]
        && parts[1] != "."
        && parts[1] != ".."
        && parts[1].chars().count() <= 100
        && parts[1].chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    if ok {
        format!("{}/{}", parts[0], parts[1])
    } else {
        String::new()
    }
}

/// Why a link won't do, or None: it needs to be an http(s) address with a host.
pub fn link_problem(url: &str) -> Option<&'static str> {
    match split_url(url.trim()) {
        Some((scheme, authority, _)) if (scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http")) && !host_of(authority).is_empty() => None,
        _ => Some("Starts with https://"),
    }
}

/// The web page's cards top to bottom, from where the Librarian page puts
/// its parts: the numbers, Threadbear activity (just under the numbers
/// unless moved on its own), GitHub activity where it sits, and projects at
/// the bottom.
pub fn card_order(look: &LibrarianLook) -> Vec<String> {
    let rank = |p: Place| Place::ALL.iter().position(|q| *q == p).unwrap_or(0);
    let mut cards = [
        ("numbers", look.place(LibrarianBlock::Numbers), 0),
        ("activity", look.place(LibrarianBlock::Activity), 1),
        ("github", look.place(LibrarianBlock::Github), 2),
        ("projects", Place::Bottom, 3),
    ];
    cards.sort_by_key(|(_, place, n)| (rank(*place), *n));
    cards.iter().map(|(name, _, _)| (*name).to_owned()).collect()
}

/// What `profilePage.get` and `profilePage.update` answer.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ProfilePageStatus {
    pub profile: Option<ProfilePageSettings>,
    /// The page's address, once it's saved and there's a backend: /profile,
    /// or the site's front page when it's shown there.
    pub url: Option<String>,
    pub sent_at: Option<String>,
    /// Why the last send didn't go, said by the runner or the backend.
    pub problem: Option<String>,
}

impl ProfilePageStatus {
    pub fn decode(v: &Value) -> ProfilePageStatus {
        ProfilePageStatus {
            profile: v["profile"].is_object().then(|| ProfilePageSettings::decode(&v["profile"])),
            url: v["url"].as_str().map(str::to_owned),
            sent_at: v["sentAt"].as_str().map(str::to_owned),
            problem: v["problem"].as_str().map(str::to_owned),
        }
    }

    /// The profile page itself, always <site>/profile. The runner gives the
    /// site's front page instead while "Show it on my site's front page" is
    /// on, and signed in that page is your threads, not the profile.
    pub fn profile_url(&self) -> Option<String> {
        let (scheme, authority, _) = split_url(self.url.as_deref()?)?;
        (!host_of(authority).is_empty()).then(|| format!("{scheme}://{authority}/profile"))
    }

    /// The site's front page, only while the profile is shown there.
    pub fn site_url(&self) -> Option<String> {
        let (scheme, authority, rest) = split_url(self.url.as_deref()?)?;
        if host_of(authority).is_empty() {
            return None;
        }
        let path_end = rest.find(['?', '#']).unwrap_or(rest.len());
        let (path, tail) = rest.split_at(path_end);
        (path.is_empty() || path == "/").then(|| format!("{scheme}://{authority}/{tail}"))
    }
}

/// A URL's scheme, authority and the rest (path, query, fragment), or None
/// when it has no `scheme://`.
fn split_url(url: &str) -> Option<(&str, &str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() || !scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c)) {
        return None;
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    Some((scheme, &rest[..end], &rest[end..]))
}

/// The host in an authority: without the user and the port.
fn host_of(authority: &str) -> &str {
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    host.split(':').next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(title: &str, url: &str) -> ProfileLink {
        ProfileLink { title: title.into(), url: url.into() }
    }

    #[test]
    fn what_is_sent() {
        let mut s = ProfilePageSettings { links: vec![link("GitHub", "https://github.com/waynesutton"), link("Half typed", "https://")], home: true, ..Default::default() };
        let sent = s.sendable();
        assert_eq!(sent.links.iter().map(|l| l.title.as_str()).collect::<Vec<_>>(), ["GitHub"], "a link still being typed waits");
        assert!(!sent.home, "the front page needs it published");
        s.published = true;
        assert!(s.sendable().home);
    }

    #[test]
    fn links() {
        assert_eq!(link_problem("https://github.com/waynesutton"), None);
        assert!(link_problem("javascript:alert(1)").is_some());
        assert!(link_problem("github.com").is_some());
    }

    #[test]
    fn cards_follow_the_librarian_page() {
        let mut look = LibrarianLook::default();
        assert_eq!(card_order(&look), ["numbers", "activity", "github", "projects"]);
        look.set_place(Place::Top, LibrarianBlock::Github);
        assert_eq!(card_order(&look), ["github", "numbers", "activity", "projects"]);
        look.set_place(Place::Bottom, LibrarianBlock::Numbers);
        assert_eq!(card_order(&look), ["github", "numbers", "activity", "projects"]);
        look.set_place(Place::Bottom, LibrarianBlock::Github);
        look.set_place(Place::Top, LibrarianBlock::Numbers);
        assert_eq!(card_order(&look), ["numbers", "activity", "github", "projects"]);
    }

    #[test]
    fn parts_start_off_and_toggle() {
        let mut s = ProfilePageSettings::default();
        assert!(s.show.is_empty() && !s.published);
        s.set_shown(true, ProfilePart::Activity);
        s.set_shown(true, ProfilePart::Activity);
        assert_eq!(s.show, [ProfilePart::Activity]);
        s.set_shown(false, ProfilePart::Activity);
        assert!(s.show.is_empty());
    }

    #[test]
    fn changelog_repository() {
        let mut s = ProfilePageSettings::default();
        assert_eq!(s.changelog_repo_for("waynesutton").as_deref(), Some("waynesutton/threadbear"), "nothing typed: the fork's usual name");
        assert_eq!(s.changelog_repo_for(""), None, "no GitHub user, no changelog");
        s.changelog_repo = Some("https://github.com/mikecann/threadbear.git".into());
        assert_eq!(s.changelog_repo_for("waynesutton").as_deref(), Some("mikecann/threadbear"));
        s.changelog_repo = Some("half typed".into());
        assert_eq!(s.changelog_repo_for("waynesutton"), None);
        assert_eq!(clean_repo("waynesutton/.."), "");
        assert_eq!(s.sendable().changelog_repo.as_deref(), Some("half typed"), "sent as typed; the runner reads it");
        s.set_shown(true, ProfilePart::Changelog);
        let back = ProfilePageSettings::decode(&s.to_value());
        assert!(back.shows(ProfilePart::Changelog) && back.changelog_repo.as_deref() == Some("half typed"));
        assert!(offered().contains(&ProfilePart::Changelog));
    }

    #[test]
    fn decodes_what_other_builds_saved() {
        // As saved before 9 Nov, with a handle and no home.
        let v: Value = serde_json::from_str(r#"{"published":true,"handle":"wayne","show":["activity","fromTheFuture"],"links":[{"title":"X","url":"https://x.com/waynesutton"}],"extra":1}"#).unwrap();
        let s = ProfilePageSettings::decode(&v);
        assert!(s.published && !s.home);
        assert!(s.to_value().get("handle").is_none());
        assert_eq!(s.show, [ProfilePart::Activity]);
        assert!(s.links.len() == 1 && s.name.is_empty());
        assert_eq!(ProfilePageSettings::decode(&s.to_value()), s);
        let status = ProfilePageStatus::decode(&serde_json::from_str(r#"{"profile":null,"url":"https://a.convex.site/profile"}"#).unwrap());
        assert!(status.profile.is_none() && status.url.as_deref() == Some("https://a.convex.site/profile"));
    }

    #[test]
    fn background_photo_is_held_back() {
        assert!(!offered().contains(&ProfilePart::Background));
        let settings = ProfilePageSettings { show: vec![ProfilePart::Picture, ProfilePart::Background], background_path: Some("/tmp/b.jpg".into()), background_dim: Some(0.4), ..Default::default() };
        let sent = settings.sendable();
        assert_eq!(sent.show, [ProfilePart::Picture], "an older build's switch doesn't send it");
        assert_eq!(sent.background_path, None);
        assert_eq!(sent.background_dim, None);
    }

    #[test]
    fn profile_opens_the_profile_even_when_it_is_the_front_page() {
        let at = |url: Option<&str>| ProfilePageStatus { url: url.map(str::to_owned), ..Default::default() };
        let home = at(Some("https://wayne.convex.site/"));
        assert_eq!(home.profile_url().as_deref(), Some("https://wayne.convex.site/profile"));
        assert_eq!(home.site_url().as_deref(), Some("https://wayne.convex.site/"));
        let plain = at(Some("https://wayne.convex.site/profile"));
        assert_eq!(plain.profile_url().as_deref(), Some("https://wayne.convex.site/profile"));
        assert_eq!(plain.site_url(), None, "no Open Site unless it's on the front page");
        assert_eq!(at(None).profile_url(), None);
        assert_eq!(at(Some("not a link")).profile_url(), None);
    }
}
