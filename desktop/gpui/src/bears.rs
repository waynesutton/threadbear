// Bears (docs/protocol.md, "Bears"; rules in threadbear_core::bears): a
// bear's tile, drawn on its thread's sidebar row in place of the folder, in
// the @ list and on a table's project page (Swift `BearTile`); the row
// menu's Bear Colour (`BearColorMenu`, `setBearColor`); and the bears this
// Mac's runner lists, for the @ list and the tables (`BearList`).
use crate::app::Threadbear;
use crate::icons;
use crate::sidebar::{MenuAction, MenuRow};
use crate::tips::Tip as _;
use gpui_kit::prelude::*;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::*;
use gpui_kit::component::Sizable;
// Shadows gpui's px, so sizes here follow the app's zoom (style.rs).
use crate::style::px;
use serde_json::json;
use std::time::{Duration, Instant};
use threadbear_core::bears::{self, BearEntry, ThreadBear};

/// The bears `bears.list` last gave (Swift `BearList`), asked for at most
/// every 20 seconds. A read that fails leaves the list as it was: the @
/// list is only a shortcut for typing a name, which works without it.
#[derive(Default)]
pub(crate) struct BearList {
    pub(crate) bears: Vec<BearEntry>,
    fetched_at: Option<Instant>,
    fetching: bool,
}

/// The busy key for colouring a bear, so a second pick waits for the first.
fn colour_key(bear: &str) -> String {
    format!("bearColor/{bear}")
}

impl Threadbear {
    /// `refreshBears`: asks this Mac's runner for its bears, unless it did
    /// in the last 20 seconds or is asking now.
    pub(crate) fn refresh_bears(&mut self, cx: &mut Context<Self>) {
        if self.bear_list.fetching || self.bear_list.fetched_at.is_some_and(|at| at.elapsed() < Duration::from_secs(20)) {
            return;
        }
        self.bear_list.fetching = true;
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = crate::call(client, "bears.list", json!({ "faces": false })).await;
            this.update(cx, |this, cx| {
                this.bear_list.fetching = false;
                this.bear_list.fetched_at = Some(Instant::now());
                if let Ok(value) = res {
                    this.bear_list.bears = bears::decode_list(&value);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// `setBearColor`: colours a bear's tile (`bears.update { bear, color }`,
    /// none for its fur). Its threads' rows change when the runner sends
    /// them again, so the banner says it's on its way meanwhile, as the
    /// click came from a menu, and a second pick waits for this one.
    pub(crate) fn set_bear_color(&mut self, bear: ThreadBear, color: Option<String>, cx: &mut Context<Self>) {
        let key = colour_key(&bear.id);
        if !self.begin_note(&key, &format!("Colouring {}…", bear.name), cx) {
            return;
        }
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let res = crate::call(client, "bears.update", json!({ "bear": bear.id, "color": color })).await;
            this.update(cx, |this, cx| {
                this.end_note(&key, cx);
                match res {
                    Ok(_) => {
                        // The list's tiles follow too, the next time it's asked.
                        this.bear_list.fetched_at = None;
                        this.flash_done(&format!("{}'s colour changed", bear.name), cx);
                    }
                    Err(e) => this.show_error(e.message, cx),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Bear Colour ▸ on a bear's thread row (`BearColorMenu`): the fur and
    /// the colours, the bear's own ticked; not while a pick is on its way.
    pub(crate) fn bear_colour_menu(&self, bear: &ThreadBear) -> MenuRow {
        let busy = self.busy_notes.iter().any(|(k, _)| *k == colour_key(&bear.id));
        let children = bears::COLOURS
            .iter()
            .map(|(name, hex)| {
                let current = bears::is_current(bear, *hex);
                let row = MenuRow::item(*name, if current { "checkmark" } else { "circle.fill" }, MenuAction::BearColor(bear.clone(), hex.map(str::to_owned)));
                if busy { row.disabled() } else { row }
            })
            .collect();
        MenuRow::submenu("Bear Colour", "paintpalette", children)
    }
}

/// A bear's tile: its symbol (a crown for the head bear when it has none,
/// a paw for any other) in white on its colour, the bear's fur when it has
/// none. `size` is the tile's side in points.
pub(crate) fn tile(bear: &ThreadBear, size: f32) -> Div {
    let (r, g, b) = bear.rgb();
    let fill = gpui_kit::rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b));
    let glyph = bear
        .icon
        .as_deref()
        .and_then(|name| icons::try_symbol(name).or_else(|| name.strip_suffix(".fill").and_then(icons::try_symbol)))
        .unwrap_or_else(|| icons::symbol(bear.fallback_symbol()));
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded(px(size * 7. / 30.))
        .bg(fill)
        .child(glyph.with_size(px(size * 14. / 30.)).text_color(gpui_kit::white()))
}

/// The tile with its tooltip, for where it stands alone (the @ list, a
/// table's page).
pub(crate) fn tipped_tile(id: impl Into<ElementId>, bear: &ThreadBear, size: f32) -> impl IntoElement {
    tile(bear, size).id(id).tip(bear.tip())
}

impl Threadbear {
    /// A table's members, then the tables among this project's Parts.
    pub(crate) fn render_table_bears(&self, project: &threadbear_core::projects::Project, cx: &App) -> Vec<AnyElement> {
        use gpui_kit::component::{v_flex, h_flex, ActiveTheme as _};
        let mut tables = Vec::new();
        if let Some(table) = &project.table { tables.push(("Bears at this table".to_owned(), table)); }
        for part in self.projects.list.iter().filter(|p| p.parent_id.as_deref() == Some(project.id.as_str()) && !p.archived) {
            if let Some(table) = &part.table { tables.push((format!("Bears at {}", part.title), table)); }
        }
        tables.into_iter().map(|(title, table)| {
            let mut section = v_flex().w_full().gap(px(2.)).child(crate::project_page::page_heading(&title, None, None, cx));
            for seat in bears::seats(table, &self.bear_list.bears) {
                let row = h_flex().id(SharedString::from(format!("seat-{}-{}", project.id, seat.tile.id))).test_support().aria_label(seat.tip.clone()).tip(seat.tip)
                    .gap(px(8.)).py(px(3.)).child(tile(&seat.tile, 22.))
                    .child(div().text_size(px(crate::style::size::LABEL)).text_color(cx.theme().foreground).child(seat.name))
                    .when_some(seat.does, |s, does| s.child(div().min_w_0().truncate().text_size(px(crate::style::size::DETAIL)).text_color(crate::style::secondary_text(cx)).child(does)));
                section = section.child(row);
            }
            if table.bears.is_empty() {
                section = section.child(div().text_size(px(crate::style::size::LABEL)).text_color(crate::style::faint_text(cx)).child(bears::NO_SEATS));
            }
            section.into_any_element()
        }).collect()
    }
}
