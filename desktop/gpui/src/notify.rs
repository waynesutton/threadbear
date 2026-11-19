// Notifications for threads that come to need Mike while he's elsewhere, and
// the Dock badge's count of the threads waiting on him: Swift's Notifier and
// DockBadge. Clicking a notification brings the app forward on that thread.
//
// A scripted app (the automation build the specs drive) shows neither, as
// the Swift app's dev and screenshot runs don't: they'd land on Mike's
// screen, and in his Dock, from apps he isn't using.
use crate::app::Threadbear;
use gpui_kit::*;
use threadbear_core::inbox::{self, ThreadSummary};

pub(crate) fn enabled() -> bool {
    !cfg!(feature = "automation")
}

/// Posts one for `thread` (`Notifier.post`). gpui asks macOS for permission
/// the first time, and posts nothing outside an app bundle (a `cargo run`).
pub(crate) fn post(thread: &ThreadSummary, cx: &App) {
    // A handover's incoming app leaves them to the one that's showing, so
    // nothing comes twice while both are running.
    if !enabled() || crate::handover::waiting() {
        return;
    }
    let (title, body) = inbox::notification_words(thread);
    cx.show_system_notification(SystemNotification { tag: inbox::notification_tag(thread).into(), title: title.into(), body: body.into(), actions: vec![] });
}

/// A task's due time came and this Mac's runner claimed its reminder
/// (runner task-reminders.ts, `tasks.due`). Its tag starts `task:`, so a
/// click opens the task, never a thread.
pub(crate) fn post_task_due(params: &serde_json::Value, cx: &App) {
    if !enabled() || crate::handover::waiting() {
        return;
    }
    let (Some(id), Some(title)) = (params["id"].as_str(), params["title"].as_str()) else { return };
    let when = params["when"].as_str().unwrap_or("now");
    cx.show_system_notification(SystemNotification {
        tag: format!("task:{id}:{}T{}", params["dueDate"].as_str().unwrap_or(""), params["dueTime"].as_str().unwrap_or("")).into(),
        title: title.to_string().into(),
        body: format!("Task due {when}").into(),
        actions: vec![],
    });
}
fn task_in_tag(tag: &str) -> Option<&str> {
    tag.strip_prefix("task:")?.split(':').next().filter(|id| !id.is_empty())
}

/// A click on one opens its thread in the main window, once the list has
/// loaded if the click is what started the app (as a link does).
pub(crate) fn install(app: WeakEntity<Threadbear>, window: AnyWindowHandle, cx: &mut App) {
    if !enabled() {
        return;
    }
    cx.on_system_notification_response(move |response, cx| {
        if let Some(task) = task_in_tag(&response.tag).map(str::to_owned) {
            cx.activate(true);
            let app = app.clone();
            window.update(cx, |_, window, cx| { app.update(cx, |this, cx| this.open_task(task, window, cx)).ok(); }).ok();
            return;
        }
        let Some(id) = inbox::thread_in_tag(&response.tag).map(str::to_owned) else { return };
        cx.activate(true);
        let app = app.clone();
        window
            .update(cx, |_, window, cx| {
                app.update(cx, |this, cx| this.open_from_outside(id, window, cx)).ok();
            })
            .ok();
    });
}

/// The badge: how many threads need him, nothing when none do
/// (`refreshDockBadge`). Only set when it changes.
pub(crate) fn set_badge(label: Option<&str>) {
    if !enabled() {
        return;
    }
    platform::set_badge(label);
}

#[cfg(target_os = "macos")]
mod platform {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::NSString;

    pub(super) fn set_badge(label: Option<&str>) {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let tile = NSApplication::sharedApplication(mtm).dockTile();
        tile.setBadgeLabel(label.map(NSString::from_str).as_deref());
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    // Windows' taskbar overlay and Linux's launcher counts are each their
    // own thing; none yet.
    pub(super) fn set_badge(_label: Option<&str>) {}
}
