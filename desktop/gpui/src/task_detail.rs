// A task opened on its own, as Linear opens an issue: the title, a block of
// properties, the description, sub-tasks, links and attachments, what
// happened to it with its comments, and what its project has been up to. It
// takes the whole Tasks pane (Back or Esc goes back, J and K or the arrows
// step through the tasks), since a sheet over the board left no room for
// sub-tasks and comments in a narrow window. Everything Mike clicks shows at
// once and goes back, with the error, if the runner refuses it; an older
// runner without the newer task calls says so where the section would be.
use super::*;
use threadbear_core::chrono::{Datelike as _, Local, NaiveDate};
use threadbear_core::task_dates::{self, Due};

/// The three bars, filled from the left as far as the priority goes; the rest
/// faint. Drawn as one mask, so the faint bars take the same colour.
const BARS: [&[u8]; 3] = [
    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="1.5" y="8" width="3" height="6" rx="1" fill="black"/><rect x="6.5" y="5" width="3" height="9" rx="1" fill="black" fill-opacity=".3"/><rect x="11.5" y="2" width="3" height="12" rx="1" fill="black" fill-opacity=".3"/></svg>"#,
    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="1.5" y="8" width="3" height="6" rx="1" fill="black"/><rect x="6.5" y="5" width="3" height="9" rx="1" fill="black"/><rect x="11.5" y="2" width="3" height="12" rx="1" fill="black" fill-opacity=".3"/></svg>"#,
    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect x="1.5" y="8" width="3" height="6" rx="1" fill="black"/><rect x="6.5" y="5" width="3" height="9" rx="1" fill="black"/><rect x="11.5" y="2" width="3" height="12" rx="1" fill="black"/></svg>"#,
];
/// Urgent: a rounded square with the "!" cut out of it.
const URGENT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path fill-rule="evenodd" fill="black" d="M4 1h8a3 3 0 0 1 3 3v8a3 3 0 0 1-3 3H4a3 3 0 0 1-3-3V4a3 3 0 0 1 3-3zM7 3.5h2v6.5H7zM7 11.5h2v2H7z"/></svg>"#;
/// No priority: three short dashes.
const NO_PRIORITY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><g fill="black" fill-opacity=".7"><rect x="1.5" y="7.25" width="3" height="1.5" rx=".75"/><rect x="6.5" y="7.25" width="3" height="1.5" rx=".75"/><rect x="11.5" y="7.25" width="3" height="1.5" rx=".75"/></g></svg>"#;

pub(super) fn urgent_red() -> Hsla {
    Hsla::from(rgb(0xe5484d))
}

/// A priority's mark: Urgent red, the bars and dashes in the text colour.
pub(super) fn priority_icon(priority: Option<&str>, size: f32, cx: &App) -> Icon {
    let icon = match priority {
        Some("urgent") => return Icon::default().data(URGENT).with_size(px(size)).text_color(urgent_red()),
        Some(p) if task_dates::priority_bars(p) > 0 => Icon::default().data(BARS[task_dates::priority_bars(p) - 1]),
        _ => Icon::default().data(NO_PRIORITY),
    };
    icon.with_size(px(size)).text_color(style::secondary_label(cx))
}

pub(super) fn today() -> NaiveDate {
    Local::now().date_naive()
}

/// The due date's chip on a row or card: red when it's passed, amber today.
pub(super) fn due_chip(due: &str, time: Option<&str>, cx: &App) -> Option<AnyElement> {
    let day = task_dates::parse(due)?;
    let now = today();
    let time = time.and_then(task_dates::parse_time);
    // Today with its time already gone counts as overdue.
    let passed = time.is_some_and(|t| day == now && t <= Local::now().time());
    let color = match if passed { Due::Overdue } else { task_dates::due_state(day, now) } {
        Due::Overdue => urgent_red(),
        Due::Today => Hsla::from(rgb(0xd9870b)),
        Due::Later => style::secondary_label(cx),
    };
    let mut label = task_dates::chip_label(day, now);
    if let Some(t) = time {
        label = format!("{label} {}", task_dates::time_label(t, style::uses_24_hour_clock()));
    }
    Some(
        h_flex().flex_none().gap(px(3.)).px(px(6.)).py(px(1.)).rounded_full().border_1().border_color(style::hairline(cx)).text_size(px(12.)).text_color(color)
            .child(icons::symbol("calendar").with_size(px(11.)).text_color(color))
            .child(label.clone().test_text(format!("Due {label}")))
            .into_any_element(),
    )
}

/// "1/3": the sub-tasks done of all of them.
pub(super) fn subtask_chip(done: usize, all: usize, cx: &App) -> AnyElement {
    h_flex().flex_none().gap(px(3.)).px(px(6.)).py(px(1.)).rounded_full().border_1().border_color(style::hairline(cx)).text_size(px(12.)).text_color(style::secondary_label(cx))
        .child(icons::symbol("checklist").with_size(px(11.)).text_color(style::tertiary_label(cx)))
        .child(format!("{done}/{all}"))
        .into_any_element()
}

/// What the open task's page has loaded and has open, beside the task itself.
#[derive(Default)]
pub(super) struct Detail {
    pub task: String,
    pub activity: Option<Result<Vec<Value>, String>>,
    pub related: Option<Result<Value, String>>,
    pub comments_only: bool,
    pub picker: Option<&'static str>,
    pub picker_closed_at: Option<std::time::Instant>,
    /// The Print search was just opened: scroll it into view once it's laid out.
    pub reveal_picker: std::rc::Rc<std::cell::Cell<bool>>,
    pub month: (i32, u32),
    pub adding_sub: bool,
    pub adding_link: bool,
    pub attaching: bool,
    pub error: Option<String>,
    pub images: HashMap<String, Option<Arc<Image>>>,
    /// linear.refresh has been asked once since this task's page opened.
    pub linear_refreshed: bool,
    /// Refresh (from Linear) was pressed and hasn't come back.
    pub linear_refreshing: bool,
}
use std::sync::Arc;

/// A picture from storage, off the UI thread, held to 10 MB.
pub(crate) fn fetch_picture(url: String) -> impl std::future::Future<Output = Result<Arc<Image>, String>> {
    async move {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(20))).build().into();
        let mut response = agent.get(&url).call().map_err(|e| e.to_string())?;
        let bytes = response.body_mut().with_config().limit(10 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?;
        let format = crate::viewer::format_of("", &bytes).ok_or("Unsupported picture format")?;
        Ok(Arc::new(Image::from_bytes(format, bytes)))
    }
}

fn link_symbol(kind: &str) -> &'static str {
    match kind {
        "print" => "doc.richtext",
        "image" => "photo",
        "thread" => "bubble.left",
        _ => "link",
    }
}

impl TasksPage {
    /// The Print search opens under Attach Print, which can sit at the foot
    /// of a short window: on GitHub's 1024 by 768 Mac it opened out of sight
    /// (18 Nov). Scrolled into view as it's laid out the first time, so it
    /// doesn't wait on another frame (a window kept behind may not draw one).
    fn reveal_print_picker(&self) -> AnyElement {
        let (scroll, pending) = (self.detail_scroll.clone(), self.detail.reveal_picker.clone());
        canvas(move |rect, window, _| {
            if !pending.replace(false) {
                return;
            }
            let pane = scroll.bounds();
            if rect.bottom() > pane.bottom() {
                let mut offset = scroll.offset();
                offset.y -= (rect.bottom() - pane.bottom()).min(rect.top() - pane.top());
                scroll.set_offset(offset);
                window.refresh();
            }
        }, |_, _, _, _| {}).absolute().inset_0().size_full().into_any_element()
    }

    pub(super) fn open_row(&self) -> Option<&TaskRow> {
        let id = self.editing.as_ref()?;
        self.rows.iter().find(|r| &r.id == id)
    }
    pub(super) fn subtasks(&self, id: &str) -> Vec<TaskRow> {
        let mut out: Vec<TaskRow> = self.rows.iter().filter(|r| r.parent_id.as_deref() == Some(id)).cloned().collect();
        out.sort_by(|a, b| a.position.total_cmp(&b.position).then(a.id.cmp(&b.id)));
        out
    }
    /// The task's own sub-task count, for its chip: done, all.
    pub(super) fn subtask_counts(&self, id: &str) -> (usize, usize) {
        let subs = self.rows.iter().filter(|r| r.parent_id.as_deref() == Some(id));
        subs.fold((0, 0), |(d, a), r| (d + (r.status == "done") as usize, a + 1))
    }

    /// The open task's activity and its project's threads and prints.
    pub(super) fn load_detail(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.detail.task != id {
            let now = today();
            self.detail = Detail { task: id.to_string(), month: (now.year(), now.month()), images: std::mem::take(&mut self.detail.images), ..Default::default() };
        }
        self.load_activity(cx);
        let (client, id) = (self.client.clone(), id.to_string());
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.related", json!({"id": id})).await;
            this.update(cx, |this, cx| {
                if this.detail.task == id {
                    this.detail.related = Some(result.map_err(|e| e.message));
                    cx.notify();
                }
            }).ok();
        }).detach();
        self.load_images(cx);
        self.load_linear_status(cx);
        self.refresh_open_linear(cx);
    }
    pub(super) fn load_activity(&mut self, cx: &mut Context<Self>) {
        let id = self.detail.task.clone();
        if id.is_empty() { return; }
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.activity", json!({"id": id})).await;
            this.update(cx, |this, cx| {
                if this.detail.task == id {
                    this.detail.activity = Some(result.map(|v| v["items"].as_array().cloned().unwrap_or_default()).map_err(|e| e.message));
                    cx.notify();
                }
            }).ok();
        }).detach();
    }
    pub(super) fn load_images(&mut self, cx: &mut Context<Self>) {
        let urls: Vec<String> = self.open_row().map(|r| r.links.iter().filter_map(|l| l["imageUrl"].as_str().map(String::from)).collect()).unwrap_or_default();
        for url in urls {
            if self.detail.images.contains_key(&url) { continue; }
            self.detail.images.insert(url.clone(), None);
            let fetch = cx.background_executor().spawn(fetch_picture(url.clone()));
            cx.spawn(async move |this, cx| {
                let result = fetch.await;
                this.update(cx, |this, cx| {
                    if let Ok(image) = result { this.detail.images.insert(url, Some(image)); }
                    cx.notify();
                }).ok();
            }).detach();
        }
    }

    /// A property of the open task changed here: shown now, sent, and put
    /// back with the error if the runner refuses it.
    pub(super) fn set_field(&mut self, id: &str, field: &str, value: Value, cx: &mut Context<Self>) {
        if field == "status" {
            let status = value.as_str().unwrap_or("todo").to_string();
            if self.editing.as_deref() == Some(id) {
                self.status = status.clone();
                let canonical = self.canonical_of(&status);
                if let Some(base) = self.editing_baseline.as_mut() { base.status = canonical; base.column_id = Some(status.clone()); }
            }
            let canonical = self.canonical_of(&status);
            self.patch(id, json!({"status": canonical, "columnId": status}), None, cx);
            return;
        }
        if field == "project" && self.editing.as_deref() == Some(id) {
            self.project = (!value.is_null()).then(|| value.clone());
            if let Some(base) = self.editing_baseline.as_mut() { base.project = self.project.clone(); }
        }
        self.patch(id, json!({ field: value }), None, cx);
    }
    fn canonical_of(&self, column: &str) -> String { self.canonical(column) }

    pub(super) fn step_task(&mut self, by: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(current) = self.editing.clone() else { return };
        let order = self.nav_order();
        let Some(index) = order.iter().position(|r| r.id == current) else { return };
        let next = index as i32 + by;
        if next < 0 || next as usize >= order.len() { return; }
        let row = order[next as usize].clone();
        self.leave_detail(window, cx);
        self.open_editor(&row, false, window, cx);
    }
    /// Back to the list or board, saving a title or description typed and not yet saved.
    pub(super) fn leave_detail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing_baseline.is_some() && self.draft_dirty_editor(cx) && !self.busy {
            self.keep_open = true;
            self.save(window, cx);
        }
        self.close_editor(window, cx);
    }

    pub(super) fn add_subtask(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(parent) = self.open_row().cloned() else { return };
        let title = self.sub_input.read(cx).value().trim().to_string();
        if title.is_empty() { return; }
        let id = threadbear_core::notes::new_id();
        let position = self.subtasks(&parent.id).iter().map(|r| r.position).fold(crate::now_ms() as f64, f64::max) + 1.;
        let mut row = parent.clone();
        row.id = id.clone();
        row.title = title.clone();
        row.notes = String::new();
        row.status = "todo".into();
        row.column_id = Some("todo".into());
        row.parent_id = Some(parent.id.clone());
        row.priority = None;
        row.due_date = None;
        row.links = vec![];
        row.thread = None;
        row.thread_command_id = None;
        row.thread_error = None;
        row.linear = None;
        row.position = position;
        row.created_at = crate::now_ms() as f64;
        let before = self.rows.clone();
        self.rows.push(row);
        self.sub_input.update(cx, |f, cx| f.set_value("", window, cx));
        cx.notify();
        let args = json!({"id": id, "title": title, "notes": "", "status": "todo", "columnId": "todo", "project": parent.project, "position": position, "parentId": parent.id});
        let typed = title.clone();
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "tasks.create", args).await;
            this.update_in(cx, |this, window, cx| {
                match result {
                    Ok(_) => this.refresh(cx),
                    Err(e) => {
                        this.rows = before;
                        this.sub_input.update(cx, |f, cx| f.set_value(typed, window, cx));
                        this.detail.error = Some(e.message);
                    }
                }
                cx.notify();
            }).ok();
        }).detach();
    }

    fn set_links(&mut self, id: &str, links: Vec<Value>, cx: &mut Context<Self>) {
        self.patch(id, json!({"links": links}), None, cx);
        self.load_images(cx);
    }
    pub(super) fn add_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.open_row().cloned() else { return };
        let mut url = self.link_url.read(cx).value().trim().to_string();
        if url.is_empty() { return; }
        if !url.contains("://") { url = format!("https://{url}"); }
        let title = self.link_title.read(cx).value().trim().to_string();
        let title = if title.is_empty() { url.split("://").nth(1).unwrap_or(&url).trim_end_matches('/').to_string() } else { title };
        let mut links = row.links.clone();
        links.push(json!({"id": threadbear_core::notes::new_id(), "kind": "link", "title": title, "url": url}));
        self.link_url.update(cx, |f, cx| f.set_value("", window, cx));
        self.link_title.update(cx, |f, cx| f.set_value("", window, cx));
        self.detail.adding_link = false;
        self.set_links(&row.id, links, cx);
    }
    /// Search the current list, so a newly refreshed Print can be picked too.
    pub(super) fn matching_prints(&self, cx: &App) -> Vec<(String, String)> {
        let query = self.print_search.read(cx).value().trim().to_lowercase();
        self.app.upgrade().map(|app| {
            let app = app.read(cx);
            if !app.prints_on() { return vec![]; }
            app.prints.list.iter().filter(|p| p.title.to_lowercase().contains(&query))
                .map(|p| (p.id.clone(), p.title.clone())).collect()
        }).unwrap_or_default()
    }

    pub(super) fn pick_print(&mut self, id: &str, title: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.attach_print(id, title, cx);
        self.close_picker(cx);
        window.focus(&self.focus, cx);
        crate::web_panels::keyboard_to_app(window);
    }

    fn print_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let matches = self.matching_prints(cx);
        let mut results = v_flex().id("task-print-results").max_h(px(240.)).overflow_y_scroll().gap(px(2.));
        if matches.is_empty() {
            results = results.child(div().p(px(10.)).text_size(px(13.)).text_color(style::secondary_label(cx)).child("No matching Prints".test_text("No matching Prints")));
        }
        for (id, title) in matches {
            let attached = self.open_row().is_some_and(|row| row.links.iter().any(|link| link["printId"] == id));
            let tip = if attached { format!("Already attached: {title}") } else { format!("Attach {title}") };
            results = results.child(Button::new(SharedString::from(format!("attach-print-{id}"))).label(title.clone())
                .icon(icons::symbol(if attached { "checkmark" } else { "doc.richtext" })).small().ghost().w_full()
                .tooltip(tip).disabled(attached)
                .on_click(cx.listener(move |this, _, window, cx| this.pick_print(&id, &title, window, cx))));
        }
        v_flex().id("task-print-picker").relative().w_full().gap(px(6.)).my(px(6.)).p(px(8.)).rounded(px(8.))
            .child(self.reveal_print_picker())
            .border_1().border_color(style::card_border(cx)).bg(style::card_background(cx))
            .child(h_flex().gap(px(6.))
                .child(div().flex_1().min_w_0().child(Input::new(&self.print_search).small()))
                .child(Button::new("task-print-cancel").label("Cancel").small().ghost().tooltip("Close Print search (Escape)")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close_picker(cx);
                        window.focus(&this.focus, cx);
                        crate::web_panels::keyboard_to_app(window);
                    }))))
            .child(results).into_any_element()
    }

    pub(super) fn attach_print(&mut self, print_id: &str, title: &str, cx: &mut Context<Self>) {
        let Some(row) = self.open_row().cloned() else { return };
        if row.links.iter().any(|l| l["printId"] == print_id) { return; }
        let mut links = row.links.clone();
        links.push(json!({"id": threadbear_core::notes::new_id(), "kind": "print", "title": title, "printId": print_id}));
        self.set_links(&row.id, links, cx);
    }
    fn remove_link(&mut self, link: &str, cx: &mut Context<Self>) {
        let Some(row) = self.open_row().cloned() else { return };
        let links: Vec<Value> = row.links.iter().filter(|l| l["id"] != link).cloned().collect();
        self.set_links(&row.id, links, cx);
    }
    /// A picture onto the open task: read here, where the picker granted it,
    /// and sent as bytes, as the Tasks photo is.
    pub(super) fn attach_image(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        self.attach_images(vec![path], cx);
    }
    /// Several pictures go up one after another in one task, so a drop or
    /// paste of three attaches all three rather than only the first.
    pub(super) fn attach_images(&mut self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        let Some(id) = self.editing.clone() else { return };
        if self.detail.attaching || paths.is_empty() { return; }
        self.detail.attaching = true;
        self.detail.error = None;
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let mut error = None;
            for path in paths {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string());
                let staged = path.clone();
                let image = cx.background_executor().spawn(async move {
                    let read = threadbear_core::local_files::read(&json!({"path":path,"maxBytes":10*1024*1024})).map_err(|e| format!("Could not read that picture: {e}"))?;
                    if read["truncated"] == true { return Err("Choose a picture under 10 MB.".to_string()); }
                    Ok(read["data"].clone())
                }).await;
                // A pasted picture's private copy is only needed until it's read.
                crate::composer::unstage(&staged);
                let result = match image {
                    Ok(base64) => call(client.clone(), "tasks.imageData", json!({"id": id, "base64": base64, "name": name})).await.map_err(|e| e.message),
                    Err(error) => Err(error),
                };
                if let Err(e) = result { error = Some(e); }
            }
            this.update(cx, |this, cx| {
                this.detail.attaching = false;
                this.detail.error = error;
                this.refresh(cx);
                this.load_activity(cx);
                cx.notify();
            }).ok();
        }).detach();
    }
    /// Paste into the description: pictures (a screenshot, files copied in
    /// Finder) attach to the task; words paste as words.
    fn paste_pictures(&mut self, cx: &mut Context<Self>) -> bool {
        let mut pictures: Vec<std::path::PathBuf> = crate::composer_media::read(false).unwrap_or_default();
        if pictures.is_empty() {
            if let Some(item) = cx.read_from_clipboard() {
                let (mut images, mut has_text) = (Vec::new(), false);
                for entry in item.entries() {
                    match entry {
                        ClipboardEntry::ExternalPaths(p) => pictures.extend(p.paths().iter().cloned()),
                        ClipboardEntry::Image(image) => images.push(image.clone()),
                        ClipboardEntry::String(_) => has_text = true,
                    }
                }
                // Rich text from a browser carries an image flavour too: it stays text.
                if pictures.is_empty() && !has_text {
                    pictures = images.iter().filter_map(crate::composer::save_pasted_image).collect();
                }
            }
        }
        let pictures: Vec<_> = pictures.into_iter().filter(|p| crate::composer::is_picture(p)).collect();
        if pictures.is_empty() { return false; }
        self.attach_images(pictures, cx);
        true
    }
    fn choose_image(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: Some("Attach".into()) });
        cx.spawn_in(window, async move |this, cx| {
            match paths.await {
                Ok(Ok(Some(paths))) => { this.update(cx, |this, cx| this.attach_images(paths, cx)).ok(); }
                Ok(Ok(None)) => {}
                _ => { this.update(cx, |this, cx| { this.detail.error = Some("Could not open the picker. Try again, or drop a picture here.".into()); cx.notify(); }).ok(); }
            }
        }).detach();
    }

    pub(super) fn send_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.editing.clone() else { return };
        let body = self.comment_input.read(cx).value().trim().to_string();
        if body.is_empty() { return; }
        let pending = threadbear_core::notes::new_id();
        if let Some(Ok(items)) = self.detail.activity.as_mut() {
            items.push(json!({"id": pending, "taskId": id, "kind": "comment", "body": body, "author": {"kind":"user","name":"You"}, "createdAt": crate::now_ms(), "pending": true}));
        }
        self.comment_input.update(cx, |f, cx| f.set_value("", window, cx));
        self.detail.error = None;
        cx.notify();
        let client = self.client.clone();
        let typed = body.clone();
        cx.spawn_in(window, async move |this, cx| {
            let result = call(client, "tasks.comment", json!({"id": id, "body": body})).await;
            this.update_in(cx, |this, window, cx| {
                if let Err(e) = result {
                    if let Some(Ok(items)) = this.detail.activity.as_mut() { items.retain(|i| i["id"] != pending); }
                    if this.comment_input.read(cx).value().is_empty() { this.comment_input.update(cx, |f, cx| f.set_value(typed, window, cx)); }
                    this.detail.error = Some(e.message);
                } else {
                    this.load_activity(cx);
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn remove_comment(&mut self, comment: String, cx: &mut Context<Self>) {
        let Some(id) = self.editing.clone() else { return };
        let before = self.detail.activity.clone();
        if let Some(Ok(items)) = self.detail.activity.as_mut() { items.retain(|i| i["id"] != comment.as_str()); }
        cx.notify();
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = call(client, "tasks.removeComment", json!({"id": id, "commentId": comment})).await;
            this.update(cx, |this, cx| {
                if let Err(e) = result {
                    this.detail.activity = before;
                    this.detail.error = Some(e.message);
                }
                cx.notify();
            }).ok();
        }).detach();
    }
    fn open_link(&mut self, link: &Value, window: &mut Window, cx: &mut Context<Self>) {
        match link["kind"].as_str().unwrap_or("link") {
            "print" => {
                let id = link["printId"].as_str().unwrap_or("").to_string();
                self.app.update(cx, |app, cx| app.open_print_window(&id, None, cx)).ok();
            }
            "thread" => {
                let id = link["thread"]["id"].as_str().unwrap_or("").to_string();
                self.app.update(cx, |app, cx| { app.close_tasks(cx); app.open_thread_link(&id, window, cx); }).ok();
            }
            _ => {
                if let Some(url) = link["url"].as_str().or(link["imageUrl"].as_str()) { crate::open_gate::open_web(url, cx); }
            }
        }
    }
    fn toggle_picker(&mut self, name: &'static str, cx: &mut Context<Self>) {
        // The press that closed it (outside the card) shouldn't open it again.
        // Print search is inline, so it has no outside-click dismissal to debounce.
        if name != "print" && self.detail.picker_closed_at.is_some_and(|t| t.elapsed().as_millis() < 250) { return; }
        self.detail.picker = if self.detail.picker == Some(name) { None } else { Some(name) };
        if name == "due" {
            let day = self.open_row().and_then(|r| r.due_date.as_deref()).and_then(task_dates::parse).unwrap_or_else(today);
            self.detail.month = (day.year(), day.month());
        }
        cx.notify();
    }

    // MARK: drawing

    fn section_head(&self, title: &str, extra: Option<String>, cx: &App) -> Div {
        h_flex().gap(px(8.)).mt(px(26.)).mb(px(8.)).text_size(px(13.)).font_weight(FontWeight::SEMIBOLD)
            .child(title.to_string())
            .children(extra.map(|e| div().font_weight(FontWeight::NORMAL).text_color(style::tertiary_label(cx)).child(e)))
    }
    fn property(&self, symbol: Icon, label: &str, value: AnyElement, cx: &App) -> AnyElement {
        h_flex().min_h(px(32.)).gap(px(10.))
            .child(div().w(px(16.)).flex_none().flex().justify_center().child(symbol))
            .child(div().w(px(96.)).flex_none().text_size(px(13.)).text_color(style::secondary_label(cx)).child(label.to_string()))
            .child(h_flex().flex_1().min_w_0().child(value))
            .into_any_element()
    }
    fn picker_card(&self, cx: &mut Context<Self>) -> Div {
        v_flex().occlude().min_w(px(220.)).p(px(6.)).rounded(px(10.)).bg(cx.theme().popover).border_1().border_color(cx.theme().border).shadow_lg().text_color(style::label_text(cx))
    }
    fn picker_row(&self, id: String, icon: Option<Icon>, label: String, right: Option<String>, checked: bool, cx: &mut Context<Self>, on: impl Fn(&mut Self, &mut Context<Self>) + 'static) -> AnyElement {
        let hover = style::hover_fill(cx);
        h_flex().id(SharedString::from(id)).role(Role::MenuItem).aria_label(SharedString::from(label.clone())).h(px(28.)).px(px(8.)).gap(px(8.)).rounded(px(6.)).cursor_pointer().text_size(px(13.))
            .hover(move |s| s.bg(hover))
            .on_click(cx.listener(move |this, _, _, cx| {
                on(this, cx);
                this.detail.picker = None;
                cx.notify();
            }))
            .children(icon)
            .child(div().flex_1().child(label.clone().test_text(label)))
            .children(right.map(|r| div().text_color(style::tertiary_label(cx)).child(r)))
            .child(div().w(px(14.)).children(checked.then(|| icons::symbol("checkmark").with_size(px(12.)).text_color(style::accent(cx)))))
            .into_any_element()
    }
    fn priority_picker(&self, row: &TaskRow, cx: &mut Context<Self>) -> AnyElement {
        let mut card = self.picker_card(cx).id("task-priority-picker").on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| this.close_picker(cx)));
        let mut choices: Vec<(Option<&'static str>, &'static str)> = vec![(None, "No priority")];
        choices.extend(task_dates::PRIORITIES.iter().map(|(id, name)| (Some(*id), *name)));
        for (value, name) in choices {
            let id = row.id.clone();
            let checked = row.priority.as_deref() == value;
            card = card.child(self.picker_row(format!("priority-{}", value.unwrap_or("none")), Some(priority_icon(value, 14., cx)), name.to_string(), None, checked, cx, move |this, cx| {
                this.set_field(&id, "priority", value.map_or(Value::Null, |v| json!(v)), cx);
            }));
        }
        card.into_any_element()
    }
    fn close_picker(&mut self, cx: &mut Context<Self>) {
        if self.detail.picker.is_some() {
            self.detail.picker = None;
            self.detail.picker_closed_at = Some(std::time::Instant::now());
            cx.notify();
        }
    }
    fn due_picker(&self, row: &TaskRow, cx: &mut Context<Self>) -> AnyElement {
        let now = today();
        let current = row.due_date.as_deref().and_then(task_dates::parse);
        let mut card = self.picker_card(cx).id("task-due-picker").w(px(260.)).on_mouse_down_out(cx.listener(|this, _: &MouseDownEvent, _, cx| this.close_picker(cx)));
        for (label, day) in task_dates::quick_picks(now) {
            let id = row.id.clone();
            card = card.child(self.picker_row(format!("due-{label}"), None, label.to_string(), Some(task_dates::long_label(day)), current == Some(day), cx, move |this, cx| {
                this.set_field(&id, "dueDate", json!(task_dates::format(day)), cx);
            }));
        }
        let (year, month) = self.detail.month;
        let step = |by: i32| move |this: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
            let (y, m) = this.detail.month;
            this.detail.month = task_dates::shift_month(y, m, by);
            cx.notify();
        };
        let head = h_flex().mt(px(6.)).pt(px(8.)).px(px(4.)).border_t_1().border_color(style::hairline(cx))
            .child(div().flex_1().text_size(px(13.)).font_weight(FontWeight::MEDIUM).child(task_dates::month_title(year, month).test_text(task_dates::month_title(year, month))))
            .child(Button::new("due-prev-month").icon(icons::symbol("chevron.left")).ghost().xsmall().tooltip("Previous month").on_click(cx.listener(step(-1))))
            .child(Button::new("due-next-month").icon(icons::symbol("chevron.right")).ghost().xsmall().tooltip("Next month").on_click(cx.listener(step(1))));
        card = card.child(head);
        let mut days = h_flex().px(px(2.)).mt(px(4.));
        for name in task_dates::WEEKDAYS {
            days = days.child(div().w(px(34.)).text_center().text_size(px(11.)).text_color(style::tertiary_label(cx)).child(name));
        }
        card = card.child(days);
        let (accent, hover) = (style::accent(cx), style::hover_fill(cx));
        for week in task_dates::month_grid(year, month) {
            let mut line = h_flex().px(px(2.));
            for slot in week {
                let Some(day) = slot else { line = line.child(div().w(px(34.)).h(px(28.))); continue };
                let (id, picked, is_today) = (row.id.clone(), current == Some(day), day == now);
                let label = task_dates::long_label(day);
                line = line.child(
                    div().id(SharedString::from(format!("due-day-{}", task_dates::format(day)))).role(Role::Button).aria_label(SharedString::from(label.clone()))
                        .w(px(34.)).h(px(28.)).flex().items_center().justify_center().rounded(px(6.)).cursor_pointer().text_size(px(12.))
                        .tip(label)
                        .when(picked, |d| d.bg(accent).text_color(gpui_kit::white()))
                        .when(!picked && is_today, |d| d.text_color(accent).font_weight(FontWeight::BOLD))
                        .when(!picked, |d| d.hover(move |s| s.bg(hover)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_field(&id, "dueDate", json!(task_dates::format(day)), cx);
                            this.detail.picker = None;
                            cx.notify();
                        }))
                        .child(day.day().to_string()),
                );
            }
            card = card.child(line);
        }
        let id = row.id.clone();
        card = card.child(div().mt(px(6.)).pt(px(4.)).border_t_1().border_color(style::hairline(cx)).child(self.picker_row("due-clear".into(), Some(icons::symbol("xmark").with_size(px(12.)).text_color(style::secondary_label(cx))), "Clear".into(), None, false, cx, move |this, cx| {
            this.set_field(&id, "dueDate", Value::Null, cx);
        })));
        card.into_any_element()
    }
    /// A property's value as a quiet button that opens its picker under it.
    fn picker_value(&self, id: &'static str, name: &'static str, content: AnyElement, tip: &'static str, picker: Option<AnyElement>, cx: &mut Context<Self>) -> AnyElement {
        let hover = style::hover_fill(cx);
        div().relative()
            .child(h_flex().id(id).role(Role::Button).aria_label(tip).self_start().gap(px(6.)).h(px(26.)).px(px(6.)).rounded(px(6.)).cursor_pointer().text_size(px(13.))
                .hover(move |s| s.bg(hover)).tip(tip)
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_picker(name, cx)))
                .child(content))
            .children(picker.map(|card| crate::chips::under(26., card)))
            .into_any_element()
    }

    /// The Linear property: Send or Link while it isn't in Linear, Sending…
    /// (or waiting for approval), the error and Retry, or the issue with its
    /// status, sync and Refresh. With Linear not ready, only a linked issue's
    /// key shows, as a plain link.
    fn linear_property(&self, row: &TaskRow, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tertiary = style::tertiary_label(cx);
        let ready = self.linear_ready;
        let muted = |text: String| div().text_size(px(12.)).text_color(tertiary).child(text);
        let mut line = h_flex().gap(px(6.)).flex_wrap().items_center();
        let linear = row.linear.as_ref();
        if let Some(l) = linear.filter(|l| l.linked()) {
            let key = l.identifier.clone().unwrap_or_default();
            let label = match l.status.as_deref().filter(|s| !s.is_empty()) { Some(s) => format!("{key} · {s}"), None => key.clone() };
            let url = l.url.clone();
            line = line.child(Button::new("task-linear-open").label(label.clone()).small().ghost().tooltip(format!("Open {key} in Linear")).on_click(move |_, _, cx| {
                if let Some(url) = &url { crate::open_gate::open_web(url, cx); }
            }));
            if ready {
                let (id, on) = (row.id.clone(), l.sync);
                line = line.child(Checkbox::new("task-linear-sync").label("Keep in sync").checked(on)
                    .tooltip(if on { "Its status and comments follow Linear. Untick to stop." } else { "Follow Linear's status and comments" })
                    .on_click(cx.listener(move |this, on: &bool, _, cx| this.set_linear_sync(&id, *on, cx))));
                let id = row.id.clone();
                let refreshing = self.detail.linear_refreshing;
                line = line.child(Button::new("task-linear-refresh").label("Refresh").icon(icons::symbol("arrow.clockwise")).small().ghost().loading(refreshing).disabled(refreshing)
                    .tooltip("Read the issue from Linear now").on_click(cx.listener(move |this, _, _, cx| this.refresh_linear(&id, cx))));
            }
            let mut column = v_flex().gap(px(2.)).py(px(4.)).child(line);
            if l.missing {
                column = column.child(div().px(px(6.)).text_size(px(12.)).text_color(style::needs_you_text(cx)).child("Not found in Linear"));
            }
            if ready {
                column = column.child(muted(if l.sync { synced_words(l.synced_at, crate::now_ms()) } else { "Not syncing".into() }).px(px(6.)));
            }
            return Some(column.into_any_element());
        }
        if !ready { return None; }
        match linear.map(|l| l.state.as_str()) {
            Some("creating") => {
                let l = linear.cloned().unwrap_or_default();
                match l.waiting_url {
                    Some(url) => {
                        line = line.child(div().px(px(6.)).text_size(px(13.)).child("Waiting for approval"))
                            .child(Button::new("task-linear-approve").label("Approve").icon(icons::symbol("arrow.up.forward")).small().ghost().tooltip("Approve the Linear call in Connectors")
                                .on_click(move |_, _, cx| { crate::open_gate::open_web(&url, cx); }));
                    }
                    None => line = line.child(crate::spinner::Spinner::new().with_size(px(12.))).child(div().text_size(px(13.)).text_color(style::secondary_label(cx)).child("Sending…".test_text("Linear Sending…"))),
                }
            }
            Some("failed") => {
                let error = linear.and_then(|l| l.error.clone()).unwrap_or_else(|| "Something went wrong".into());
                let retry = row.clone();
                line = line.child(div().px(px(6.)).min_w_0().text_size(px(13.)).text_color(style::failed_text(cx)).child(format!("Couldn't send: {error}")))
                    .child(Button::new("task-linear-retry").label("Retry").small().ghost().tooltip("Open the send sheet again")
                        .on_click(cx.listener(move |this, _, window, cx| this.open_linear_send(&retry, window, cx))));
            }
            _ => {
                let (send, link) = (row.clone(), row.clone());
                let done = row.status == "done";
                line = line.child(Button::new("task-linear-send").label("Send to Linear").icon(icons::symbol("paperplane")).small().ghost().disabled(done)
                        .tooltip(if done { "Reopen it to send it to Linear" } else { "Make a Linear issue from this task" })
                        .on_click(cx.listener(move |this, _, window, cx| this.open_linear_send(&send, window, cx))))
                    .child(Button::new("task-linear-link").label("Link existing…").icon(icons::symbol("link")).small().ghost().tooltip("Link this task to an issue already in Linear")
                        .on_click(cx.listener(move |this, _, window, cx| this.open_linear_link(&link, window, cx))));
            }
        }
        Some(line.into_any_element())
    }

    pub(super) fn detail_view(&self, row: &TaskRow, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let _ = window;
        let page = cx.entity().downgrade();
        let (secondary, tertiary) = (style::secondary_label(cx), style::tertiary_label(cx));
        let order = self.nav_order();
        let index = order.iter().position(|r| r.id == row.id);
        let parent = row.parent_id.as_ref().and_then(|p| self.rows.iter().find(|r| &r.id == p)).cloned();
        let crumb = row.project.as_ref().map(|p| self.project_name(p)).unwrap_or_else(|| "Tasks".into());

        // The bar: Back, where it is, previous and next, Delete and the rest.
        let mut bar = h_flex().gap(px(6.)).mb(px(18.)).w_full().max_w(px(760.)).mx_auto()
            .child(Button::new("task-back").icon(icons::symbol("chevron.left")).label("Back").small().ghost().tooltip("Back to the tasks (Esc)").on_click(cx.listener(|this, _, window, cx| this.leave_detail(window, cx))))
            .child(div().text_size(px(13.)).text_color(tertiary).child(crumb));
        if let Some(parent) = parent.clone() {
            bar = bar.child(div().text_color(tertiary).child("›")).child(
                Button::new("task-parent").label(parent.title.clone()).small().ghost().tooltip("Open the task this is part of").on_click(cx.listener(move |this, _, window, cx| {
                    this.leave_detail(window, cx);
                    this.open_editor(&parent, false, window, cx);
                })),
            );
        }
        bar = bar.child(div().text_color(tertiary).child("›")).child(div().min_w_0().truncate().text_size(px(13.)).text_color(secondary).child(row.title.clone().test_text(row.title.clone())))
            .child(div().flex_1())
            .children(index.map(|i| div().text_size(px(12.)).text_color(tertiary).child(format!("{} of {}", i + 1, order.len()))))
            .child(Button::new("task-prev").icon(icons::symbol("chevron.up")).small().ghost().tooltip("Previous task (K)").disabled(!index.is_some_and(|i| i > 0)).on_click(cx.listener(|this, _, window, cx| this.step_task(-1, window, cx))))
            .child(Button::new("task-next").icon(icons::symbol("chevron.down")).small().ghost().tooltip("Next task (J)").disabled(!index.is_some_and(|i| i + 1 < order.len())).on_click(cx.listener(|this, _, window, cx| this.step_task(1, window, cx))))
            .child({
                let fav = row.clone();
                Button::new("task-favorite").icon(keys::favorite_icon(row.favorite, 14., cx)).small().ghost()
                    .tooltip(if row.favorite { "Remove from Favorites (⌥F)" } else { "Add to Favorites (⌥F)" })
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_favorite(&fav, cx)))
            })
            .child({ let id = row.id.clone(); Button::new("task-delete").label("Delete").small().ghost().tooltip("Delete the task. You can undo it.").on_click(cx.listener(move |this, _, window, cx| this.ask_delete(id.clone(), window, cx))) })
            .child(self.more_menu(row, cx));

        let mut body = v_flex().w_full().max_w(px(760.)).mx_auto().pb(px(60.));
        body = body.child(div().font_weight(FontWeight::SEMIBOLD).child(Input::new(&self.title).appearance(false).disabled(self.busy).text_size(px(22.)).line_height(px(30.)).h(px(36.)).px_0()));
        if let Some(error) = self.error.clone().or(self.detail.error.clone()) {
            body = body.child(div().mt(px(6.)).text_size(px(12.)).text_color(style::failed_text(cx)).child(error.clone().test_text(error)));
        }

        // Properties.
        let statuses = self.board_order();
        let labels: Vec<String> = statuses.iter().map(|s| self.column_name(s)).collect();
        let (p, sid, current) = (page.clone(), row.id.clone(), column_id(row).to_string());
        let status = Button::new("task-status").label(self.column_name(column_id(row))).icon(status_icon(&self.canonical(column_id(row)), cx)).small().ghost().tooltip("Change the status").dropdown_menu(move |menu: PopupMenu, _, cx| {
            let mut menu = menu;
            for (i, s) in statuses.iter().enumerate() {
                let (p, s, id) = (p.clone(), s.clone(), sid.clone());
                menu = menu.item(PopupMenuItem::new(labels[i].clone()).icon(status_icon(&s, cx)).checked(current == s).on_click(move |_, _, cx| {
                    p.update(cx, |p, cx| p.set_field(&id, "status", json!(s), cx)).ok();
                }));
            }
            menu
        });
        let priority = self.picker_value("task-priority", "priority",
            h_flex().gap(px(6.)).child(priority_icon(row.priority.as_deref(), 14., cx)).child(task_dates::priority_name(row.priority.as_deref()).test_text(format!("Priority {}", task_dates::priority_name(row.priority.as_deref())))).into_any_element(),
            "Set the priority", (self.detail.picker == Some("priority")).then(|| self.priority_picker(row, cx)), cx);
        let due_words = row.due_date.as_deref().and_then(task_dates::parse).map(task_dates::long_label);
        let due_color = row.due_date.as_deref().and_then(task_dates::parse).map(|d| match task_dates::due_state(d, today()) { Due::Overdue => urgent_red(), Due::Today => Hsla::from(rgb(0xd9870b)), Due::Later => style::label_text(cx) }).unwrap_or(tertiary);
        let due = self.picker_value("task-due", "due",
            div().text_color(due_color).child(due_words.clone().unwrap_or("No due date".into()).test_text(format!("Due date {}", due_words.unwrap_or("none".into())))).into_any_element(),
            "Set the due date", (self.detail.picker == Some("due")).then(|| self.due_picker(row, cx)), cx);
        let (p, pid, selected, choices) = (page.clone(), row.id.clone(), row.project.clone().unwrap_or(Value::Null), self.project_choices());
        let project = Button::new("task-project").label(self.project_name(&selected)).small().ghost().tooltip("Move it to a project").dropdown_menu(move |menu: PopupMenu, _, _| {
            let mut menu = menu;
            for (label, value) in &choices {
                let (p, value, id) = (p.clone(), value.clone(), pid.clone());
                menu = menu.item(PopupMenuItem::new(label.clone()).checked(value == selected).on_click(move |_, _, cx| {
                    p.update(cx, |p, cx| p.set_field(&id, "project", value.clone(), cx)).ok();
                }));
            }
            menu
        });
        let by = row.created_by.as_ref();
        let by_agent = by.is_some_and(|b| b["kind"] == "agent");
        let by_name = by.and_then(|b| b["name"].as_str()).unwrap_or("You").to_string();
        let created = Local.timestamp_millis_opt(row.created_at as i64).single().map(|t| t.format("%b %-d").to_string()).unwrap_or_default();
        let creator = h_flex().gap(px(6.)).px(px(6.)).text_size(px(13.)).child(by_name.clone().test_text(format!("Created by {by_name}"))).child(div().text_color(tertiary).child(if by_agent { format!("agent, {created}") } else { created }));
        let starting = row.thread_command_id.is_some() && row.thread.is_none() && row.thread_error.is_none();
        let send_label = if row.thread.is_some() { "View Thread" } else if row.thread_error.is_some() { "Thread Failed" } else if starting { "Starting Thread…" } else { "Send to Thread" };
        let send_row = row.clone();
        let thread = h_flex().gap(px(6.))
            .child(Button::new("task-send-thread").label(send_label).icon(icons::symbol("bubble.left")).small().ghost().tooltip(if row.thread.is_some() { "Open the thread working on this task" } else { "Start a thread with its title, description and project" }).disabled(self.busy || starting).loading(starting).on_click(cx.listener(move |this, _, window, cx| {
                if send_row.thread.is_some() || send_row.thread_command_id.is_some() || send_row.thread_error.is_some() { this.send_task(&send_row, window, cx); return; }
                if this.draft_dirty_editor(cx) { this.send_after_save = true; this.keep_open = true; this.save(window, cx); } else { this.change("tasks.startThread", json!({"id": send_row.id}), cx); }
            })))
            .children(row.thread_error.clone().map(|e| div().text_size(px(12.)).text_color(style::failed_text(cx)).child(e)));
        let link_count = row.links.len();
        let links_value = h_flex().gap(px(6.)).child(div().px(px(6.)).text_size(px(13.)).text_color(if link_count == 0 { tertiary } else { style::label_text(cx) }).child(match link_count { 0 => "None".to_string(), 1 => "1 link".into(), n => format!("{n} links") }))
            .child(Button::new("task-links-add").icon(icons::symbol("plus")).xsmall().ghost().tooltip("Add a link").on_click(cx.listener(|this, _, window, cx| {
                this.detail.adding_link = true;
                this.link_url.update(cx, |f, cx| f.focus(window, cx));
                crate::web_panels::keyboard_to_app(window);
                cx.notify();
            })));
        let props = v_flex().id("task-properties").mt(px(14.)).py(px(6.)).px(px(10.)).rounded(px(10.)).border_1().border_color(style::card_border(cx)).bg(style::card_background(cx))
            .child(self.property(status_icon(&self.canonical(column_id(row)), cx), "Status", status.into_any_element(), cx))
            .child(self.property(priority_icon(row.priority.as_deref(), 14., cx), "Priority", priority, cx))
            .child(self.property(icons::symbol("calendar").with_size(px(14.)).text_color(tertiary), "Due date", due, cx))
            .child(self.property(icons::symbol("square.stack").with_size(px(14.)).text_color(tertiary), "Project", project.into_any_element(), cx))
            .child(self.property(icons::symbol(if by_agent { "sparkle" } else { "person.crop.circle" }).with_size(px(14.)).text_color(tertiary), "Created by", creator.into_any_element(), cx))
            .child(self.property(icons::symbol("bubble.left").with_size(px(14.)).text_color(tertiary), "Threads", thread.into_any_element(), cx))
            .child(self.property(icons::symbol("link").with_size(px(14.)).text_color(tertiary), "Links", links_value.into_any_element(), cx));
        let props = match self.linear_property(row, cx) {
            Some(value) => props.child(self.property(crate::linear_send::linear_mark(14., tertiary), "Linear", value, cx)),
            None => props,
        };
        body = body.child(props);

        // Description.
        // Pictures pasted or dropped here attach to the task (as Attach image
        // does) and show as previews under the words.
        let accent_soft = style::accent(cx).opacity(0.08);
        let mut previews = h_flex().id("task-description-images").gap(px(8.)).flex_wrap().mt(px(6.));
        let mut preview_count = 0;
        for link in row.links.iter().filter(|l| l["kind"] == "image") {
            let Some(url) = link["imageUrl"].as_str() else { continue };
            preview_count += 1;
            let lid = link["id"].as_str().unwrap_or("").to_string();
            let title = link["title"].as_str().filter(|t| !t.is_empty()).unwrap_or("Image").to_string();
            let (open, remove) = (link.clone(), lid.clone());
            let picture = match self.detail.images.get(url).cloned().flatten() {
                Some(image) => img(ImageSource::Image(image)).h(px(120.)).max_w(px(220.)).rounded(px(6.)).object_fit(ObjectFit::Cover).into_any_element(),
                None => div().h(px(120.)).w(px(160.)).rounded(px(6.)).bg(style::card_background(cx)).flex().items_center().justify_center()
                    .child(icons::symbol("photo").with_size(px(18.)).text_color(tertiary)).into_any_element(),
            };
            previews = previews.child(
                div().id(SharedString::from(format!("desc-image-{lid}"))).group("desc-image").relative().rounded(px(6.)).border_1().border_color(style::card_border(cx)).overflow_hidden().cursor_pointer()
                    .role(Role::Link).aria_label(SharedString::from(format!("Open {title}"))).tip(format!("Open {title}"))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_link(&open, window, cx)))
                    .child(picture)
                    .child(div().absolute().top(px(4.)).right(px(4.)).invisible().group_hover("desc-image", |s| s.visible())
                        .child(Button::new(SharedString::from(format!("desc-image-remove-{lid}"))).icon(icons::symbol("xmark")).xsmall().tooltip(format!("Remove {title}"))
                            .on_click(cx.listener(move |this, _, _, cx| { cx.stop_propagation(); this.remove_link(&remove, cx); })))),
            );
        }
        if self.detail.attaching {
            previews = previews.child(div().h(px(120.)).w(px(160.)).rounded(px(6.)).border_1().border_color(style::card_border(cx)).flex().items_center().justify_center()
                .child(div().text_size(px(12.)).text_color(tertiary).child("Adding picture…")));
            preview_count += 1;
        }
        body = body.child(self.section_head("Description", None, cx))
            .child(div().id("task-description").px(px(2.)).rounded(px(8.))
                .drag_over::<ExternalPaths>(move |s, _, _, _| s.bg(accent_soft))
                .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                    let pictures: Vec<_> = paths.paths().iter().filter(|p| crate::composer::is_picture(p)).cloned().collect();
                    this.attach_images(pictures, cx);
                }))
                .capture_action(cx.listener(|this, _: &gpui_kit::base::input::Paste, _, cx| {
                    if this.paste_pictures(cx) { cx.stop_propagation(); }
                }))
                .child(Textarea::new(&self.notes).appearance(false).disabled(self.busy))
                .when(preview_count > 0, |d| d.child(previews)));

        // Sub-tasks (one level: a sub-task has none of its own).
        if row.parent_id.is_none() {
            let subs = self.subtasks(&row.id);
            let closed = subs.iter().filter(|s| s.status == "done").count();
            let mut list = v_flex();
            for sub in &subs {
                let (check, open) = (sub.clone(), sub.clone());
                let done = sub.status == "done";
                let hover = style::hover_fill(cx);
                list = list.child(
                    h_flex().id(SharedString::from(format!("sub-{}", sub.id))).gap(px(10.)).px(px(6.)).py(px(6.)).border_b_1().border_color(style::hairline(cx)).hover(move |s| s.bg(hover))
                        .child(Checkbox::new(SharedString::from(format!("sub-check-{}", sub.id))).checked(done).accessibility_label(format!("Mark {} {}", sub.title, if done { "not done" } else { "done" })).tooltip(if done { "Mark not done" } else { "Mark done" })
                            .on_click(cx.listener(move |this, _: &bool, _, cx| this.complete(&check, cx))))
                        .child(h_flex().id(SharedString::from(format!("sub-open-{}", sub.id))).role(Role::Button).aria_label(SharedString::from(format!("Open {}", sub.title))).flex_1().min_w_0().cursor_pointer().tip("Open the sub-task")
                            .on_click(cx.listener(move |this, _, window, cx| { this.leave_detail(window, cx); this.open_editor(&open, false, window, cx); }))
                            .child(div().min_w_0().truncate().text_size(px(14.)).when(done, |d| d.line_through().text_color(tertiary)).child(sub.title.clone().test_text(sub.title.clone()))))
                        .children(sub.due_date.as_deref().and_then(|d| due_chip(d, sub.due_time.as_deref(), cx)))
                        .when(sub.priority.is_some(), |d| d.child(priority_icon(sub.priority.as_deref(), 14., cx))),
                );
            }
            if self.detail.adding_sub {
                list = list.child(h_flex().gap(px(10.)).px(px(6.)).py(px(4.)).child(div().size(px(16.)).rounded(px(4.)).border_1().border_color(style::hairline(cx))).child(div().flex_1().child(Input::new(&self.sub_input).appearance(false))));
            }
            list = list.child(Button::new("task-add-sub").label("Add sub-task").icon(icons::symbol("plus")).small().ghost().tooltip("Add a sub-task. Return adds it.").on_click(cx.listener(|this, _, window, cx| {
                if this.detail.adding_sub && !this.sub_input.read(cx).value().trim().is_empty() { this.add_subtask(window, cx); return; }
                this.detail.adding_sub = true;
                this.sub_input.update(cx, |f, cx| f.focus(window, cx));
                crate::web_panels::keyboard_to_app(window);
                cx.notify();
            })));
            body = body.child(self.section_head("Sub-tasks", (!subs.is_empty()).then(|| format!("{closed}/{} closed", subs.len())), cx)).child(list);
        }

        // Links and attachments.
        let attach_print = Button::new("task-attach-print").label("Attach Print").icon(icons::symbol("doc.richtext")).small().ghost().tooltip("Search your Prints and attach one").on_click(cx.listener(|this, _, window, cx| {
            this.toggle_picker("print", cx);
            if this.detail.picker == Some("print") {
                this.print_search.update(cx, |field, cx| {
                    field.set_value("", window, cx);
                    field.focus(window, cx);
                });
                crate::web_panels::keyboard_to_app(window);
                this.detail.reveal_picker.set(true);
            }
        }));
        let actions = h_flex().gap(px(4.)).flex_wrap()
            .child(Button::new("task-add-link").label("Add link").icon(icons::symbol("link")).small().ghost().tooltip("Add a link with an optional title").on_click(cx.listener(|this, _, window, cx| {
                this.detail.adding_link = !this.detail.adding_link;
                if this.detail.adding_link { this.link_url.update(cx, |f, cx| f.focus(window, cx)); crate::web_panels::keyboard_to_app(window); }
                cx.notify();
            })))
            .child(attach_print)
            .child(Button::new("task-attach-image").label("Attach image").icon(icons::symbol("photo")).small().ghost().loading(self.detail.attaching).disabled(self.detail.attaching).tooltip("Attach a picture. You can drop one here too.").on_click(cx.listener(|this, _, window, cx| this.choose_image(window, cx))));
        let accent_soft = style::accent(cx).opacity(0.08);
        let mut links = v_flex().id("task-links").rounded(px(8.)).drag_over::<ExternalPaths>(move |s, _, _, _| s.bg(accent_soft))
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| this.attach_images(paths.paths().to_vec(), cx)))
            .child(actions);
        if self.detail.picker == Some("print") {
            links = links.child(self.print_picker(cx));
        }
        if self.detail.adding_link {
            links = links.child(h_flex().gap(px(8.)).my(px(6.)).p(px(8.)).rounded(px(8.)).border_1().border_color(style::card_border(cx)).bg(style::card_background(cx))
                .child(div().flex_1().child(Input::new(&self.link_url).small()))
                .child(div().w(px(180.)).child(Input::new(&self.link_title).small()))
                .child(Button::new("task-link-save").label("Add").small().primary().tooltip("Add the link (Return)").on_click(cx.listener(|this, _, window, cx| this.add_link(window, cx))))
                .child(Button::new("task-link-cancel").label("Cancel").small().ghost().tooltip("Don't add a link").on_click(cx.listener(|this, _, _, cx| { this.detail.adding_link = false; cx.notify(); }))));
        }
        for link in &row.links {
            let kind = link["kind"].as_str().unwrap_or("link").to_string();
            let title = link["title"].as_str().filter(|t| !t.is_empty()).or(link["url"].as_str()).unwrap_or("Link").to_string();
            let detail = match kind.as_str() { "print" => "Print".to_string(), "thread" => "Thread".into(), "image" => "Image".into(), _ => link["url"].as_str().unwrap_or("").split("://").nth(1).unwrap_or("").split('/').next().unwrap_or("").to_string() };
            let lid = link["id"].as_str().unwrap_or("").to_string();
            let (open, remove) = (link.clone(), lid.clone());
            let thumb = link["imageUrl"].as_str().and_then(|u| self.detail.images.get(u).cloned().flatten());
            let hover = style::hover_fill(cx);
            links = links.child(
                h_flex().id(SharedString::from(format!("link-{lid}"))).group("task-link").gap(px(10.)).px(px(6.)).py(px(6.)).rounded(px(6.)).cursor_pointer().hover(move |s| s.bg(hover))
                    .role(Role::Link).aria_label(SharedString::from(format!("Open {title}"))).tip(format!("Open {title}"))
                    .on_click(cx.listener(move |this, _, window, cx| this.open_link(&open, window, cx)))
                    .child(match thumb {
                        Some(image) => img(ImageSource::Image(image)).w(px(48.)).h(px(36.)).rounded(px(4.)).object_fit(ObjectFit::Cover).into_any_element(),
                        None => icons::symbol(link_symbol(&kind)).with_size(px(14.)).text_color(secondary).into_any_element(),
                    })
                    .child(div().flex_1().min_w_0().truncate().text_size(px(14.)).child(title.clone().test_text(title.clone())))
                    .child(div().text_size(px(12.)).text_color(tertiary).child(detail))
                    .child(div().invisible().group_hover("task-link", |s| s.visible()).child(Button::new(SharedString::from(format!("link-remove-{lid}"))).icon(icons::symbol("xmark")).xsmall().ghost().tooltip(format!("Remove {title}")).on_click(cx.listener(move |this, _, _, cx| { cx.stop_propagation(); this.remove_link(&remove, cx); })))),
            );
        }
        body = body.child(self.section_head("Links and attachments", (link_count > 0).then(|| link_count.to_string()), cx)).child(links);

        // From the project.
        if let Some(project) = row.project.as_ref() {
            let mut from = v_flex();
            match &self.detail.related {
                None => from = from.child(div().text_size(px(13.)).text_color(tertiary).child("Loading…")),
                Some(Err(e)) => from = from.child(div().text_size(px(13.)).text_color(tertiary).child(format!("This runner can't show the project's work yet: {e}"))),
                Some(Ok(related)) => {
                    let threads = related["threads"].as_array().cloned().unwrap_or_default();
                    let prints = related["prints"].as_array().cloned().unwrap_or_default();
                    if threads.is_empty() && prints.is_empty() { from = from.child(div().text_size(px(13.)).text_color(tertiary).child("Nothing from this project yet.")); }
                    let hover = style::hover_fill(cx);
                    for t in threads.iter().map(|t| (t, true)).chain(prints.iter().map(|p| (p, false))) {
                        let (item, is_thread) = t;
                        let id = item["id"].as_str().unwrap_or("").to_string();
                        let title = item["title"].as_str().unwrap_or("Untitled").to_string();
                        let trailing = if is_thread { item["status"].as_str().unwrap_or("").to_string() } else { item["kind"].as_str().unwrap_or("print").to_string() };
                        from = from.child(
                            h_flex().id(SharedString::from(format!("related-{id}"))).gap(px(10.)).px(px(6.)).py(px(6.)).rounded(px(6.)).cursor_pointer().hover(move |s| s.bg(hover))
                                .role(Role::Link).aria_label(SharedString::from(format!("Open {title}"))).tip(if is_thread { "Open the thread" } else { "Open the print" })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let id = id.clone();
                                    if is_thread { this.app.update(cx, |app, cx| { app.close_tasks(cx); app.open_thread_link(&id, window, cx); }).ok(); }
                                    else { this.app.update(cx, |app, cx| app.open_print_window(&id, None, cx)).ok(); }
                                }))
                                .child(icons::symbol(if is_thread { "bubble.left" } else { "doc.richtext" }).with_size(px(14.)).text_color(secondary))
                                .child(div().flex_1().min_w_0().truncate().text_size(px(14.)).child(title.clone().test_text(title)))
                                .child(div().text_size(px(12.)).text_color(tertiary).child(trailing)),
                        );
                    }
                }
            }
            body = body.child(self.section_head(&format!("From {}", self.project_name(project)), None, cx)).child(from);
        }

        // Activity, oldest first, and the comment box under it.
        let comments_only = self.detail.comments_only;
        let mut filter = h_flex().gap(px(2.)).p(px(2.)).rounded(px(7.)).bg(style::hover_fill(cx));
        for (label, only) in [("All", false), ("Comments", true)] {
            filter = filter.child(Button::new(SharedString::from(format!("activity-{label}"))).label(label).xsmall().ghost().selected(comments_only == only).tooltip(if only { "Show only comments" } else { "Show everything that happened" }).on_click(cx.listener(move |this, _, _, cx| { this.detail.comments_only = only; cx.notify(); })));
        }
        let mut activity = v_flex().gap(px(2.));
        match &self.detail.activity {
            None => activity = activity.child(div().text_size(px(13.)).text_color(tertiary).child("Loading…")),
            Some(Err(e)) => activity = activity.child(div().text_size(px(13.)).text_color(tertiary).child(format!("This runner can't show activity yet: {e}"))),
            Some(Ok(items)) => {
                let now = crate::now_ms();
                let shown: Vec<&Value> = items.iter().filter(|i| !comments_only || i["kind"] == "comment").collect();
                if shown.is_empty() { activity = activity.child(div().text_size(px(13.)).text_color(tertiary).child(if comments_only { "No comments yet." } else { "Nothing yet." })); }
                for item in shown {
                    let author = item["author"]["name"].as_str().unwrap_or("Someone").to_string();
                    let when = threadbear_core::inbox::ago(item["createdAt"].as_f64().unwrap_or(0.) as i64, now);
                    let body_text = item["body"].as_str().unwrap_or("").to_string();
                    if item["kind"] == "comment" {
                        let cid = item["id"].as_str().unwrap_or("").to_string();
                        // Copied from Linear: read-only here (remove it in Linear).
                        let from_linear = item["source"] == "linear";
                        let mine = item["author"]["kind"] != "agent" && !from_linear;
                        let pending = item["pending"] == true;
                        let face = if from_linear { crate::linear_send::linear_mark(13., secondary) } else { icons::symbol(if mine { "person.crop.circle" } else { "sparkle" }).with_size(px(13.)).text_color(secondary) };
                        activity = activity.child(
                            v_flex().id(SharedString::from(format!("comment-{cid}"))).group("task-comment").my(px(4.)).p(px(10.)).rounded(px(8.)).border_1().border_color(style::card_border(cx)).bg(style::card_background(cx)).when(pending, |d| d.opacity(0.6))
                                .child(h_flex().gap(px(6.)).text_size(px(12.))
                                    .child(face)
                                    .child(div().font_weight(FontWeight::SEMIBOLD).child(author))
                                    .child(div().text_color(tertiary).child(when))
                                    .when(from_linear, |d| d.child(div().text_color(tertiary).child("from Linear".test_text("Comment from Linear"))))
                                    .child(div().flex_1())
                                    .when(mine && !pending, |d| d.child(div().invisible().group_hover("task-comment", |s| s.visible()).child(Button::new(SharedString::from(format!("comment-remove-{cid}"))).icon(icons::symbol("trash")).xsmall().ghost().tooltip("Delete this comment").on_click(cx.listener(move |this, _, _, cx| this.remove_comment(cid.clone(), cx)))))))
                                .child(div().mt(px(4.)).text_size(px(14.)).child(body_text.clone().test_text(body_text))),
                        );
                    } else {
                        activity = activity.child(
                            h_flex().gap(px(8.)).px(px(4.)).py(px(4.)).text_size(px(12.)).text_color(secondary)
                                .child(div().size(px(6.)).rounded_full().bg(tertiary))
                                .child(div().font_weight(FontWeight::MEDIUM).child(author))
                                .child(div().min_w_0().child(body_text))
                                .child(div().text_color(tertiary).child(when)),
                        );
                    }
                }
            }
        }
        let comments = row.comment_count.unwrap_or(0);
        body = body.child(self.section_head("Activity", (comments > 0).then(|| if comments == 1 { "1 comment".into() } else { format!("{comments} comments") }), cx).child(div().flex_1()).child(filter))
            .child(activity)
            .child(h_flex().id("task-comment-box").items_end().gap(px(8.)).mt(px(10.)).p(px(9.)).pl(px(12.)).rounded(px(10.)).border_1().border_color(style::card_border(cx)).bg(style::card_background(cx))
                .child(div().flex_1().text_size(px(14.)).child(Textarea::new(&self.comment_input).appearance(false)))
                .child(Button::new("task-comment-send").icon(icons::symbol("arrow.up")).small().primary().tooltip("Comment (Return)").on_click(cx.listener(|this, _, window, cx| this.send_comment(window, cx)))));

        v_flex().id("task-detail").w_full().flex_1().min_h_0()
            .child(bar)
            .child(div().id("task-detail-scroll").flex_1().min_h_0().overflow_y_scroll().track_scroll(&self.detail_scroll).child(body))
            .into_any_element()
    }
}
use threadbear_core::chrono::TimeZone as _;
