//! How the customizable pages look (the Librarian's, the Scheduled and
//! Projects pages'), a port of the Swift app's PageLook.swift,
//! ScheduledLook.swift and ProjectsLook.swift: which picture each shows, a
//! background photo behind it, where each part sits (top, middle or bottom)
//! and which parts show. Kept on this machine (desktop-settings.json, in the
//! same JSON shape the Swift app kept in UserDefaults), never sent to the
//! runner. Every field decodes on its own, so a look saved by an older or
//! newer build still loads.

use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub mod activity;
pub mod github;
pub mod profile;
pub mod quotes;

/// Where a part sits on its page.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Picture {
    /// The page's own bear, as shipped.
    Bear,
    /// A picture of the user's own, copied into the data folder.
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

    pub fn from_raw(raw: &str) -> Option<Picture> {
        Picture::ALL.into_iter().find(|p| p.raw() == raw)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PictureSize {
    Small,
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

    pub fn from_raw(raw: &str) -> Option<PictureSize> {
        PictureSize::ALL.into_iter().find(|p| p.raw() == raw)
    }
}

/// Where a background photo sits when it fills a space of another shape and
/// is cropped: 0...1 across and down, as with a unit point. (0.5, 1) keeps
/// the photo's bottom edge on the space's bottom edge, centred.
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

impl BackdropFocus {
    pub const BOTTOM: BackdropFocus = BackdropFocus { x: 0.5, y: 1. };

    pub fn new(x: f64, y: f64) -> BackdropFocus {
        BackdropFocus {
            x: Self::clamp(x),
            y: Self::clamp(y),
        }
    }

    fn clamp(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(0., 1.)
        } else {
            0.5
        }
    }

    /// Missing parts are the bottom centre's.
    pub fn decode(v: &Value) -> BackdropFocus {
        BackdropFocus::new(
            v["x"].as_f64().unwrap_or(0.5),
            v["y"].as_f64().unwrap_or(1.),
        )
    }

    pub fn to_value(self) -> Value {
        json!({ "x": self.x, "y": self.y })
    }

    /// The photo's frame (x, y, width, height), scaled to fill `space` (as
    /// aspect-fill does) and shifted so the focus point lines up: 0 puts the
    /// photo's left (or top) edge on the space's, 1 its right (or bottom)
    /// edge on the space's. Along the side that already fits, the photo
    /// can't move.
    pub fn frame(&self, image: (f32, f32), space: (f32, f32)) -> (f32, f32, f32, f32) {
        let ((iw, ih), (sw, sh)) = (image, space);
        if iw <= 0. || ih <= 0. || sw <= 0. || sh <= 0. {
            return (0., 0., sw, sh);
        }
        let scale = (sw / iw).max(sh / ih);
        let (w, h) = (iw * scale, ih * scale);
        ((sw - w) * self.x as f32, (sh - h) * self.y as f32, w, h)
    }

    /// The focus after dragging the photo by `by` (in the space's points)
    /// from `start`: the photo follows the pointer, and stops at its edges.
    pub fn dragged(
        start: BackdropFocus,
        by: (f32, f32),
        image: (f32, f32),
        space: (f32, f32),
    ) -> BackdropFocus {
        let (_, _, w, h) = start.frame(image, space);
        let spare_x = w - space.0;
        let spare_y = h - space.1;
        BackdropFocus::new(
            if spare_x > 0.5 {
                start.x - f64::from(by.0 / spare_x)
            } else {
                start.x
            },
            if spare_y > 0.5 {
                start.y - f64::from(by.1 / spare_y)
            } else {
                start.y
            },
        )
    }
}

/// What a customizable page's picture and background are made of, shared by
/// every page's look, so the Customize panels' Picture and Background
/// sections work on any of them rather than being copied per page.
#[derive(Clone, Debug, PartialEq)]
pub struct PictureParts {
    pub picture: Picture,
    /// The own picture's file name in the looks folder.
    pub picture_file: Option<String>,
    pub picture_size: PictureSize,
    /// The background picture's file name in the looks folder; None for none.
    pub background: Option<String>,
    /// How much of the window's own colour lies over the background, so text
    /// stays readable in light and dark. 0 is the photo as it is.
    pub background_dim: f64,
    /// Which part of the background shows when the window crops it: bottom
    /// centre unless moved, so the photo's bottom edge sits on the window's.
    pub background_focus: BackdropFocus,
}

impl PictureParts {
    fn standard(picture: Picture, size: PictureSize) -> PictureParts {
        PictureParts {
            picture,
            picture_file: None,
            picture_size: size,
            background: None,
            background_dim: 0.55,
            background_focus: BackdropFocus::BOTTOM,
        }
    }

    fn decode(v: &Value, picture: Picture, size: PictureSize) -> PictureParts {
        PictureParts {
            picture: v["picture"]
                .as_str()
                .and_then(Picture::from_raw)
                .unwrap_or(picture),
            picture_file: v["pictureFile"].as_str().map(str::to_owned),
            picture_size: v["pictureSize"]
                .as_str()
                .and_then(PictureSize::from_raw)
                .unwrap_or(size),
            background: v["background"].as_str().map(str::to_owned),
            background_dim: v["backgroundDim"].as_f64().unwrap_or(0.55).clamp(0., 1.),
            background_focus: if v["backgroundFocus"].is_object() {
                BackdropFocus::decode(&v["backgroundFocus"])
            } else {
                BackdropFocus::BOTTOM
            },
        }
    }

    fn write(&self, out: &mut Map<String, Value>) {
        out.insert("picture".into(), json!(self.picture.raw()));
        if let Some(file) = &self.picture_file {
            out.insert("pictureFile".into(), json!(file));
        }
        out.insert("pictureSize".into(), json!(self.picture_size.raw()));
        if let Some(file) = &self.background {
            out.insert("background".into(), json!(file));
        }
        out.insert("backgroundDim".into(), json!(self.background_dim));
        out.insert("backgroundFocus".into(), self.background_focus.to_value());
    }

    /// The picture shows: not none, and your own needs a file.
    fn has_picture(&self) -> bool {
        self.picture != Picture::None
            && (self.picture != Picture::Own || self.picture_file.is_some())
    }
}

/// A page's look as the Picture and Background sections see it.
pub trait PictureLook {
    fn pic(&self) -> &PictureParts;
    fn pic_mut(&mut self) -> &mut PictureParts;
    /// Puts the picture back on the page after it was hidden, as choosing a
    /// new one should.
    fn show_picture(&mut self);
}

/// The parts of a page, in the order they stack within a place.
pub trait Block: Copy + Ord + 'static {
    const ALL: &'static [Self];
    fn raw(self) -> &'static str;
    fn title(self) -> &'static str;
    fn from_raw(raw: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|b| b.raw() == raw)
    }
}

macro_rules! block {
    ($(#[$meta:meta])* $name:ident { $($case:ident => $raw:literal, $title:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name { $($case),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$case),+];
            pub fn raw(self) -> &'static str { match self { $($name::$case => $raw),+ } }
            pub fn title(self) -> &'static str { match self { $($name::$case => $title),+ } }
            pub fn from_raw(raw: &str) -> Option<$name> { $name::ALL.iter().copied().find(|b| b.raw() == raw) }
        }

        impl Block for $name {
            const ALL: &'static [$name] = $name::ALL;
            fn raw(self) -> &'static str { $name::raw(self) }
            fn title(self) -> &'static str { $name::title(self) }
        }
    };
}

block!(
    /// The Librarian page's parts.
    LibrarianBlock {
        Greeting => "greeting", "Greeting",
        Picture => "picture", "Picture",
        About => "about", "Title and status",
        Quote => "quote", "Quote of the day",
        Numbers => "numbers", "Numbers",
        // Short enough for its row in Layout; the card says Threadbear activity.
        Activity => "activity", "Thread activity",
        Github => "github", "GitHub activity",
        Settings => "settings", "Settings",
    }
);

block!(
    /// The parts of the Librarian page's title block.
    AboutPart {
        Title => "title", "Title",
        Line => "line", "Description",
        Status => "status", "Status",
    }
);

block!(
    /// The Scheduled page's parts.
    ScheduledBlock {
        Picture => "picture", "Picture",
        About => "about", "Title and status",
        Numbers => "numbers", "Numbers",
    }
);

block!(
    /// The Projects page's parts.
    ProjectsBlock {
        Picture => "picture", "Picture",
        Header => "header", "Title and search",
        Cards => "cards", "Projects",
    }
);

block!(
    /// The parts of a project's card that can go.
    CardPart {
        Art => "art", "Icon tile",
        Threads => "threads", "Open threads",
        Status => "status", "Status",
        State => "state", "Where it's up to",
    }
);

/// Unknown parts and places from another build are dropped, not fatal.
fn decode_places<B: Block>(v: &Value) -> BTreeMap<B, Place> {
    v.as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, p)| Some((B::from_raw(k)?, Place::from_raw(p.as_str()?)?)))
        .collect()
}

fn decode_set<B: Block>(v: &Value) -> BTreeSet<B> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|b| B::from_raw(b.as_str()?))
        .collect()
}

fn decode_strings(v: &Value) -> BTreeSet<String> {
    // Swift decodes the whole array or nothing.
    match v.as_array() {
        Some(list) if list.iter().all(Value::is_string) => list
            .iter()
            .filter_map(|s| s.as_str().map(str::to_owned))
            .collect(),
        _ => BTreeSet::new(),
    }
}

fn places_value<B: Block>(places: &BTreeMap<B, Place>) -> Value {
    Value::Object(
        places
            .iter()
            .map(|(b, p)| (b.raw().to_owned(), json!(p.raw())))
            .collect(),
    )
}

fn set_value<B: Block>(set: &BTreeSet<B>) -> Value {
    let mut raw: Vec<&str> = set.iter().map(|b| b.raw()).collect();
    raw.sort_unstable();
    json!(raw)
}

fn layout_of<B: Block>(place: impl Fn(B) -> Place, shows: impl Fn(B) -> bool) -> Vec<B> {
    Place::ALL
        .into_iter()
        .flat_map(|p| {
            B::ALL
                .iter()
                .copied()
                .filter(|&b| place(b) == p && shows(b))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn set_shown_in<B: Ord>(set: &mut BTreeSet<B>, shown: bool, part: B) {
    if shown {
        set.remove(&part);
    } else {
        set.insert(part);
    }
}

/// How the Librarian's page is laid out when Settings > General > Look has
/// "Customize the Librarian page" on.
#[derive(Clone, Debug, PartialEq)]
pub struct LibrarianLook {
    pub pic: PictureParts,
    pub places: BTreeMap<LibrarianBlock, Place>,
    pub hidden: BTreeSet<LibrarianBlock>,
    /// The numbers (by their key) taken off the page.
    pub hidden_numbers: BTreeSet<String>,
    /// Whose public GitHub contributions the GitHub block shows.
    pub github_user: String,
    /// The greeting at the top ("Good morning, Wayne") and the date and time
    /// under it. Off until turned on, so a look saved before it existed
    /// doesn't grow one.
    pub greeting_shown: bool,
    /// Who the greeting names; empty says just "Good morning".
    pub greeting_name: String,
    pub greeting_clock: bool,
    /// The title, the line under it and the status line, each of which can
    /// go while the others stay.
    pub hidden_about: BTreeSet<AboutPart>,
    /// A quote a day. Off until turned on, like the greeting.
    pub quote_shown: bool,
    /// The day the refresh button was last used, and how many times, so the
    /// skips last the day and start again the next.
    pub quote_day: String,
    pub quote_skips: i64,
    /// A year of the threads you started. Off until turned on.
    pub activity_shown: bool,
    /// A View profile page button beside Customize, while the profile page
    /// is published. On unless turned off.
    pub profile_button: bool,
}

impl Default for LibrarianLook {
    fn default() -> Self {
        LibrarianLook {
            pic: PictureParts::standard(Picture::Bear, PictureSize::Medium),
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

impl PictureLook for LibrarianLook {
    fn pic(&self) -> &PictureParts {
        &self.pic
    }
    fn pic_mut(&mut self) -> &mut PictureParts {
        &mut self.pic
    }
    fn show_picture(&mut self) {
        self.set_shown(true, LibrarianBlock::Picture);
    }
}

impl LibrarianLook {
    pub fn is_standard(&self) -> bool {
        *self == LibrarianLook::default()
    }

    /// Where a part sits when it hasn't been moved: as the page was before it
    /// could be customized, with GitHub under the numbers.
    pub fn default_place(block: LibrarianBlock) -> Place {
        use LibrarianBlock::*;
        match block {
            Greeting | Picture | About | Quote => Place::Top,
            Numbers | Activity | Github => Place::Middle,
            Settings => Place::Bottom,
        }
    }

    /// Threadbear activity goes with the numbers until it's moved on its
    /// own, as the profile page always had it just under them.
    pub fn place(&self, block: LibrarianBlock) -> Place {
        if let Some(place) = self.places.get(&block) {
            return *place;
        }
        if block == LibrarianBlock::Activity {
            self.place(LibrarianBlock::Numbers)
        } else {
            Self::default_place(block)
        }
    }

    pub fn set_place(&mut self, place: Place, block: LibrarianBlock) {
        if block == LibrarianBlock::Activity || place != Self::default_place(block) {
            self.places.insert(block, place);
        } else {
            self.places.remove(&block);
        }
    }

    /// Shown, as the user set it. GitHub also needs a user name.
    pub fn shows(&self, block: LibrarianBlock) -> bool {
        use LibrarianBlock::*;
        match block {
            Greeting => return self.greeting_shown,
            Quote => return self.quote_shown,
            Activity => return self.activity_shown,
            _ => {}
        }
        if self.hidden.contains(&block) {
            return false;
        }
        match block {
            About => self.hidden_about.len() < AboutPart::ALL.len(),
            Github => !github_user_clean(&self.github_user).is_empty(),
            Picture => self.pic.has_picture(),
            _ => true,
        }
    }

    pub fn set_shown(&mut self, shown: bool, block: LibrarianBlock) {
        match block {
            LibrarianBlock::Greeting => self.greeting_shown = shown,
            LibrarianBlock::Quote => self.quote_shown = shown,
            LibrarianBlock::Activity => self.activity_shown = shown,
            _ => set_shown_in(&mut self.hidden, shown, block),
        }
    }

    /// Whether the Layout switch for a part is on: greeting, quote and
    /// Threadbear activity have flags of their own, as they start off; the
    /// rest are on unless hidden.
    pub fn switched_on(&self, block: LibrarianBlock) -> bool {
        match block {
            LibrarianBlock::Greeting => self.greeting_shown,
            LibrarianBlock::Quote => self.quote_shown,
            LibrarianBlock::Activity => self.activity_shown,
            _ => !self.hidden.contains(&block),
        }
    }

    pub fn shows_part(&self, part: AboutPart) -> bool {
        !self.hidden_about.contains(&part)
    }

    pub fn set_part_shown(&mut self, shown: bool, part: AboutPart) {
        set_shown_in(&mut self.hidden_about, shown, part);
    }

    /// The parts to draw, top to bottom: by place, then in the parts' order.
    pub fn layout(&self) -> Vec<LibrarianBlock> {
        layout_of(|b| self.place(b), |b| self.shows(b))
    }

    /// How far today's quote has been moved on.
    pub fn quote_skips_on(&self, day: &str) -> i64 {
        if self.quote_day == day {
            self.quote_skips
        } else {
            0
        }
    }

    /// The refresh button: another quote for the rest of today.
    pub fn skip_quote(&mut self, day: &str) {
        self.quote_skips = self.quote_skips_on(day) + 1;
        self.quote_day = day.to_owned();
    }

    pub fn decode(v: &Value) -> LibrarianLook {
        let d = LibrarianLook::default();
        let bool_or = |key: &str, default: bool| v[key].as_bool().unwrap_or(default);
        let string = |key: &str| v[key].as_str().unwrap_or_default().to_owned();
        LibrarianLook {
            pic: PictureParts::decode(v, Picture::Bear, PictureSize::Medium),
            places: decode_places(&v["places"]),
            hidden: decode_set(&v["hidden"]),
            hidden_numbers: decode_strings(&v["hiddenNumbers"]),
            github_user: string("githubUser"),
            greeting_shown: bool_or("greetingShown", d.greeting_shown),
            greeting_name: string("greetingName"),
            greeting_clock: bool_or("greetingClock", d.greeting_clock),
            hidden_about: decode_set(&v["hiddenAbout"]),
            quote_shown: bool_or("quoteShown", d.quote_shown),
            quote_day: string("quoteDay"),
            quote_skips: v["quoteSkips"].as_i64().unwrap_or(0).max(0),
            activity_shown: bool_or("activityShown", d.activity_shown),
            profile_button: bool_or("profileButton", d.profile_button),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut out = Map::new();
        self.pic.write(&mut out);
        out.insert("places".into(), places_value(&self.places));
        out.insert("hidden".into(), set_value(&self.hidden));
        out.insert("hiddenNumbers".into(), json!(self.hidden_numbers));
        out.insert("githubUser".into(), json!(self.github_user));
        out.insert("greetingShown".into(), json!(self.greeting_shown));
        out.insert("greetingName".into(), json!(self.greeting_name));
        out.insert("greetingClock".into(), json!(self.greeting_clock));
        out.insert("quoteShown".into(), json!(self.quote_shown));
        out.insert("quoteDay".into(), json!(self.quote_day));
        out.insert("quoteSkips".into(), json!(self.quote_skips));
        out.insert("activityShown".into(), json!(self.activity_shown));
        out.insert("profileButton".into(), json!(self.profile_button));
        out.insert("hiddenAbout".into(), set_value(&self.hidden_about));
        Value::Object(out)
    }
}

/// How the Scheduled page is laid out when Settings > General > Look has
/// "Customize the Scheduled page" on. Smaller than the Librarian's look (no
/// greeting, quote or activity cards): the page is about what runs next.
/// Top, middle and bottom are real places in the window on this page.
#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledLook {
    pub pic: PictureParts,
    pub places: BTreeMap<ScheduledBlock, Place>,
    pub hidden: BTreeSet<ScheduledBlock>,
    /// The numbers (by their key) taken off the page.
    pub hidden_numbers: BTreeSet<String>,
}

impl Default for ScheduledLook {
    fn default() -> Self {
        ScheduledLook {
            pic: PictureParts::standard(Picture::Bear, PictureSize::Medium),
            places: BTreeMap::new(),
            hidden: BTreeSet::new(),
            hidden_numbers: BTreeSet::new(),
        }
    }
}

impl PictureLook for ScheduledLook {
    fn pic(&self) -> &PictureParts {
        &self.pic
    }
    fn pic_mut(&mut self) -> &mut PictureParts {
        &mut self.pic
    }
    fn show_picture(&mut self) {
        self.set_shown(true, ScheduledBlock::Picture);
    }
}

impl ScheduledLook {
    pub fn is_standard(&self) -> bool {
        *self == ScheduledLook::default()
    }

    /// All at the top, so the page looks as it ships. Middle and bottom are
    /// real places in the window on this page (the middle of the space, the
    /// bottom edge), so anything there moves away from the rest.
    pub fn default_place(_: ScheduledBlock) -> Place {
        Place::Top
    }

    pub fn place(&self, block: ScheduledBlock) -> Place {
        self.places
            .get(&block)
            .copied()
            .unwrap_or(Self::default_place(block))
    }

    /// Moving a part back to where it starts isn't kept as a change, so the
    /// look is standard again.
    pub fn set_place(&mut self, place: Place, block: ScheduledBlock) {
        if place == Self::default_place(block) {
            self.places.remove(&block);
        } else {
            self.places.insert(block, place);
        }
    }

    /// Shown, as the user set it. Your picture also needs a file.
    pub fn shows(&self, block: ScheduledBlock) -> bool {
        !self.hidden.contains(&block)
            && (block != ScheduledBlock::Picture || self.pic.has_picture())
    }

    pub fn set_shown(&mut self, shown: bool, block: ScheduledBlock) {
        set_shown_in(&mut self.hidden, shown, block);
    }

    pub fn layout(&self) -> Vec<ScheduledBlock> {
        layout_of(|b| self.place(b), |b| self.shows(b))
    }

    pub fn decode(v: &Value) -> ScheduledLook {
        ScheduledLook {
            pic: PictureParts::decode(v, Picture::Bear, PictureSize::Medium),
            places: decode_places(&v["places"]),
            hidden: decode_set(&v["hidden"]),
            hidden_numbers: decode_strings(&v["hiddenNumbers"]),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut out = Map::new();
        self.pic.write(&mut out);
        out.insert("places".into(), places_value(&self.places));
        out.insert("hidden".into(), set_value(&self.hidden));
        out.insert("hiddenNumbers".into(), json!(self.hidden_numbers));
        Value::Object(out)
    }
}

/// How the Projects page is laid out when Settings > General > Look has
/// "Customize the Projects page" on. The page ships with no picture, so
/// there's no bear: a picture of your own or none, starting as none. Then a
/// background photo, the picture, the title and search, and the cards each
/// at the top, middle or bottom or hidden, and which parts of each card show.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectsLook {
    pub pic: PictureParts,
    pub places: BTreeMap<ProjectsBlock, Place>,
    pub hidden: BTreeSet<ProjectsBlock>,
    pub hidden_card_parts: BTreeSet<CardPart>,
}

impl Default for ProjectsLook {
    fn default() -> Self {
        ProjectsLook {
            pic: PictureParts::standard(Picture::None, PictureSize::Small),
            places: BTreeMap::new(),
            hidden: BTreeSet::new(),
            hidden_card_parts: BTreeSet::new(),
        }
    }
}

impl PictureLook for ProjectsLook {
    fn pic(&self) -> &PictureParts {
        &self.pic
    }
    fn pic_mut(&mut self) -> &mut PictureParts {
        &mut self.pic
    }
    fn show_picture(&mut self) {
        self.set_shown(true, ProjectsBlock::Picture);
    }
}

impl ProjectsLook {
    /// The pictures this page offers: no bear, as it never had one.
    pub const PICTURES: [Picture; 2] = [Picture::None, Picture::Own];

    pub fn is_standard(&self) -> bool {
        *self == ProjectsLook::default()
    }

    /// Everything starts at the top, so the standard look is the page as it
    /// ships; middle and bottom are real places in the window.
    pub fn default_place(_: ProjectsBlock) -> Place {
        Place::Top
    }

    pub fn place(&self, block: ProjectsBlock) -> Place {
        self.places
            .get(&block)
            .copied()
            .unwrap_or(Self::default_place(block))
    }

    pub fn set_place(&mut self, place: Place, block: ProjectsBlock) {
        if place == Self::default_place(block) {
            self.places.remove(&block);
        } else {
            self.places.insert(block, place);
        }
    }

    /// Shown, as the user set it. The picture is only ever your own, so it
    /// needs a file.
    pub fn shows(&self, block: ProjectsBlock) -> bool {
        !self.hidden.contains(&block)
            && (block != ProjectsBlock::Picture
                || (self.pic.picture == Picture::Own && self.pic.picture_file.is_some()))
    }

    pub fn set_shown(&mut self, shown: bool, block: ProjectsBlock) {
        set_shown_in(&mut self.hidden, shown, block);
    }

    pub fn shows_card(&self, part: CardPart) -> bool {
        !self.hidden_card_parts.contains(&part)
    }

    pub fn set_card_shown(&mut self, shown: bool, part: CardPart) {
        set_shown_in(&mut self.hidden_card_parts, shown, part);
    }

    pub fn layout(&self) -> Vec<ProjectsBlock> {
        layout_of(|b| self.place(b), |b| self.shows(b))
    }

    pub fn decode(v: &Value) -> ProjectsLook {
        let mut pic = PictureParts::decode(v, Picture::None, PictureSize::Small);
        // The bear isn't offered here; one saved by another build is none.
        if !Self::PICTURES.contains(&pic.picture) {
            pic.picture = Picture::None;
        }
        ProjectsLook {
            pic,
            places: decode_places(&v["places"]),
            hidden: decode_set(&v["hidden"]),
            hidden_card_parts: decode_set(&v["hiddenCardParts"]),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut out = Map::new();
        self.pic.write(&mut out);
        out.insert("places".into(), places_value(&self.places));
        out.insert("hidden".into(), set_value(&self.hidden));
        out.insert("hiddenCardParts".into(), set_value(&self.hidden_card_parts));
        Value::Object(out)
    }
}

/// What the Librarian page's greeting says for the time on this machine, as
/// Claude greets you: morning from 5, afternoon from 12, evening from 17,
/// night from 22 until 5.
pub struct Greeting;

impl Greeting {
    pub fn part(hour: u32) -> &'static str {
        match hour {
            5..=11 => "morning",
            12..=16 => "afternoon",
            17..=21 => "evening",
            _ => "night",
        }
    }

    /// "Good morning, Wayne", or "Good morning" with no name. A name is
    /// trimmed and kept to its first 40 characters.
    pub fn text(hour: u32, name: &str) -> String {
        let hello = format!("Good {}", Self::part(hour));
        let name: String = name.trim().chars().take(40).collect();
        if name.is_empty() {
            hello
        } else {
            format!("{hello}, {name}")
        }
    }

    /// The first word of the account's full name ("Wayne Sutton" is Wayne),
    /// what the greeting suggests before you type one.
    pub fn first_name(full_name: &str) -> String {
        full_name
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned()
    }
}

/// A GitHub user name as typed or pasted: an @, a profile link or spaces
/// taken off. GitHub's own rule: letters, digits and single hyphens, at most
/// 39. Anything else is empty, so nothing odd goes in a URL.
pub fn github_user_clean(typed: &str) -> String {
    let mut name = typed.trim().to_owned();
    if let Some(at) = name.to_ascii_lowercase().find("github.com/") {
        name = name[at + "github.com/".len()..].to_owned();
    }
    name = name
        .split('/')
        .find(|s| !s.is_empty())
        .unwrap_or_default()
        .to_owned();
    if let Some(rest) = name.strip_prefix('@') {
        name = rest.to_owned();
    }
    let ok = !name.is_empty()
        && name.chars().count() <= 39
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--");
    if ok {
        name
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LibrarianBlock as B;

    fn load(json: &str) -> LibrarianLook {
        serde_json::from_str::<Value>(json)
            .map(|v| LibrarianLook::decode(&v))
            .unwrap_or_default()
    }

    #[test]
    fn standard_look_is_the_page_as_it_was() {
        let look = LibrarianLook::default();
        // GitHub stays off until there's a user name.
        assert_eq!(
            look.layout(),
            vec![B::Picture, B::About, B::Numbers, B::Settings]
        );
        assert!(look.is_standard());
    }

    #[test]
    fn parts_move_between_places_and_keep_their_order_within() {
        let mut look = LibrarianLook {
            github_user: "waynesutton".into(),
            ..Default::default()
        };
        look.set_place(Place::Bottom, B::Picture);
        look.set_place(Place::Top, B::Github);
        assert_eq!(
            look.layout(),
            vec![B::About, B::Github, B::Numbers, B::Picture, B::Settings]
        );
        // Moving back to where it starts isn't stored as a change.
        look.set_place(Place::Top, B::Picture);
        look.set_place(Place::Middle, B::Github);
        look.github_user = String::new();
        assert!(look.is_standard());
    }

    #[test]
    fn hidden_parts_and_picture_choices() {
        let mut look = LibrarianLook::default();
        look.set_shown(false, B::Numbers);
        look.pic.picture = Picture::None;
        assert_eq!(look.layout(), vec![B::About, B::Settings]);
        look.pic.picture = Picture::Own;
        assert!(!look.shows(B::Picture), "your picture needs a file");
        look.pic.picture_file = Some("picture.png".into());
        assert!(look.shows(B::Picture));
        look.set_shown(true, B::Numbers);
        assert_eq!(
            look.layout(),
            vec![B::Picture, B::About, B::Numbers, B::Settings]
        );
    }

    #[test]
    fn round_trips_and_tolerates_other_builds() {
        let mut look = LibrarianLook::default();
        look.pic.picture = Picture::Own;
        look.pic.picture_file = Some("a.png".into());
        look.pic.background = Some("b.jpg".into());
        look.pic.background_dim = 0.3;
        look.pic.background_focus = BackdropFocus::new(0.2, 0.4);
        look.pic.picture_size = PictureSize::Large;
        look.set_place(Place::Bottom, B::Numbers);
        look.hidden = [B::Settings].into();
        look.hidden_numbers = ["Waiting".to_owned()].into();
        look.github_user = "octocat".into();
        assert_eq!(LibrarianLook::decode(&look.to_value()), look);

        let loaded = load(
            r#"{"picture":"hologram","places":{"numbers":"top","rocket":"top","about":"sideways"},"hidden":["github","rocket"],"backgroundDim":7,"newThing":1}"#,
        );
        assert_eq!(loaded.pic.picture, Picture::Bear);
        assert_eq!(loaded.place(B::Numbers), Place::Top);
        assert_eq!(loaded.place(B::About), Place::Top);
        assert_eq!(loaded.hidden, [B::Github].into());
        assert_eq!(loaded.pic.background_dim, 1.);
        assert_eq!(
            loaded.pic.background_focus,
            BackdropFocus::BOTTOM,
            "a look saved before photos could move keeps them on the bottom"
        );
        assert!(
            loaded.profile_button,
            "a look saved before the View profile page button has it on"
        );
        let no_button = LibrarianLook {
            profile_button: false,
            ..Default::default()
        };
        assert!(!LibrarianLook::decode(&no_button.to_value()).profile_button);
        assert_eq!(
            load(r#"{"backgroundFocus":{"x":-3,"y":9}}"#)
                .pic
                .background_focus,
            BackdropFocus::new(0., 1.)
        );
        assert_eq!(load("nonsense"), LibrarianLook::default());
        assert_eq!(
            LibrarianLook::decode(&Value::Null),
            LibrarianLook::default()
        );
    }

    #[test]
    fn saves_in_the_swift_apps_shape() {
        let v = LibrarianLook::default().to_value();
        for key in [
            "picture",
            "pictureSize",
            "backgroundDim",
            "backgroundFocus",
            "places",
            "hidden",
            "hiddenNumbers",
            "githubUser",
            "greetingShown",
            "greetingName",
            "greetingClock",
            "quoteShown",
            "quoteDay",
            "quoteSkips",
            "activityShown",
            "profileButton",
            "hiddenAbout",
        ] {
            assert!(v.get(key).is_some(), "{key}");
        }
        assert!(
            v.get("pictureFile").is_none() && v.get("background").is_none(),
            "nil fields are left out, as encodeIfPresent does"
        );
        assert_eq!(v["backgroundFocus"], json!({ "x": 0.5, "y": 1.0 }));
    }

    #[test]
    fn backdrop_sits_on_the_bottom_unless_moved() {
        // A 4:3 photo in a wide window: cropped top and bottom, and by
        // default its bottom edge is the window's bottom edge.
        let photo = (400., 300.);
        let wide = (1000., 500.);
        let (_, y, w, h) = BackdropFocus::BOTTOM.frame(photo, wide);
        assert_eq!((w, h), (1000., 750.));
        assert_eq!(y + h, 500., "the photo's bottom is on the window's bottom");
        assert_eq!(BackdropFocus::new(0.5, 0.).frame(photo, wide).1, 0.);
        // A tall window crops the sides instead, centred.
        let (x, y, w, h) = BackdropFocus::BOTTOM.frame(photo, (300., 600.));
        assert_eq!(y + h, 600.);
        assert_eq!(x + w / 2., 150.);
    }

    #[test]
    fn dragging_moves_the_photo_with_the_pointer_and_stops_at_its_edges() {
        let photo = (400., 300.);
        let wide = (1000., 500.); // 250 points spare down
                                  // Dragging down shows more of the top: the photo moves down.
        let down = BackdropFocus::dragged(BackdropFocus::BOTTOM, (0., 125.), photo, wide);
        assert!((down.y - 0.5).abs() < 0.0001);
        assert_eq!(down.x, 0.5, "no spare width, so it can't move across");
        assert_eq!(
            BackdropFocus::dragged(BackdropFocus::BOTTOM, (0., 9999.), photo, wide).y,
            0.
        );
        assert_eq!(
            BackdropFocus::dragged(BackdropFocus::BOTTOM, (0., -50.), photo, wide).y,
            1.
        );
    }

    #[test]
    fn github_user_names() {
        assert_eq!(github_user_clean(" @waynesutton "), "waynesutton");
        assert_eq!(
            github_user_clean("https://github.com/mikecann/threadbear"),
            "mikecann"
        );
        assert_eq!(github_user_clean("a-b"), "a-b");
        assert_eq!(github_user_clean("a--b"), "");
        assert_eq!(github_user_clean("-ab"), "");
        assert_eq!(github_user_clean("a b"), "");
        assert_eq!(github_user_clean("ab?y=1"), "");
        assert_eq!(github_user_clean("é"), "");
        assert_eq!(github_user_clean(&"a".repeat(40)), "");
    }

    #[test]
    fn greeting_by_the_clock() {
        assert_eq!(Greeting::text(5, "Wayne"), "Good morning, Wayne");
        assert_eq!(Greeting::text(11, " Wayne "), "Good morning, Wayne");
        assert_eq!(Greeting::text(12, "Wayne"), "Good afternoon, Wayne");
        assert_eq!(Greeting::text(17, ""), "Good evening");
        assert_eq!(Greeting::text(22, "Wayne"), "Good night, Wayne");
        assert_eq!(Greeting::text(0, "Wayne"), "Good night, Wayne");
        assert_eq!(Greeting::text(4, "Wayne"), "Good night, Wayne");
        assert_eq!(Greeting::first_name("Wayne Sutton"), "Wayne");
        assert_eq!(Greeting::first_name(""), "");
    }

    #[test]
    fn greeting_and_title_parts_are_off_or_on_by_themselves() {
        let mut look = LibrarianLook::default();
        assert!(
            !look.shows(B::Greeting),
            "a look saved before the greeting doesn't grow one"
        );
        look.set_shown(true, B::Greeting);
        assert_eq!(look.layout().first(), Some(&B::Greeting));
        look.set_part_shown(false, AboutPart::Title);
        look.set_part_shown(false, AboutPart::Line);
        assert!(look.shows(B::About), "the status line is still there");
        look.set_part_shown(false, AboutPart::Status);
        assert!(!look.shows(B::About), "nothing left of it, so no gap");
        look.greeting_name = "Wayne".into();
        look.greeting_clock = false;
        assert_eq!(LibrarianLook::decode(&look.to_value()), look);
        let old = load(r#"{"picture":"bear"}"#);
        assert!(!old.greeting_shown);
        assert!(old.greeting_clock);
        assert!(old.hidden_about.is_empty());
    }

    #[test]
    fn quote_refresh_lasts_the_day() {
        let mut look = LibrarianLook::default();
        assert!(!look.shows(B::Quote), "off until turned on");
        look.set_shown(true, B::Quote);
        assert_eq!(
            look.layout(),
            vec![B::Picture, B::About, B::Quote, B::Numbers, B::Settings]
        );
        look.skip_quote("2026-11-09");
        look.skip_quote("2026-11-09");
        assert_eq!(look.quote_skips_on("2026-11-09"), 2);
        assert_eq!(
            look.quote_skips_on("2026-11-10"),
            0,
            "a new day starts again"
        );
        look.skip_quote("2026-11-10");
        assert_eq!(look.quote_skips_on("2026-11-10"), 1);
        assert_eq!(LibrarianLook::decode(&look.to_value()), look);
    }

    #[test]
    fn activity_starts_off_and_follows_the_numbers() {
        let mut look = LibrarianLook::default();
        assert!(!look.shows(B::Activity));
        look.set_shown(true, B::Activity);
        assert_eq!(
            look.layout(),
            vec![B::Picture, B::About, B::Numbers, B::Activity, B::Settings]
        );
        // With the numbers wherever they go, until moved on its own.
        look.set_place(Place::Bottom, B::Numbers);
        assert_eq!(
            look.layout(),
            vec![B::Picture, B::About, B::Numbers, B::Activity, B::Settings]
        );
        look.set_place(Place::Top, B::Activity);
        assert_eq!(
            look.layout(),
            vec![B::Picture, B::About, B::Activity, B::Numbers, B::Settings]
        );
        // Numbers went to the bottom, so GitHub (middle) is between them.
        assert_eq!(
            profile::card_order(&look),
            vec!["activity", "github", "numbers", "projects"]
        );
        let back = LibrarianLook::decode(&look.to_value());
        assert_eq!(back, look);
        assert!(back.activity_shown);
        assert!(!load(r#"{"picture":"bear","quoteShown":true}"#).activity_shown);
    }

    #[test]
    fn scheduled_look() {
        use ScheduledBlock as S;
        let mut look = ScheduledLook::default();
        assert_eq!(look.layout(), vec![S::Picture, S::About, S::Numbers]);
        assert!(look.is_standard());
        assert!(
            S::ALL.iter().all(|&b| look.place(b) == Place::Top),
            "all at the top, as the page ships"
        );
        look.set_place(Place::Bottom, S::Picture);
        assert_eq!(look.layout(), vec![S::About, S::Numbers, S::Picture]);
        look.set_place(Place::Middle, S::About);
        assert_eq!(look.layout(), vec![S::Numbers, S::About, S::Picture]);
        look.set_place(Place::Top, S::Picture);
        look.set_place(Place::Top, S::About);
        assert!(look.is_standard());

        look.set_shown(false, S::About);
        assert_eq!(look.layout(), vec![S::Picture, S::Numbers]);
        look.pic.picture = Picture::None;
        assert_eq!(look.layout(), vec![S::Numbers]);
        look.pic.picture = Picture::Own;
        assert!(!look.shows(S::Picture), "your picture needs a file");
        look.pic.picture_file = Some("picture.png".into());
        look.set_shown(false, S::Picture);
        look.show_picture();
        assert!(
            look.shows(S::Picture),
            "choosing a picture puts it back on the page"
        );
    }

    #[test]
    fn scheduled_round_trips_and_tolerates_other_builds() {
        use ScheduledBlock as S;
        let mut look = ScheduledLook::default();
        look.pic.picture = Picture::Own;
        look.pic.picture_file = Some("a.png".into());
        look.pic.picture_size = PictureSize::Small;
        look.pic.background = Some("b.jpg".into());
        look.pic.background_dim = 0.2;
        look.pic.background_focus = BackdropFocus::new(0.1, 0.3);
        look.set_place(Place::Bottom, S::About);
        look.hidden = [S::Numbers].into();
        look.hidden_numbers = ["paused".to_owned()].into();
        assert_eq!(ScheduledLook::decode(&look.to_value()), look);

        let v: Value = serde_json::from_str(r#"{"picture":"hologram","places":{"numbers":"bottom","rocket":"top","about":"sideways"},"hidden":["picture","greeting"],"backgroundDim":-4,"newThing":1}"#).unwrap();
        let loaded = ScheduledLook::decode(&v);
        assert_eq!(loaded.pic.picture, Picture::Bear);
        assert_eq!(loaded.place(S::Numbers), Place::Bottom);
        assert_eq!(loaded.place(S::About), Place::Top);
        assert_eq!(loaded.hidden, [S::Picture].into());
        assert_eq!(loaded.pic.background_dim, 0.);
        assert_eq!(
            ScheduledLook::decode(&Value::Null),
            ScheduledLook::default()
        );
    }

    #[test]
    fn projects_look() {
        use ProjectsBlock as P;
        let mut look = ProjectsLook::default();
        assert_eq!(
            look.pic.picture,
            Picture::None,
            "the page has no picture as it ships"
        );
        assert_eq!(look.layout(), vec![P::Header, P::Cards]);
        assert!(
            CardPart::ALL.iter().all(|&c| look.shows_card(c)),
            "every card part shows"
        );
        assert!(look.is_standard());
        look.set_place(Place::Bottom, P::Header);
        assert_eq!(look.layout(), vec![P::Cards, P::Header]);
        look.set_place(Place::Middle, P::Cards);
        assert_eq!(look.layout(), vec![P::Cards, P::Header]);
        look.set_place(Place::Top, P::Header);
        look.set_place(Place::Top, P::Cards);
        assert!(look.is_standard(), "moving back isn't kept as a change");

        look.pic.picture = Picture::Own;
        assert!(!look.shows(P::Picture), "your picture needs a file");
        look.pic.picture_file = Some("picture.png".into());
        assert_eq!(look.layout(), vec![P::Picture, P::Header, P::Cards]);
        look.set_shown(false, P::Picture);
        look.show_picture();
        assert!(
            look.shows(P::Picture),
            "choosing a picture puts it back on the page"
        );
        look.pic.picture = Picture::Bear;
        assert!(!look.shows(P::Picture), "there's no bear on this page");

        let mut cards = ProjectsLook::default();
        cards.set_card_shown(false, CardPart::Art);
        cards.set_card_shown(false, CardPart::State);
        assert_eq!(
            cards.hidden_card_parts,
            [CardPart::Art, CardPart::State].into()
        );
        assert!(!cards.is_standard());
        cards.set_card_shown(true, CardPart::Art);
        cards.set_card_shown(true, CardPart::State);
        assert!(cards.is_standard());
    }

    #[test]
    fn projects_round_trips_and_tolerates_other_builds() {
        use ProjectsBlock as P;
        let mut look = ProjectsLook::default();
        look.pic.picture = Picture::Own;
        look.pic.picture_file = Some("a.png".into());
        look.pic.picture_size = PictureSize::Large;
        look.pic.background = Some("b.jpg".into());
        look.pic.background_dim = 0.2;
        look.pic.background_focus = BackdropFocus::new(0.1, 0.3);
        look.set_place(Place::Bottom, P::Cards);
        look.hidden = [P::Header].into();
        look.hidden_card_parts = [CardPart::Status].into();
        assert_eq!(ProjectsLook::decode(&look.to_value()), look);

        let v: Value = serde_json::from_str(r#"{"picture":"bear","places":{"cards":"middle","rocket":"top","header":"sideways"},"hidden":["picture","numbers"],"hiddenCardParts":["art","sparkles"],"backgroundDim":9,"newThing":1}"#).unwrap();
        let loaded = ProjectsLook::decode(&v);
        assert_eq!(
            loaded.pic.picture,
            Picture::None,
            "a bear from another build is none here"
        );
        assert_eq!(loaded.place(P::Cards), Place::Middle);
        assert_eq!(loaded.place(P::Header), Place::Top);
        assert_eq!(loaded.hidden, [P::Picture].into());
        assert_eq!(loaded.hidden_card_parts, [CardPart::Art].into());
        assert_eq!(loaded.pic.background_dim, 1.);
        assert_eq!(
            ProjectsLook::decode(&json!("nonsense")),
            ProjectsLook::default()
        );
    }
}
