// Pictures of the user's own on the Librarian and Scheduled pages and behind
// the main window (the Swift app's Looks.swift, PageLook.swift,
// ScheduledLook.swift, Quotes.swift and ThreadActivity.swift): what each
// page's Customize panel can change, kept on this Mac and never sent to the
// runner. Every field reads on its own, so a look saved by an older or newer
// build, or brought across from the Swift app, still loads.
use crate::inbox::{parse_ms, ThreadSummary};
use chrono::{Datelike, NaiveDate, TimeZone};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// How much of the window's own colour lies over a background as it ships.
pub const DEFAULT_DIM: f64 = 0.55;
/// The Fade slider's top: past this the photo is all but gone.
pub const MAX_DIM: f64 = 0.9;

/// Where a page's part sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    Top,
    Middle,
    Bottom,
}

impl Place {
    pub const ALL: [Place; 3] = [Place::Top, Place::Middle, Place::Bottom];

    pub fn raw(self) -> &'static str {
        match self {
            Place::Top => "top",
            Place::Middle => "middle",
            Place::Bottom => "bottom",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Place::Top => "Top",
            Place::Middle => "Middle",
            Place::Bottom => "Bottom",
        }
    }

    pub fn from_raw(raw: &str) -> Option<Place> {
        Place::ALL.into_iter().find(|p| p.raw() == raw)
    }
}

/// The page's picture: its own bear, one of yours, or none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Picture {
    #[default]
    Bear,
    Own,
    None,
}

impl Picture {
    pub const ALL: [Picture; 3] = [Picture::Bear, Picture::Own, Picture::None];

    pub fn raw(self) -> &'static str {
        match self {
            Picture::Bear => "bear",
            Picture::Own => "own",
            Picture::None => "none",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Picture::Bear => "The bear",
            Picture::Own => "Your picture",
            Picture::None => "None",
        }
    }

    fn from_raw(raw: &str) -> Option<Picture> {
        Picture::ALL.into_iter().find(|p| p.raw() == raw)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PictureSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl PictureSize {
    pub const ALL: [PictureSize; 3] = [PictureSize::Small, PictureSize::Medium, PictureSize::Large];

    pub fn raw(self) -> &'static str {
        match self {
            PictureSize::Small => "small",
            PictureSize::Medium => "medium",
            PictureSize::Large => "large",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            PictureSize::Small => "Small",
            PictureSize::Medium => "Medium",
            PictureSize::Large => "Large",
        }
    }

    /// The picture's largest side, before zoom.
    pub fn side(self) -> f32 {
        match self {
            PictureSize::Small => 160.,
            PictureSize::Medium => 280.,
            PictureSize::Large => 400.,
        }
    }

    fn from_raw(raw: &str) -> Option<PictureSize> {
        PictureSize::ALL.into_iter().find(|p| p.raw() == raw)
    }
}

/// Where a background photo sits when it fills a space of another shape and
/// is cropped: 0...1 across and down. (0.5, 1) keeps the photo's bottom edge
/// on the space's bottom edge, centred.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropFocus {
    pub x: f64,
    pub y: f64,
}

impl Default for BackdropFocus {
    fn default() -> Self {
        BackdropFocus::BOTTOM
    }
}

fn clamp_unit(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0., 1.) } else { 0.5 }
}

impl BackdropFocus {
    pub const BOTTOM: BackdropFocus = BackdropFocus { x: 0.5, y: 1. };

    pub fn new(x: f64, y: f64) -> Self {
        BackdropFocus { x: clamp_unit(x), y: clamp_unit(y) }
    }

    fn read(v: &Value) -> Self {
        BackdropFocus::new(v["x"].as_f64().unwrap_or(0.5), v["y"].as_f64().unwrap_or(1.))
    }

    fn write(self) -> Value {
        json!({ "x": self.x, "y": self.y })
    }

    /// The photo's frame (x, y, width, height), scaled to fill `space` and
    /// shifted so the focus lines up: 0 puts the photo's left (or top) edge on
    /// the space's, 1 its right (or bottom) edge. Along the side that already
    /// fits, the photo can't move.
    pub fn frame(self, image: (f64, f64), space: (f64, f64)) -> (f64, f64, f64, f64) {
        let ((iw, ih), (sw, sh)) = (image, space);
        if iw <= 0. || ih <= 0. || sw <= 0. || sh <= 0. {
            return (0., 0., sw, sh);
        }
        let scale = f64::max(sw / iw, sh / ih);
        let (w, h) = (iw * scale, ih * scale);
        ((sw - w) * self.x, (sh - h) * self.y, w, h)
    }

    /// The focus after dragging the photo by `by` (in the space's points)
    /// from `self`: the photo follows the pointer and stops at its edges.
    pub fn dragged(self, by: (f64, f64), image: (f64, f64), space: (f64, f64)) -> Self {
        let (_, _, w, h) = self.frame(image, space);
        let (spare_x, spare_y) = (w - space.0, h - space.1);
        BackdropFocus::new(if spare_x > 0.5 { self.x - by.0 / spare_x } else { self.x }, if spare_y > 0.5 { self.y - by.1 / spare_y } else { self.y })
    }
}

/// What every customizable page's picture and background are made of, so
/// the Customize panels' Picture and Background sections work on any page.
pub trait PictureLook {
    fn picture(&self) -> Picture;
    fn set_picture(&mut self, picture: Picture);
    fn picture_file(&self) -> Option<&str>;
    fn set_picture_file(&mut self, file: Option<String>);
    fn picture_size(&self) -> PictureSize;
    fn set_picture_size(&mut self, size: PictureSize);
    fn background(&self) -> Option<&str>;
    fn set_background(&mut self, file: Option<String>);
    fn background_dim(&self) -> f64;
    fn set_background_dim(&mut self, dim: f64);
    fn background_focus(&self) -> BackdropFocus;
    fn set_background_focus(&mut self, focus: BackdropFocus);
    /// Puts the picture back on the page after it was hidden, as choosing a
    /// new one should.
    fn show_picture(&mut self);
}

/// The pictures a look has in the looks folder.
pub fn files(look: &impl PictureLook) -> Vec<String> {
    [look.picture_file(), look.background()].into_iter().flatten().map(str::to_owned).collect()
}

/// The pictures `old` had that `new` no longer uses, to delete. Every copy
/// gets a new name, so one page never deletes another's.
pub fn unused(old: &impl PictureLook, new: &impl PictureLook) -> Vec<String> {
    let keep = files(new);
    files(old).into_iter().filter(|f| !keep.contains(f)).collect()
}

/// A name of a picture in the looks folder, never a path.
pub fn safe_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('/') && !name.contains('\\') && name != "." && name != ".."
}

macro_rules! picture_look {
    ($t:ty, $b:ident) => {
        impl PictureLook for $t {
            fn picture(&self) -> Picture {
                self.picture
            }
            fn set_picture(&mut self, picture: Picture) {
                self.picture = picture;
            }
            fn picture_file(&self) -> Option<&str> {
                self.picture_file.as_deref()
            }
            fn set_picture_file(&mut self, file: Option<String>) {
                self.picture_file = file;
            }
            fn picture_size(&self) -> PictureSize {
                self.picture_size
            }
            fn set_picture_size(&mut self, size: PictureSize) {
                self.picture_size = size;
            }
            fn background(&self) -> Option<&str> {
                self.background.as_deref()
            }
            fn set_background(&mut self, file: Option<String>) {
                self.background = file;
            }
            fn background_dim(&self) -> f64 {
                self.background_dim
            }
            fn set_background_dim(&mut self, dim: f64) {
                self.background_dim = dim.clamp(0., 1.);
            }
            fn background_focus(&self) -> BackdropFocus {
                self.background_focus
            }
            fn set_background_focus(&mut self, focus: BackdropFocus) {
                self.background_focus = focus;
            }
            fn show_picture(&mut self) {
                self.hidden.remove(&$b::Picture);
            }
        }
    };
}

/// The fields every look has, read from what was saved.
struct Common {
    picture: Picture,
    picture_file: Option<String>,
    picture_size: PictureSize,
    background: Option<String>,
    background_dim: f64,
    background_focus: BackdropFocus,
    hidden_numbers: BTreeSet<String>,
}

fn file_name(v: &Value) -> Option<String> {
    v.as_str().filter(|s| safe_name(s)).map(str::to_owned)
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().into_iter().flatten().filter_map(|s| s.as_str().map(str::to_owned)).collect()
}

fn common(v: &Value) -> Common {
    Common {
        picture: v["picture"].as_str().and_then(Picture::from_raw).unwrap_or_default(),
        picture_file: file_name(&v["pictureFile"]),
        picture_size: v["pictureSize"].as_str().and_then(PictureSize::from_raw).unwrap_or_default(),
        background: file_name(&v["background"]),
        background_dim: v["backgroundDim"].as_f64().filter(|d| d.is_finite()).unwrap_or(DEFAULT_DIM).clamp(0., 1.),
        background_focus: if v["backgroundFocus"].is_object() { BackdropFocus::read(&v["backgroundFocus"]) } else { BackdropFocus::BOTTOM },
        hidden_numbers: strings(&v["hiddenNumbers"]).into_iter().collect(),
    }
}

fn write_common(out: &mut Map<String, Value>, look: &impl PictureLook, hidden_numbers: &BTreeSet<String>) {
    out.insert("picture".into(), json!(look.picture().raw()));
    if let Some(f) = look.picture_file() {
        out.insert("pictureFile".into(), json!(f));
    }
    out.insert("pictureSize".into(), json!(look.picture_size().raw()));
    if let Some(f) = look.background() {
        out.insert("background".into(), json!(f));
    }
    out.insert("backgroundDim".into(), json!(look.background_dim()));
    out.insert("backgroundFocus".into(), look.background_focus().write());
    out.insert("hiddenNumbers".into(), json!(hidden_numbers.iter().collect::<Vec<_>>()));
}

/// The parts of the Librarian page, in the order they stack within a place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LibrarianBlock {
    Greeting,
    Picture,
    About,
    Quote,
    Numbers,
    Activity,
    Github,
    Settings,
}

impl LibrarianBlock {
    pub const ALL: [LibrarianBlock; 8] = [
        LibrarianBlock::Greeting,
        LibrarianBlock::Picture,
        LibrarianBlock::About,
        LibrarianBlock::Quote,
        LibrarianBlock::Numbers,
        LibrarianBlock::Activity,
        LibrarianBlock::Github,
        LibrarianBlock::Settings,
    ];

    pub fn raw(self) -> &'static str {
        match self {
            LibrarianBlock::Greeting => "greeting",
            LibrarianBlock::Picture => "picture",
            LibrarianBlock::About => "about",
            LibrarianBlock::Quote => "quote",
            LibrarianBlock::Numbers => "numbers",
            LibrarianBlock::Activity => "activity",
            LibrarianBlock::Github => "github",
            LibrarianBlock::Settings => "settings",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            LibrarianBlock::Greeting => "Greeting",
            LibrarianBlock::Picture => "Picture",
            LibrarianBlock::About => "Title and status",
            LibrarianBlock::Quote => "Quote of the day",
            LibrarianBlock::Numbers => "Numbers",
            // Short enough for its row in Layout; the card says Threadbear activity.
            LibrarianBlock::Activity => "Thread activity",
            LibrarianBlock::Github => "GitHub activity",
            LibrarianBlock::Settings => "Settings",
        }
    }

    fn from_raw(raw: &str) -> Option<LibrarianBlock> {
        LibrarianBlock::ALL.into_iter().find(|b| b.raw() == raw)
    }

    /// Where it sits when it hasn't been moved: as the page was before it
    /// could be customized, with GitHub under the numbers.
    pub fn default_place(self) -> Place {
        match self {
            LibrarianBlock::Greeting | LibrarianBlock::Picture | LibrarianBlock::About | LibrarianBlock::Quote => Place::Top,
            LibrarianBlock::Numbers | LibrarianBlock::Activity | LibrarianBlock::Github => Place::Middle,
            LibrarianBlock::Settings => Place::Bottom,
        }
    }
}

/// The parts of the Librarian's title block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AboutPart {
    Title,
    Line,
    Status,
}

impl AboutPart {
    pub const ALL: [AboutPart; 3] = [AboutPart::Title, AboutPart::Line, AboutPart::Status];

    pub fn raw(self) -> &'static str {
        match self {
            AboutPart::Title => "title",
            AboutPart::Line => "line",
            AboutPart::Status => "status",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            AboutPart::Title => "Title",
            AboutPart::Line => "Description",
            AboutPart::Status => "Status",
        }
    }

    fn from_raw(raw: &str) -> Option<AboutPart> {
        AboutPart::ALL.into_iter().find(|b| b.raw() == raw)
    }
}

/// How the Librarian's page is laid out when Settings > General > Look has
/// Customize the Librarian page on (`LibrarianLook`).
#[derive(Clone, Debug, PartialEq)]
pub struct LibrarianLook {
    pub picture: Picture,
    pub picture_file: Option<String>,
    pub picture_size: PictureSize,
    pub background: Option<String>,
    pub background_dim: f64,
    pub background_focus: BackdropFocus,
    pub places: BTreeMap<LibrarianBlock, Place>,
    pub hidden: BTreeSet<LibrarianBlock>,
    /// The numbers (by `LIBRARIAN_NUMBERS`' key) taken off the page.
    pub hidden_numbers: BTreeSet<String>,
    /// Whose public GitHub contributions the GitHub block shows.
    pub github_user: String,
    /// Off until turned on, so a look saved before it existed doesn't grow one.
    pub greeting_shown: bool,
    pub greeting_name: String,
    pub greeting_clock: bool,
    pub hidden_about: BTreeSet<AboutPart>,
    pub quote_shown: bool,
    /// The day the refresh button was last used, and how many times.
    pub quote_day: String,
    pub quote_skips: u32,
    pub activity_shown: bool,
    /// The Swift app's View profile page button, kept as it was saved: this
    /// app has no profile page yet.
    pub profile_button: bool,
}

impl Default for LibrarianLook {
    fn default() -> Self {
        LibrarianLook {
            picture: Picture::Bear,
            picture_file: None,
            picture_size: PictureSize::Medium,
            background: None,
            background_dim: DEFAULT_DIM,
            background_focus: BackdropFocus::BOTTOM,
            places: BTreeMap::new(),
            hidden: BTreeSet::new(),
            hidden_numbers: BTreeSet::new(),
            github_user: String::new(),
            greeting_shown: false,
            greeting_name: String::new(),
            greeting_clock: true,
            hidden_about: BTreeSet::new(),
            quote_shown: false,
            quote_day: String::new(),
            quote_skips: 0,
            activity_shown: false,
            profile_button: true,
        }
    }
}

picture_look!(LibrarianLook, LibrarianBlock);

/// The Librarian page's numbers, by key and title, in the order its tiles
/// come (`LibrarianStats.names`).
pub const LIBRARIAN_NUMBERS: [(&str, &str); 6] = [("looks", "Looks today"), ("skipped", "Skipped today"), ("tokens", "Tokens today"), ("made", "Projects it made"), ("filed", "Threads it filed"), ("waiting", "Waiting")];

/// The Scheduled page's numbers (`ScheduledStats.names`).
pub const SCHEDULED_NUMBERS: [(&str, &str); 6] = [("on", "On"), ("dueSoon", "Due in a day"), ("repeating", "Repeating"), ("byThreads", "Set by threads"), ("paused", "Paused"), ("runs", "Runs so far")];

impl LibrarianLook {
    pub fn is_standard(&self) -> bool {
        *self == LibrarianLook::default()
    }

    /// Threadbear activity goes with the numbers until it's moved on its own.
    pub fn place(&self, block: LibrarianBlock) -> Place {
        if let Some(place) = self.places.get(&block) {
            return *place;
        }
        if block == LibrarianBlock::Activity { self.place(LibrarianBlock::Numbers) } else { block.default_place() }
    }

    pub fn set_place(&mut self, place: Place, block: LibrarianBlock) {
        if block != LibrarianBlock::Activity && place == block.default_place() {
            self.places.remove(&block);
        } else {
            self.places.insert(block, place);
        }
    }

    /// Whether the Layout switch for a part is on. Greeting, quote and
    /// activity have flags of their own, as they start off.
    pub fn switched_on(&self, block: LibrarianBlock) -> bool {
        match block {
            LibrarianBlock::Greeting => self.greeting_shown,
            LibrarianBlock::Quote => self.quote_shown,
            LibrarianBlock::Activity => self.activity_shown,
            _ => !self.hidden.contains(&block),
        }
    }

    /// Shown, as the user set it. GitHub also needs a user name.
    pub fn shows(&self, block: LibrarianBlock) -> bool {
        match block {
            LibrarianBlock::Greeting | LibrarianBlock::Quote | LibrarianBlock::Activity => self.switched_on(block),
            _ if self.hidden.contains(&block) => false,
            LibrarianBlock::About => self.hidden_about.len() < AboutPart::ALL.len(),
            LibrarianBlock::Github => !github_user(&self.github_user).is_empty(),
            LibrarianBlock::Picture => self.picture != Picture::None && (self.picture != Picture::Own || self.picture_file.is_some()),
            _ => true,
        }
    }

    pub fn set_shown(&mut self, shown: bool, block: LibrarianBlock) {
        match block {
            LibrarianBlock::Greeting => self.greeting_shown = shown,
            LibrarianBlock::Quote => self.quote_shown = shown,
            LibrarianBlock::Activity => self.activity_shown = shown,
            _ if shown => {
                self.hidden.remove(&block);
            }
            _ => {
                self.hidden.insert(block);
            }
        }
    }

    pub fn shows_part(&self, part: AboutPart) -> bool {
        !self.hidden_about.contains(&part)
    }

    pub fn set_part_shown(&mut self, shown: bool, part: AboutPart) {
        if shown {
            self.hidden_about.remove(&part);
        } else {
            self.hidden_about.insert(part);
        }
    }

    /// The parts to draw, top to bottom: by place, then in block order.
    pub fn layout(&self) -> Vec<LibrarianBlock> {
        Place::ALL.into_iter().flat_map(|place| LibrarianBlock::ALL.into_iter().filter(move |b| self.place(*b) == place && self.shows(*b))).collect()
    }

    /// How far today's quote has been moved on.
    pub fn quote_skips_on(&self, day: &str) -> u32 {
        if self.quote_day == day { self.quote_skips } else { 0 }
    }

    /// The refresh button: another quote for the rest of today.
    pub fn skip_quote(&mut self, day: &str) {
        self.quote_skips = self.quote_skips_on(day) + 1;
        self.quote_day = day.to_owned();
    }

    /// The saved look, or the standard one when nothing readable is saved.
    pub fn load(v: Option<&Value>) -> LibrarianLook {
        let Some(v) = v.filter(|v| v.is_object()) else { return LibrarianLook::default() };
        let c = common(v);
        let mut places = BTreeMap::new();
        // Unknown parts and places from another build are dropped, not fatal.
        for (k, p) in v["places"].as_object().into_iter().flatten() {
            if let (Some(b), Some(p)) = (LibrarianBlock::from_raw(k), p.as_str().and_then(Place::from_raw)) {
                places.insert(b, p);
            }
        }
        LibrarianLook {
            picture: c.picture,
            picture_file: c.picture_file,
            picture_size: c.picture_size,
            background: c.background,
            background_dim: c.background_dim,
            background_focus: c.background_focus,
            places,
            hidden: strings(&v["hidden"]).iter().filter_map(|s| LibrarianBlock::from_raw(s)).collect(),
            hidden_numbers: c.hidden_numbers,
            github_user: v["githubUser"].as_str().unwrap_or_default().to_owned(),
            greeting_shown: v["greetingShown"].as_bool().unwrap_or(false),
            greeting_name: v["greetingName"].as_str().unwrap_or_default().to_owned(),
            greeting_clock: v["greetingClock"].as_bool().unwrap_or(true),
            hidden_about: strings(&v["hiddenAbout"]).iter().filter_map(|s| AboutPart::from_raw(s)).collect(),
            quote_shown: v["quoteShown"].as_bool().unwrap_or(false),
            quote_day: v["quoteDay"].as_str().unwrap_or_default().to_owned(),
            quote_skips: v["quoteSkips"].as_u64().map_or(0, |n| n.min(u32::MAX as u64) as u32),
            activity_shown: v["activityShown"].as_bool().unwrap_or(false),
            profile_button: v["profileButton"].as_bool().unwrap_or(true),
        }
    }

    /// In the Swift app's JSON shape, so either app reads the other's.
    pub fn save(&self) -> Value {
        let mut out = Map::new();
        write_common(&mut out, self, &self.hidden_numbers);
        out.insert("places".into(), Value::Object(self.places.iter().map(|(b, p)| (b.raw().to_owned(), json!(p.raw()))).collect()));
        out.insert("hidden".into(), json!(self.hidden.iter().map(|b| b.raw()).collect::<Vec<_>>()));
        out.insert("githubUser".into(), json!(self.github_user));
        out.insert("greetingShown".into(), json!(self.greeting_shown));
        out.insert("greetingName".into(), json!(self.greeting_name));
        out.insert("greetingClock".into(), json!(self.greeting_clock));
        out.insert("quoteShown".into(), json!(self.quote_shown));
        out.insert("quoteDay".into(), json!(self.quote_day));
        out.insert("quoteSkips".into(), json!(self.quote_skips));
        out.insert("activityShown".into(), json!(self.activity_shown));
        out.insert("profileButton".into(), json!(self.profile_button));
        out.insert("hiddenAbout".into(), json!(self.hidden_about.iter().map(|p| p.raw()).collect::<Vec<_>>()));
        Value::Object(out)
    }
}

/// The parts of the Scheduled page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScheduledBlock {
    Picture,
    About,
    Numbers,
}

impl ScheduledBlock {
    pub const ALL: [ScheduledBlock; 3] = [ScheduledBlock::Picture, ScheduledBlock::About, ScheduledBlock::Numbers];

    pub fn raw(self) -> &'static str {
        match self {
            ScheduledBlock::Picture => "picture",
            ScheduledBlock::About => "about",
            ScheduledBlock::Numbers => "numbers",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ScheduledBlock::Picture => "Picture",
            ScheduledBlock::About => "Title and status",
            ScheduledBlock::Numbers => "Numbers",
        }
    }

    fn from_raw(raw: &str) -> Option<ScheduledBlock> {
        ScheduledBlock::ALL.into_iter().find(|b| b.raw() == raw)
    }
}

/// How the Scheduled page is laid out when Customize the Scheduled page is
/// on (`ScheduledLook`): smaller than the Librarian's, as the page is about
/// what runs next. Every part starts at the top, and middle and bottom are
/// real places in the window.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledLook {
    pub picture: Picture,
    pub picture_file: Option<String>,
    pub picture_size: PictureSize,
    pub background: Option<String>,
    pub background_dim: f64,
    pub background_focus: BackdropFocus,
    pub places: BTreeMap<ScheduledBlock, Place>,
    pub hidden: BTreeSet<ScheduledBlock>,
    pub hidden_numbers: BTreeSet<String>,
}

impl Default for ScheduledLook {
    fn default() -> Self {
        ScheduledLook {
            picture: Picture::Bear,
            picture_file: None,
            picture_size: PictureSize::Medium,
            background: None,
            background_dim: DEFAULT_DIM,
            background_focus: BackdropFocus::BOTTOM,
            places: BTreeMap::new(),
            hidden: BTreeSet::new(),
            hidden_numbers: BTreeSet::new(),
        }
    }
}

picture_look!(ScheduledLook, ScheduledBlock);

impl ScheduledLook {
    pub fn is_standard(&self) -> bool {
        *self == ScheduledLook::default()
    }

    pub fn place(&self, block: ScheduledBlock) -> Place {
        self.places.get(&block).copied().unwrap_or(Place::Top)
    }

    /// Moving a part back to the top isn't kept as a change, so the look is
    /// standard again.
    pub fn set_place(&mut self, place: Place, block: ScheduledBlock) {
        if place == Place::Top {
            self.places.remove(&block);
        } else {
            self.places.insert(block, place);
        }
    }

    pub fn shows(&self, block: ScheduledBlock) -> bool {
        if self.hidden.contains(&block) {
            return false;
        }
        block != ScheduledBlock::Picture || (self.picture != Picture::None && (self.picture != Picture::Own || self.picture_file.is_some()))
    }

    pub fn set_shown(&mut self, shown: bool, block: ScheduledBlock) {
        if shown {
            self.hidden.remove(&block);
        } else {
            self.hidden.insert(block);
        }
    }

    pub fn layout(&self) -> Vec<ScheduledBlock> {
        Place::ALL.into_iter().flat_map(|place| ScheduledBlock::ALL.into_iter().filter(move |b| self.place(*b) == place && self.shows(*b))).collect()
    }

    pub fn load(v: Option<&Value>) -> ScheduledLook {
        let Some(v) = v.filter(|v| v.is_object()) else { return ScheduledLook::default() };
        let c = common(v);
        let mut places = BTreeMap::new();
        for (k, p) in v["places"].as_object().into_iter().flatten() {
            if let (Some(b), Some(p)) = (ScheduledBlock::from_raw(k), p.as_str().and_then(Place::from_raw)) {
                places.insert(b, p);
            }
        }
        ScheduledLook {
            picture: c.picture,
            picture_file: c.picture_file,
            picture_size: c.picture_size,
            background: c.background,
            background_dim: c.background_dim,
            background_focus: c.background_focus,
            places,
            hidden: strings(&v["hidden"]).iter().filter_map(|s| ScheduledBlock::from_raw(s)).collect(),
            hidden_numbers: c.hidden_numbers,
        }
    }

    pub fn save(&self) -> Value {
        let mut out = Map::new();
        write_common(&mut out, self, &self.hidden_numbers);
        out.insert("places".into(), Value::Object(self.places.iter().map(|(b, p)| (b.raw().to_owned(), json!(p.raw()))).collect()));
        out.insert("hidden".into(), json!(self.hidden.iter().map(|b| b.raw()).collect::<Vec<_>>()));
        Value::Object(out)
    }
}

/// "Good morning, Wayne" by the hour on this Mac: morning from 5, afternoon
/// from 12, evening from 17, night from 22 until 5. A name is trimmed and
/// kept to its first 40 characters.
pub fn greeting(hour: u32, name: &str) -> String {
    let part = match hour {
        5..=11 => "morning",
        12..=16 => "afternoon",
        17..=21 => "evening",
        _ => "night",
    };
    let name: String = name.trim().chars().take(40).collect();
    if name.is_empty() { format!("Good {part}") } else { format!("Good {part}, {name}") }
}

/// The first word of a full name ("Wayne Sutton" is Wayne), what the
/// greeting suggests before one's typed.
pub fn first_name(full: &str) -> String {
    full.split_whitespace().next().unwrap_or_default().to_owned()
}

/// A GitHub user name as typed or pasted: an @, a profile link or spaces
/// taken off. Letters, digits and single hyphens, at most 39; anything else
/// is empty, so nothing odd goes in a URL.
pub fn github_user(typed: &str) -> String {
    let mut name = typed.trim();
    if let Some(at) = name.to_ascii_lowercase().find("github.com/") {
        name = &name[at + "github.com/".len()..];
    }
    let name = name.split('/').next().unwrap_or_default();
    let name = name.strip_prefix('@').unwrap_or(name);
    let ok = !name.is_empty() && name.len() <= 39 && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !name.starts_with('-') && !name.ends_with('-') && !name.contains("--");
    if ok { name.to_owned() } else { String::new() }
}

/// One day of a contributions calendar.
#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    /// yyyy-MM-dd.
    pub date: String,
    pub count: u64,
    /// 0 (none) to 4 (most), as GitHub shades it.
    pub level: u8,
}

/// A year of public GitHub contributions, or of threads started
/// (`GitHubContributions`): both draw with the same calendar.
#[derive(Clone, Debug, PartialEq)]
pub struct Contributions {
    pub year: i32,
    pub total: u64,
    pub days: Vec<Day>,
}

/// From the GitHub Contributions API (github-contributions-api.jogruber.de),
/// which reads the calendar on a GitHub profile. No token, and only the user
/// name is sent.
pub fn github_url(user: &str, year: i32) -> Option<String> {
    let name = github_user(user);
    (!name.is_empty()).then(|| format!("https://github-contributions-api.jogruber.de/v4/{name}?y={year}"))
}

/// What the service said, or why it couldn't be read.
#[derive(Debug, PartialEq)]
pub enum ParseError {
    Unreadable,
    /// The service's own error, such as an unknown user.
    Service(String),
}

/// What to say when a year couldn't be fetched. Only a real "no such user"
/// says the name is wrong; anything else says to try again.
pub fn github_failure(status: u16, service: Option<&str>, user: &str) -> String {
    let m = service.unwrap_or_default().to_lowercase();
    if status == 404 || m.contains("not found") || m.contains("no user") || m.contains("does not exist") {
        return format!("GitHub has no user called {}.", github_user(user));
    }
    "GitHub isn't answering right now. Try again in a few minutes.".into()
}

impl Contributions {
    pub fn parse(data: &[u8], year: i32) -> Result<Contributions, ParseError> {
        let v: Value = serde_json::from_slice(data).map_err(|_| ParseError::Unreadable)?;
        if let Some(e) = v["error"].as_str() {
            return Err(ParseError::Service(e.to_owned()));
        }
        let list = v["contributions"].as_array().ok_or(ParseError::Unreadable)?;
        let prefix = format!("{year}-");
        let mut days: Vec<Day> = list
            .iter()
            .filter_map(|d| {
                let date = d["date"].as_str()?;
                date.starts_with(&prefix).then(|| Day { date: date.to_owned(), count: d["count"].as_i64().unwrap_or(0).max(0) as u64, level: d["level"].as_i64().unwrap_or(0).clamp(0, 4) as u8 })
            })
            .collect();
        days.sort_by(|a, b| a.date.cmp(&b.date));
        let total = v["total"][year.to_string()].as_u64().unwrap_or_else(|| days.iter().map(|d| d.count).sum());
        Ok(Contributions { year, total, days })
    }

    /// The calendar as GitHub draws it: a column per week, Sunday at the
    /// top, every day of the year in place, and None before the 1st and
    /// after the 31st.
    pub fn weeks(&self) -> Vec<[Option<Day>; 7]> {
        let (Some(first), Some(next)) = (NaiveDate::from_ymd_opt(self.year, 1, 1), NaiveDate::from_ymd_opt(self.year + 1, 1, 1)) else { return Vec::new() };
        let by_date: BTreeMap<&str, &Day> = self.days.iter().map(|d| (d.date.as_str(), d)).collect();
        let mut weeks = Vec::new();
        let mut week: Vec<Option<Day>> = vec![None; first.weekday().num_days_from_sunday() as usize];
        let mut date = first;
        while date < next {
            let key = date.format("%Y-%m-%d").to_string();
            week.push(Some(by_date.get(key.as_str()).map_or(Day { date: key.clone(), count: 0, level: 0 }, |d| (*d).clone())));
            if week.len() == 7 {
                weeks.push(std::mem::take(&mut week).try_into().unwrap_or_else(|_| Default::default()));
            }
            date = date.succ_opt().unwrap_or(next);
        }
        if !week.is_empty() {
            week.resize(7, None);
            weeks.push(week.try_into().unwrap_or_else(|_| Default::default()));
        }
        weeks
    }

    /// Each month's short name and the week column its 1st falls in.
    pub fn month_starts(&self) -> Vec<(&'static str, usize)> {
        const NAMES: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let mut out = Vec::new();
        for (i, week) in self.weeks().iter().enumerate() {
            for day in week.iter().flatten().filter(|d| d.date.ends_with("-01")) {
                let month: usize = day.date.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
                out.push((NAMES[month.clamp(1, 12) - 1], i));
            }
        }
        out
    }
}

/// "Wednesday 12 November" for a yyyy-MM-dd key, for the calendar's readout.
pub fn day_title(date: &str) -> String {
    NaiveDate::parse_from_str(date, "%Y-%m-%d").map(|d| d.format("%A %-d %B").to_string()).unwrap_or_else(|_| date.to_owned())
}

/// A year of threads started, by this Mac's calendar day, shaded by quarters
/// of the busiest day (`ThreadActivity`). Threads in a private folder and
/// the tour's made-up threads aren't counted.
pub fn thread_activity<'a, Tz: TimeZone>(year: i32, threads: impl IntoIterator<Item = &'a ThreadSummary>, private_folders: &[String], home: &str, tz: &Tz) -> Contributions {
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for t in threads {
        if t.sample == Some(true) || is_private(&t.cwd, private_folders, home) {
            continue;
        }
        let Some(day) = t.created_at.as_deref().and_then(parse_ms).and_then(|ms| tz.timestamp_millis_opt(ms).single()).map(|d| d.date_naive()) else { continue };
        if day.year() == year {
            *counts.entry(day.format("%Y-%m-%d").to_string()).or_default() += 1;
        }
    }
    let most = counts.values().copied().max().unwrap_or(0);
    let days = counts.iter().map(|(date, n)| Day { date: date.clone(), count: *n, level: activity_level(*n, most) }).collect();
    Contributions { year, total: counts.values().sum(), days }
}

/// The first year with a thread, for how far back the arrows go.
pub fn first_thread_year<'a, Tz: TimeZone>(threads: impl IntoIterator<Item = &'a ThreadSummary>, tz: &Tz) -> Option<i32> {
    threads.into_iter().filter_map(|t| t.created_at.as_deref().and_then(parse_ms)).min().and_then(|ms| tz.timestamp_millis_opt(ms).single()).map(|d| d.year())
}

/// 0 for none, then 1 to 4 by quarters of the busiest day.
pub fn activity_level(count: u64, most: u64) -> u8 {
    if count == 0 || most == 0 {
        return 0;
    }
    ((4. * count as f64 / most as f64).ceil() as u8).clamp(1, 4)
}

/// A folder in the list, or anywhere under one, with `~` for home, as the
/// runner's own check (runner/src/models/index.ts, isPrivate).
pub fn is_private(cwd: &str, folders: &[String], home: &str) -> bool {
    let standard = |p: &str| {
        let p = match p.strip_prefix('~') {
            Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("{home}{rest}"),
            _ => p.to_owned(),
        };
        let p = p.trim_end_matches('/');
        if p.is_empty() && !home.is_empty() { "/".to_owned() } else { p.to_owned() }
    };
    let at = standard(cwd);
    folders.iter().any(|f| {
        let p = standard(f);
        !p.is_empty() && (at == p || at.starts_with(&format!("{}/", p.trim_end_matches('/'))))
    })
}

/// A quote for the Librarian page's Quote of the day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quote {
    pub text: &'static str,
    pub author: &'static str,
    /// Writer, physicist, artist...
    pub role: &'static str,
    /// Where it was said or written, and when.
    pub source: &'static str,
}

/// The day a quote belongs to: the local date after taking 4 hours off, so a
/// quote lasts through a late night and changes at 4 am.
pub fn quote_day<Tz: TimeZone>(now_ms: i64, tz: &Tz) -> String {
    tz.timestamp_millis_opt(now_ms - 4 * 3_600_000).single().map(|d| d.date_naive().format("%Y-%m-%d").to_string()).unwrap_or_default()
}

/// Today's quote, moved on by the refresh button's skips: the same on every
/// Mac (and in the Swift app) for the same day.
pub fn pick_quote(day: &str, skips: u32) -> Option<Quote> {
    let list = QUOTES;
    if list.is_empty() {
        return None;
    }
    let n = list.len() as i64;
    let order = shuffled(list.len());
    let index = (days(day) + skips as i64).rem_euclid(n);
    Some(list[order[index as usize]])
}

/// Days since 1 Jan 1970 for a yyyy-MM-dd key, by the calendar alone.
fn days(key: &str) -> i64 {
    let parts: Vec<i64> = key.split('-').filter_map(|p| p.parse().ok()).collect();
    let [y0, m, d] = parts[..] else { return 0 };
    // Days from civil (Howard Hinnant's), as the Swift app counts them.
    let y = if m <= 2 { y0 - 1 } else { y0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A fixed shuffle of 0..count (a seeded Fisher-Yates), the Swift app's.
fn shuffled(count: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    let mut seed: u64 = 0x7468_7265_6164_6265;
    for i in (1..count).rev() {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let j = ((seed >> 33) % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

const fn q(text: &'static str, author: &'static str, role: &'static str, source: &'static str) -> Quote {
    Quote { text, author, role, source }
}

/// Writers, poets, scientists, engineers, designers, artists, athletes and
/// secular philosophers, each with where they said it (Quotes.swift, which
/// says who's left out and why). The same list in the same order, so both
/// apps show the same quote on the same day.
pub const QUOTES: &[Quote] = &[
    // Building
    q("The purpose of business is to create a customer.", "Peter Drucker", "Management writer", "The Practice of Management, 1954"),
    q("There is nothing so useless as doing efficiently that which should not be done at all.", "Peter Drucker", "Management writer", "Harvard Business Review, 1963"),
    q("Plan to throw one away; you will, anyhow.", "Fred Brooks", "Computer scientist", "The Mythical Man-Month, 1975"),
    q("A complex system that works is invariably found to have evolved from a simple system that worked.", "John Gall", "Writer", "Systemantics, 1975"),
    q("The best is the enemy of the good.", "Voltaire", "Writer", "1772, in translation"),
    q("Good is the enemy of great.", "Jim Collins", "Writer", "Good to Great, 2001"),
    q("People don't buy what you do; they buy why you do it.", "Simon Sinek", "Writer", "Start With Why, 2009"),
    q("The best way to predict the future is to invent it.", "Alan Kay", "Computer scientist", "Xerox PARC, 1971"),
    q("It is possible to fly without motors, but not without knowledge and skill.", "Wilbur Wright", "Inventor", "Letter to Octave Chanute, 1900"),
    q("Any sufficiently advanced technology is indistinguishable from magic.", "Arthur C. Clarke", "Writer", "Profiles of the Future, 1973"),
    q("Be so good they can't ignore you.", "Steve Martin", "Comedian and writer", "Born Standing Up, 2007"),
    q("If you do not work on an important problem, it's unlikely you'll do important work.", "Richard Hamming", "Mathematician", "You and Your Research, 1986"),
    q("A ship in harbor is safe, but that is not what ships are built for.", "John A. Shedd", "Writer", "Salt from My Attic, 1928"),
    // Craft
    q("Simplicity is prerequisite for reliability.", "Edsger W. Dijkstra", "Computer scientist", "EWD498, 1975"),
    q("Premature optimization is the root of all evil.", "Donald Knuth", "Computer scientist", "Structured Programming with go to Statements, 1974"),
    q("Debugging is twice as hard as writing the code in the first place.", "Brian Kernighan and P. J. Plauger", "Computer scientists", "The Elements of Programming Style, 1978"),
    q("Any fool can write code that a computer can understand. Good programmers write code that humans can understand.", "Martin Fowler", "Engineer", "Refactoring, 1999"),
    q("Programs must be written for people to read, and only incidentally for machines to execute.", "Harold Abelson and Gerald Jay Sussman", "Computer scientists", "Structure and Interpretation of Computer Programs, 1985"),
    q("Talk is cheap. Show me the code.", "Linus Torvalds", "Engineer", "Linux kernel mailing list, 2000"),
    q("Good design is as little design as possible.", "Dieter Rams", "Designer", "Ten principles of good design"),
    q("Perfection is achieved, not when there is nothing more to add, but when there is nothing left to take away.", "Antoine de Saint-Exupéry", "Writer", "Terre des Hommes, 1939, in translation"),
    q("What I cannot create, I do not understand.", "Richard Feynman", "Physicist", "His Caltech blackboard, 1988"),
    q("If I have seen further it is by standing on the shoulders of Giants.", "Isaac Newton", "Scientist", "Letter to Robert Hooke, 1676"),
    // Creativity and habits
    q("Inspiration is for amateurs. The rest of us just show up and get to work.", "Chuck Close", "Artist", "Interview, 2006"),
    q("Genius is one percent inspiration and ninety-nine percent perspiration.", "Thomas Edison", "Inventor", "Harper's Monthly, 1932"),
    q("You can't use up creativity. The more you use, the more you have.", "Maya Angelou", "Writer", "Bell Telephone Magazine, 1982"),
    q("Creativity is a habit, and the best creativity is the result of good work habits.", "Twyla Tharp", "Choreographer", "The Creative Habit, 2003"),
    q("It's only by going through a volume of work that you will close that gap.", "Ira Glass", "Radio host", "Interview, 2009"),
    q("First forget inspiration. Habit is more dependable.", "Octavia E. Butler", "Writer", "Furor Scribendi, 1995"),
    q("Almost all good writing begins with terrible first efforts.", "Anne Lamott", "Writer", "Bird by Bird, 1994"),
    q("If there's a book that you want to read, but it hasn't been written yet, then you must write it.", "Toni Morrison", "Writer", "Speech, 1981"),
    q("How we spend our days is, of course, how we spend our lives.", "Annie Dillard", "Writer", "The Writing Life, 1989"),
    q("You do not rise to the level of your goals. You fall to the level of your systems.", "James Clear", "Writer", "Atomic Habits, 2018"),
    q("We are what we repeatedly do. Excellence, then, is not an act, but a habit.", "Will Durant", "Historian", "The Story of Philosophy, 1926"),
    q("Great things are done by a series of small things brought together.", "Vincent van Gogh", "Artist", "Letter to Theo, 1882"),
    q("If you hear a voice within you say you cannot paint, then by all means paint, and that voice will be silenced.", "Vincent van Gogh", "Artist", "Letter to Theo, 1883"),
    q("Have no fear of perfection, you'll never reach it.", "Salvador Dalí", "Artist", "50 Secrets of Magic Craftsmanship, 1948"),
    // Mindset
    q("Life is like riding a bicycle. To keep your balance you must keep moving.", "Albert Einstein", "Physicist", "Letter to his son Eduard, 1930"),
    q("I have no special talents. I am only passionately curious.", "Albert Einstein", "Physicist", "Letter to Carl Seelig, 1952"),
    q("I've failed over and over and over again in my life. And that is why I succeed.", "Michael Jordan", "Athlete", "Nike advert, 1997"),
    q("Try again. Fail again. Fail better.", "Samuel Beckett", "Writer", "Worstward Ho, 1983"),
    q("The best way out is always through.", "Robert Frost", "Poet", "A Servant to Servants, 1914"),
    q("I am not afraid of storms, for I am learning how to sail my ship.", "Louisa May Alcott", "Writer", "Little Women, 1868"),
    q("Men are disturbed not by things, but by the opinions about things.", "Epictetus", "Philosopher", "Enchiridion 5, in translation"),
    q("The first principle is that you must not fool yourself, and you are the easiest person to fool.", "Richard Feynman", "Physicist", "Caltech commencement, 1974"),
    q("All we have to decide is what to do with the time that is given us.", "J. R. R. Tolkien", "Writer", "The Fellowship of the Ring, 1954"),
    q("Tell me, what is it you plan to do with your one wild and precious life?", "Mary Oliver", "Poet", "The Summer Day, 1990"),
    q("It is good to have an end to journey toward; but it is the journey that matters, in the end.", "Ursula K. Le Guin", "Writer", "The Left Hand of Darkness, 1969"),
    q("One never notices what has been done; one can only see what remains to be done.", "Marie Curie", "Scientist", "Letter to her brother, 1894"),
    q("I love deadlines. I love the whooshing noise they make as they go by.", "Douglas Adams", "Writer", "The Salmon of Doubt, 2002"),
    q("No human is limited.", "Eliud Kipchoge", "Athlete", "INEOS 1:59 Challenge, 2019"),
    q("Pressure is a privilege.", "Billie Jean King", "Athlete", "Pressure Is a Privilege, 2008"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_swift_look_loads_and_saves_back_the_same() {
        let swift = json!({
            "picture": "own", "pictureFile": "picture-1a2b3c4d.png", "pictureSize": "large",
            "background": "background-9f8e7d6c.jpg", "backgroundDim": 0.3, "backgroundFocus": { "x": 0.2, "y": 0.7 },
            "places": { "github": "top", "numbers": "bottom", "nonsense": "top", "quote": "sideways" },
            "hidden": ["settings", "unknown"], "hiddenNumbers": ["tokens"], "githubUser": "octocat",
            "greetingShown": true, "greetingName": "Wayne", "greetingClock": false, "quoteShown": true,
            "quoteDay": "2026-11-15", "quoteSkips": 2, "activityShown": true, "profileButton": false, "hiddenAbout": ["line"],
        });
        let look = LibrarianLook::load(Some(&swift));
        assert_eq!(look.picture, Picture::Own);
        assert_eq!(look.picture_size, PictureSize::Large);
        assert_eq!(look.background_focus, BackdropFocus::new(0.2, 0.7));
        assert_eq!(look.place(LibrarianBlock::Github), Place::Top);
        assert_eq!(look.place(LibrarianBlock::Quote), Place::Top, "an unknown place is dropped");
        // Activity follows the numbers until it's moved on its own.
        assert_eq!(look.place(LibrarianBlock::Activity), Place::Bottom);
        assert!(!look.shows(LibrarianBlock::Settings));
        assert!(look.shows(LibrarianBlock::Github));
        assert_eq!(LibrarianLook::load(Some(&look.save())), look);
    }

    #[test]
    fn nothing_saved_is_the_page_as_it_ships() {
        let look = LibrarianLook::load(None);
        assert!(look.is_standard());
        assert_eq!(look.layout(), vec![LibrarianBlock::Picture, LibrarianBlock::About, LibrarianBlock::Numbers, LibrarianBlock::Settings]);
        assert!(ScheduledLook::load(Some(&json!("not a look"))).is_standard());
        assert_eq!(ScheduledLook::default().layout(), vec![ScheduledBlock::Picture, ScheduledBlock::About, ScheduledBlock::Numbers]);
    }

    #[test]
    fn a_file_name_that_is_a_path_is_never_used() {
        let look = ScheduledLook::load(Some(&json!({ "pictureFile": "../../secret.png", "background": "a/b.jpg" })));
        assert_eq!(look.picture_file, None);
        assert_eq!(look.background, None);
    }

    #[test]
    fn moving_a_part_back_is_no_change() {
        let mut look = ScheduledLook::default();
        look.set_place(Place::Bottom, ScheduledBlock::Numbers);
        assert!(!look.is_standard());
        look.set_place(Place::Top, ScheduledBlock::Numbers);
        assert!(look.is_standard());
    }

    #[test]
    fn your_picture_with_no_file_shows_nothing() {
        let mut look = LibrarianLook::default();
        look.picture = Picture::Own;
        assert!(!look.shows(LibrarianBlock::Picture));
        look.picture_file = Some("picture-1.png".into());
        assert!(look.shows(LibrarianBlock::Picture));
    }

    #[test]
    fn replacing_a_picture_deletes_only_the_old_copy() {
        let mut old = ScheduledLook::default();
        old.picture_file = Some("picture-old.png".into());
        old.background = Some("background-keep.jpg".into());
        let mut new = old.clone();
        new.picture_file = Some("picture-new.png".into());
        assert_eq!(unused(&old, &new), vec!["picture-old.png".to_owned()]);
    }

    #[test]
    fn the_photo_fills_the_space_around_its_focus() {
        // A 2:1 photo in a square: as wide as twice the space, bottom centre.
        assert_eq!(BackdropFocus::BOTTOM.frame((200., 100.), (100., 100.)), (-50., 0., 200., 100.));
        let left = BackdropFocus::BOTTOM.dragged((50., 0.), (200., 100.), (100., 100.));
        assert_eq!(left, BackdropFocus::new(0., 1.));
        // The side that already fits can't move.
        assert_eq!(BackdropFocus::BOTTOM.dragged((0., -40.), (200., 100.), (100., 100.)).y, 1.);
    }

    #[test]
    fn the_greeting_follows_the_clock() {
        assert_eq!(greeting(4, "Wayne"), "Good night, Wayne");
        assert_eq!(greeting(5, ""), "Good morning");
        assert_eq!(greeting(12, "  Wayne  "), "Good afternoon, Wayne");
        assert_eq!(greeting(17, "x"), "Good evening, x");
        assert_eq!(greeting(22, ""), "Good night");
        assert_eq!(first_name("Wayne Sutton"), "Wayne");
    }

    #[test]
    fn a_github_name_comes_out_of_whatever_was_pasted() {
        assert_eq!(github_user("@octocat"), "octocat");
        assert_eq!(github_user("https://github.com/waynesutton/threadbear"), "waynesutton");
        assert_eq!(github_user("no spaces"), "");
        assert_eq!(github_user("-bad"), "");
        assert_eq!(github_user("a--b"), "");
        assert_eq!(github_url("octocat", 2026).as_deref(), Some("https://github-contributions-api.jogruber.de/v4/octocat?y=2026"));
    }

    #[test]
    fn a_year_of_contributions_lays_out_as_github_draws_it() {
        let data = br#"{"total":{"2026":5},"contributions":[{"date":"2026-01-01","count":2,"level":1},{"date":"2025-12-31","count":9,"level":4},{"date":"2026-03-01","count":3,"level":7}]}"#;
        let c = Contributions::parse(data, 2026).unwrap();
        assert_eq!(c.total, 5);
        assert_eq!(c.days.len(), 2);
        assert_eq!(c.days[1].level, 4);
        let weeks = c.weeks();
        // 1 Jan 2026 is a Thursday.
        assert!(weeks[0][4].is_some() && weeks[0][3].is_none());
        assert_eq!(weeks.iter().flatten().flatten().count(), 365);
        assert_eq!(c.month_starts()[0], ("Jan", 0));
        assert_eq!(Contributions::parse(br#"{"error":"Not found"}"#, 2026), Err(ParseError::Service("Not found".into())));
        assert_eq!(github_failure(200, Some("GitHub: Gateway timeout."), "x"), "GitHub isn't answering right now. Try again in a few minutes.");
    }

    #[test]
    fn private_folders_are_left_out_of_thread_activity() {
        let thread = |cwd: &str, at: &str| ThreadSummary { cwd: cwd.into(), created_at: Some(at.into()), ..Default::default() };
        let threads = [thread("/Users/w/code", "2026-03-02T10:00:00Z"), thread("/Users/w/code", "2026-03-02T11:00:00Z"), thread("/Users/w/private/x", "2026-03-03T10:00:00Z"), thread("/Users/w/code", "2025-03-03T10:00:00Z")];
        let year = thread_activity(2026, &threads, &["~/private".into()], "/Users/w", &chrono::Utc);
        assert_eq!(year.total, 2);
        assert_eq!(year.days, vec![Day { date: "2026-03-02".into(), count: 2, level: 4 }]);
        assert_eq!(first_thread_year(&threads, &chrono::Utc), Some(2025));
        assert!(!is_private("/Users/w/privateer", &["~/private".into()], "/Users/w"));
    }

    #[test]
    fn the_quote_is_the_swift_apps_for_the_day() {
        assert_eq!(days("1970-01-01"), 0);
        assert_eq!(days("2026-11-15"), 20_772);
        let today = pick_quote("2026-11-15", 0).unwrap();
        assert_eq!(pick_quote("2026-11-15", 0), Some(today));
        assert_ne!(pick_quote("2026-11-15", 1), Some(today));
        assert_eq!(pick_quote("2026-11-15", QUOTES.len() as u32), Some(today));
        // The day turns at 4 am.
        let at = |s: &str| chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp_millis();
        assert_eq!(quote_day(at("2026-11-15T03:59:00Z"), &chrono::Utc), "2026-11-14");
        assert_eq!(quote_day(at("2026-11-15T04:00:00Z"), &chrono::Utc), "2026-11-15");
    }
}
