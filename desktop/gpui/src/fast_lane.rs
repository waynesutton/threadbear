// Fast lane icon button beside Send (Option D): icon-only lightning bolt,
// visible when the `fastLane` experiment is on. Off sits quiet beside Send;
// Armed lights up in the accent fill with an outer halo ring and pairs with
// composer border, dimmed coding chips, placeholder, and accent Send cues.
use crate::style;
use crate::style::px;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::{h_flex, Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use serde_json::{json, Value};
use std::time::Duration;

/// Experiment id checked against `Threadbear::experiment_on`.
pub(crate) const EXPERIMENT: &str = "fastLane";

/// Composer placeholder while Fast lane is armed for the next turn.
pub(crate) const PLACEHOLDER: &str = "Fast web search...";

/// Filled lightning bolt (256x256 viewBox, tinted via `text_color`).
const BOLT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path fill="black" d="M215.79,118.17a8,8,0,0,0-5-5.66L153.18,90.9l14.66-73.33a8,8,0,0,0-13.69-7l-112,120a8,8,0,0,0,3,13l57.63,21.61L88.16,238.43a8,8,0,0,0,13.69,7l112-120A8,8,0,0,0,215.79,118.17Z"/></svg>"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ButtonState {
    Off,
    Armed,
    Sending,
}

/// The send button with the icon-only Fast lane bolt directly to its left.
pub(crate) fn beside_send(
    send: AnyElement,
    state: ButtonState,
    menu: Option<(Choices, WeakEntity<crate::app::Threadbear>)>,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut App,
) -> AnyElement {
    let side = px(style::size::SEND_BUTTON);
    let accent = style::accent(cx);
    let bolt_color = match state {
        ButtonState::Off => style::secondary_text(cx),
        ButtonState::Armed | ButtonState::Sending => gpui_kit::white(),
    };
    let bolt = Icon::default()
        .data(BOLT_SVG.as_bytes())
        .with_size(side * 0.54)
        .text_color(bolt_color);

    let (aria, tip) = match state {
        ButtonState::Off => (
            "Fast web search",
            if cfg!(target_os = "macos") {
                "Fast web search (click or Option + Enter, right-click to choose who answers)"
            } else {
                "Fast web search (click or Alt + Enter, right-click to choose who answers)"
            },
        ),
        ButtonState::Armed => ("Fast web search on", "Fast web search on (click to cancel)"),
        ButtonState::Sending => ("Fast web search running", "Fast web search running"),
    };

    let bg = match state {
        ButtonState::Off => style::primary_opacity(cx, 0.1),
        ButtonState::Armed | ButtonState::Sending => accent,
    };
    let hover_bg = match state {
        ButtonState::Off => style::primary_opacity(cx, 0.16),
        ButtonState::Armed | ButtonState::Sending => accent,
    };

    let circle = div()
        .id("composer-fast-lane")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(side)
        .rounded_full()
        .bg(bg)
        .when(state != ButtonState::Sending, |d| {
            d.cursor_pointer().hover(move |s| s.bg(hover_bg))
        })
        .child(bolt)
        .test_support()
        .role(Role::Button)
        .aria_label(aria)
        .tip_leftward(tip)
        .when(state != ButtonState::Sending, |d| d.on_click(on_toggle));

    // Outer halo ring drawn outside the circle so button spacing never shifts.
    let halo: Option<AnyElement> = match state {
        ButtonState::Off => None,
        ButtonState::Armed => Some(halo_ring(accent.opacity(0.32)).into_any_element()),
        ButtonState::Sending if cx.reduce_motion() => {
            Some(halo_ring(accent.opacity(0.36)).into_any_element())
        }
        ButtonState::Sending => Some(
            halo_ring(accent.opacity(0.24))
                .with_animation(
                    "fast-lane-pulse",
                    Animation::new(Duration::from_millis(1200)).repeat(),
                    move |_, t| {
                        let wave = 0.5 - 0.5 * (t * std::f32::consts::TAU).cos();
                        halo_ring(accent.opacity(0.18 + 0.34 * wave))
                    },
                )
                .into_any_element(),
        ),
    };

    let circle = match menu {
        Some((choices, app)) => div()
            .id("composer-fast-lane-menu")
            .flex_none()
            .child(circle)
            .context_menu(move |menu, _, _| choice_menu(menu, &choices, &app))
            .into_any_element(),
        None => circle.into_any_element(),
    };
    let wrapped = div().relative().flex_none().children(halo).child(circle);
    h_flex()
        .gap(px(6.))
        .items_center()
        .child(wrapped)
        .child(send)
        .into_any_element()
}

/// Who answers Fast lane, as the runner's settings list them
/// (`Settings.fastLane`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Choices {
    picked: String,
    all: Vec<(String, String, String)>,
}

impl Choices {
    /// None from a runner from before there was a choice.
    pub(crate) fn read(settings: Option<&Value>) -> Option<Self> {
        let lane = &settings?["fastLane"];
        let all: Vec<_> = lane["choices"]
            .as_array()?
            .iter()
            .filter_map(|c| {
                Some((
                    c["id"].as_str()?.to_string(),
                    c["title"].as_str()?.to_string(),
                    c["detail"].as_str().unwrap_or_default().to_string(),
                ))
            })
            .collect();
        (!all.is_empty()).then(|| Self {
            picked: lane["model"].as_str().unwrap_or_default().to_string(),
            all,
        })
    }
}

/// The bolt's right-click menu. The pick is the runner's setting, so it holds
/// for every thread until it's changed.
fn choice_menu(
    mut menu: PopupMenu,
    choices: &Choices,
    app: &WeakEntity<crate::app::Threadbear>,
) -> PopupMenu {
    for (id, title, detail) in &choices.all {
        let (app, id) = (app.clone(), id.clone());
        let label = if detail.is_empty() {
            title.clone()
        } else {
            format!("{title}: {detail}")
        };
        menu = menu.item(
            PopupMenuItem::new(label)
                .checked(id == choices.picked)
                .on_click(move |_, _, cx| {
                    let change = json!({ "fastLane": { "model": id } });
                    app.update(cx, |app, cx| app.update_runner_settings(change, cx))
                        .ok();
                }),
        );
    }
    menu
}

fn halo_ring(color: Hsla) -> Div {
    div()
        .absolute()
        .top(px(-3.))
        .right(px(-3.))
        .bottom(px(-3.))
        .left(px(-3.))
        .rounded_full()
        .border(px(2.5))
        .border_color(color)
}

#[cfg(test)]
mod tests {
    use super::Choices;
    use serde_json::json;

    #[test]
    fn choices_come_from_the_runner_and_an_older_one_has_none() {
        assert_eq!(Choices::read(None), None);
        assert_eq!(Choices::read(Some(&json!({}))), None);
        let got = Choices::read(Some(&json!({ "fastLane": { "model": "hy4", "choices": [
            { "id": "exa", "title": "Exa", "detail": "Quickest" },
            { "id": "hy4", "title": "Hy4 Flash Free" },
        ] } })))
        .unwrap();
        assert_eq!(got.picked, "hy4");
        assert_eq!(got.all.len(), 2);
    }
}
