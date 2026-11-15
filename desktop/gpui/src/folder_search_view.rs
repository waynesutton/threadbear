// The search box over a folder on this Mac in the side pane, a port of the
// Swift app's FolderSearchView (Wayne's 3708b1c7): type, and the folder's
// list gives way to every line inside its files with those words, file by
// file, with line numbers. A line opens its file there with the words found,
// and Back comes back to the same results. Clearing the box shows the folder
// again. The search is threadbear_core::folder_search, run off the main
// thread, reading this Mac's disk: another Mac's folder has no box.
use crate::highlights::{icon, palette, Palette, PanePreview, SidePane};
use crate::style::{self, px};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{h_flex, v_flex, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use threadbear_core::folder_search::{self as core_search, Found};
use threadbear_core::inbox::ThreadSummary;

/// A pause in the typing before the folder is read.
const DELAY_MS: u64 = 250;

/// One folder's box and what it found, kept while the pane has the folder, so
/// Back from a file comes back to the same results.
pub(crate) struct SearchBox {
    pub(crate) input: Entity<InputState>,
    root: String,
    /// The words the results are for (trimmed).
    pub(crate) query: String,
    pub(crate) found: Option<Found>,
    searching: bool,
    /// Tells the search under way to stop: the words changed again.
    stop: Arc<AtomicBool>,
    _task: Option<Task<()>>,
    _sub: Subscription,
    scroll: ScrollHandle,
}

impl SidePane {
    /// The box for the folder `key` shows, made the first time it's drawn.
    pub(crate) fn ensure_search_box(&mut self, key: &str, root: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.files.searches.contains_key(key) {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search in these files"));
        let k = key.to_owned();
        let sub = cx.subscribe_in(&input, window, move |this, _, ev: &InputEvent, _, cx| {
            if matches!(ev, InputEvent::Change) {
                this.search_changed(&k, false, cx);
            }
        });
        self.files.searches.insert(key.to_owned(), SearchBox { input, root: root.to_owned(), query: String::new(), found: None, searching: false, stop: Arc::new(AtomicBool::new(false)), _task: None, _sub: sub, scroll: ScrollHandle::new() });
    }

    /// Whether the folder's list has given way to search results.
    pub(crate) fn searching_folder(&self, key: &str) -> bool {
        self.files.searches.get(key).is_some_and(|s| !s.query.is_empty())
    }

    /// The words changed: read the folder for them after a pause in the
    /// typing (or at once, for a script), stopping a search still going.
    fn search_changed(&mut self, key: &str, now: bool, cx: &mut Context<Self>) {
        let Some(search) = self.files.searches.get_mut(key) else { return };
        let query = search.input.read(cx).value().trim().to_owned();
        if query == search.query && (search.found.is_some() || search.searching) {
            return;
        }
        search.stop.store(true, Ordering::Relaxed);
        search.query = query.clone();
        search.found = None;
        if query.is_empty() {
            search.searching = false;
            search._task = None;
            cx.notify();
            return;
        }
        search.searching = true;
        let stop = Arc::new(AtomicBool::new(false));
        search.stop = stop.clone();
        let (root, k) = (search.root.clone(), key.to_owned());
        search._task = Some(cx.spawn(async move |this, cx| {
            if !now {
                cx.background_executor().timer(Duration::from_millis(DELAY_MS)).await;
            }
            if stop.load(Ordering::Relaxed) {
                return;
            }
            let (q, s) = (query.clone(), stop.clone());
            let found = cx.background_executor().spawn(async move { core_search::search(&root, &q, core_search::DEFAULT_LIMIT, &s) }).await;
            this.update(cx, |this, cx| {
                if let Some(search) = this.files.searches.get_mut(&k).filter(|s| s.query == query && !stop.load(Ordering::Relaxed)) {
                    search.found = Some(found);
                    search.searching = false;
                    cx.notify();
                }
            })
            .ok();
        }));
        cx.notify();
    }

    /// The box at the top of the folder: a magnifier and the words, with the
    /// field's own clear button.
    pub(crate) fn render_search_field(&self, key: &str, p: Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let search = self.files.searches.get(key)?;
        let fill = style::primary_opacity(cx, 0.07);
        Some(
            div()
                .flex_none()
                .w_full()
                .px(px(10.))
                .py(px(6.))
                .border_b_1()
                .border_color(p.hairline)
                .child(
                    crate::highlights::tipped(
                        h_flex()
                            .id("folder-search")
                            .h(px(24.))
                            .pl(px(8.))
                            .pr(px(4.))
                            .gap(px(5.))
                            .rounded(px(6.))
                            .bg(fill)
                            .items_center()
                            .child(icon("magnifyingglass", 12., p.secondary).flex_none())
                            .child(div().flex_1().min_w_0().child(Input::new(&search.input).appearance(false).small().cleanable(true).pl(px(0.)).text_size(px(style::size::LABEL)))),
                        "Search inside the files in this folder",
                    )
                    .into_any_element(),
                )
                .into_any_element(),
        )
    }

    /// What was found, in place of the list: a line saying how much, then
    /// each file's path with its lines under it, numbered.
    pub(crate) fn render_search_results(&mut self, key: &str, thread: &ThreadSummary, preview: &PanePreview, cx: &mut Context<Self>) -> AnyElement {
        let p = palette(cx);
        let Some(search) = self.files.searches.get(key) else { return div().into_any_element() };
        let quiet = |text: String| div().px(px(14.)).py(px(10.)).text_size(px(style::size::LABEL)).text_color(p.secondary).child(text).into_any_element();
        let Some(found) = search.found.clone() else {
            return v_flex().flex_1().min_h_0().child(quiet("Searching…".to_owned())).into_any_element();
        };
        let root = search.root.clone();
        let query = search.query.clone();
        let scroll = search.scroll.clone();
        let mut list = v_flex().id("folder-search-results").flex_1().min_h_0().overflow_y_scroll().track_scroll(&scroll).pb(px(8.));
        list = list.child(quiet(if found.hits.is_empty() { format!("Nothing in these files says \u{201c}{query}\u{201d}.") } else { core_search::summary(&found) }));
        let hover = p.hover;
        for (file_ix, (path, hits)) in core_search::by_file(&found.hits).into_iter().enumerate() {
            let relative = core_search::relative(path, &root).to_owned();
            list = list.child(
                h_flex()
                    .px(px(14.))
                    .pt(px(8.))
                    .pb(px(3.))
                    .gap(px(6.))
                    .items_center()
                    .child(icon("doc.text", 12., p.secondary).flex_none())
                    .child(div().min_w_0().truncate().text_size(px(style::size::LABEL)).font_weight(FontWeight::SEMIBOLD).text_color(p.fg).child(relative)),
            );
            for (hit_ix, hit) in hits.into_iter().enumerate() {
                let (hit, thread, preview, query, k) = (hit.clone(), thread.clone(), preview.clone(), query.clone(), key.to_owned());
                let row = h_flex()
                    .id(SharedString::from(format!("folder-hit-{file_ix}-{hit_ix}")))
                    .role(Role::Button)
                    .pl(px(32.))
                    .pr(px(14.))
                    .py(px(2.))
                    .gap(px(8.))
                    .items_start()
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover))
                    .child(div().flex_none().w(px(32.)).text_right().text_size(px(style::size::SMALL)).text_color(p.faint).font_family(cx.theme_mono()).child(hit.line.to_string()))
                    .child(div().flex_1().min_w_0().truncate().text_size(px(style::size::SMALL)).text_color(p.fg).font_family(cx.theme_mono()).child(hit.text.clone()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| this.open_search_hit(&k, &hit.path, hit.line, &query, &thread, &preview, cx)));
                list = list.child(crate::highlights::tipped(row, format!("Open {} at line {}", core_search::relative(path, &root), hit.line)));
            }
        }
        list.into_any_element()
    }

    /// The scripted `folder` command's `search`: the words in the box, and
    /// the folder read at once.
    #[cfg(feature = "automation")]
    pub(crate) fn search_typed(&mut self, key: &str, words: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.files.searches.get(key).map(|s| s.input.clone()) else { return };
        input.update(cx, |s, cx| s.set_value(words.to_owned(), window, cx));
        self.search_changed(key, true, cx);
    }

    /// The dump's `folderSearch`, while there are words in the box.
    #[cfg(feature = "automation")]
    pub(crate) fn search_dump(&self, key: &str) -> Option<serde_json::Value> {
        let search = self.files.searches.get(key).filter(|s| !s.query.is_empty())?;
        let (summary, hits) = match &search.found {
            Some(found) => (core_search::summary(found), found.hits.iter().map(|h| format!("{}:{}", core_search::relative(&h.path, &search.root), h.line)).collect::<Vec<_>>()),
            None => ("Searching…".to_owned(), vec![]),
        };
        Some(serde_json::json!({ "query": search.query, "summary": summary, "hits": hits }))
    }

    /// The scripted `folder` command's `openHit`: "src/app.ts:2".
    #[cfg(feature = "automation")]
    pub(crate) fn open_hit_named(&mut self, key: &str, named: &str, thread: &ThreadSummary, preview: &PanePreview, cx: &mut Context<Self>) -> Result<(), String> {
        let search = self.files.searches.get(key).ok_or("the folder has no search")?;
        let found = search.found.as_ref().ok_or("the search hasn't finished")?;
        let hit = found.hits.iter().find(|h| format!("{}:{}", core_search::relative(&h.path, &search.root), h.line) == named).cloned().ok_or_else(|| format!("no match {named}"))?;
        let query = search.query.clone();
        self.open_search_hit(key, &hit.path, hit.line, &query, thread, preview, cx);
        Ok(())
    }
}

trait MonoFont {
    fn theme_mono(&self) -> SharedString;
}

impl<T> MonoFont for Context<'_, T> {
    fn theme_mono(&self) -> SharedString {
        use gpui_kit::component::ActiveTheme as _;
        self.theme().mono_font_family.clone()
    }
}
