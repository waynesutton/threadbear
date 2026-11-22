// Voice's button beside Send (the Swift app's VoiceButton and VoiceCircle):
// off, it's Send's quiet grey circle with the icon picked in Settings > Voice,
// so it doesn't pull the eye; on, it's red with five bars that follow whoever
// is talking, and pulse while it gets ready or the agent works. And the pill
// over the message box while Fn dictation listens (KeyDictationHUD).
use super::keys::DictationPhase;
use super::mode::{Phase, NEW_THREAD};
use crate::a11y::A11yRole as _;
use crate::app::Threadbear;
use crate::style;
use crate::style::px;
use crate::tips::Tip as _;
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{h_flex, ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::time::Duration;

fn red() -> Hsla {
    rgb(0xff3b30).into()
}

/// The send button with Voice's to its left, while Voice is on for this Mac.
/// `thread` is None for the new-thread page, where the first words start one.
pub(crate) fn beside_send(
    send: AnyElement,
    thread: Option<&str>,
    app: &WeakEntity<Threadbear>,
    cx: &mut App,
) -> AnyElement {
    let voice = super::get(app, cx);
    if !voice.read(cx).available(cx) {
        return send;
    }
    let thread = thread.unwrap_or(NEW_THREAD).to_owned();
    let v = voice.read(cx);
    let on = v.mode.is_on_for(&thread);
    let level = if on { v.mode.level } else { 0.0 };
    let busy = on && v.mode.busy();
    let failure = v.mode.failure_for(&thread).map(str::to_owned);
    let icon = v.prefs.icon();
    let side = px(style::size::SEND_BUTTON);
    let circle = div()
        .id("composer-voice")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(side)
        .rounded_full()
        .cursor_pointer();
    let face = if on {
        circle.bg(red()).child(bars(level, busy, side * 0.5))
    } else {
        circle.bg(style::primary_opacity(cx, 0.1)).child(
            Icon::default()
                .data(icon.svg().as_bytes())
                .with_size(side * 0.54)
                .text_color(style::secondary_text(cx)),
        )
    };
    let (toggle, dismiss) = (voice.clone(), voice.clone());
    let id = thread;
    let button = face
        .test_support()
        .role(Role::Button)
        .aria_label(if on { "End voice" } else { "Start voice" })
        .tip(if on {
            "End voice"
        } else {
            "Talk with this thread out loud"
        })
        .on_click(move |_, _, cx| toggle.update(cx, |v, cx| v.toggle_mode(&id, cx)));
    let bubble = failure.map(|message| {
        div()
            .id("composer-voice-failure")
            .test_support()
            .role(Role::Button)
            .aria_label(message.clone())
            .absolute()
            .bottom(side + px(8.))
            .right(px(0.))
            .w(px(280.))
            .p(px(10.))
            .rounded(px(8.))
            .bg(cx.theme().popover)
            .border_1()
            .border_color(style::hairline(cx))
            .shadow_md()
            .text_size(px(12.))
            .cursor_pointer()
            .child(message)
            .tip("Close")
            .on_click(move |_, _, cx| dismiss.update(cx, |v, cx| v.dismiss_mode_failure(cx)))
    });
    h_flex()
        .gap(px(6.))
        .items_center()
        .child(div().relative().child(button).children(bubble))
        .child(send)
        .into_any_element()
}

/// Five bars: following the sound level while someone talks, gently
/// pulsing while busy and quiet.
fn bars(level: f32, busy: bool, size: Pixels) -> AnyElement {
    const SHAPE: [f32; 5] = [0.45, 0.75, 1.0, 0.75, 0.45];
    let width = size * 0.12;
    let row = move |heights: [f32; 5]| {
        h_flex()
            .h(size)
            .gap(size * 0.1)
            .items_center()
            .children(heights.into_iter().map(move |h| {
                div()
                    .w(width)
                    .h(size * h)
                    .rounded_full()
                    .bg(gpui_kit::white())
            }))
    };
    if busy && level < 0.05 {
        return row([0.3; 5])
            .with_animation(
                "voice-pulse",
                Animation::new(Duration::from_millis(1600)).repeat(),
                move |_, t| {
                    let heights = std::array::from_fn(|i| {
                        0.3 + 0.25
                            * (0.5 - 0.5 * ((t + i as f32 * 0.125) * std::f32::consts::TAU).cos())
                    });
                    row(heights)
                },
            )
            .into_any_element();
    }
    row(SHAPE.map(|s| (s * (0.25 + level)).clamp(0.2, 1.0))).into_any_element()
}

/// The pill over the message box while Fn dictation is on: what it's doing,
/// the words so far, and how to finish or cancel. Takes no clicks.
/// The same pill shows Voice mode on this box's thread too, in place of the
/// Swift app's bear: what it's doing, and what you're saying as you say it.
/// `thread` is the box's (None on the new-thread page).
pub(crate) fn pill(thread: Option<&str>, cx: &mut App) -> Option<AnyElement> {
    let voice = super::existing(cx)?;
    let v = voice.read(cx);
    let d = &v.keys;
    let (line, failed, heard, busy, level) = if d.phase != DictationPhase::Off {
        let keys = v.prefs.keys();
        let line = match &d.phase {
            DictationPhase::Off => String::new(),
            DictationPhase::Starting(status) => status.clone(),
            DictationPhase::Finishing => if d.mode == threadbear_core::voice_keys::KeyMode::Command
            {
                "Sending…"
            } else {
                "Typing it in…"
            }
            .to_owned(),
            DictationPhase::Failed(message) => message.clone(),
            // The end of what he's saying, where it's growing.
            DictationPhase::Hearing(words) => tail(words, 70),
            DictationPhase::Listening => keys.listening_line(d.mode),
        };
        (
            line,
            matches!(d.phase, DictationPhase::Failed(_)),
            matches!(d.phase, DictationPhase::Hearing(_)),
            matches!(
                d.phase,
                DictationPhase::Starting(_) | DictationPhase::Finishing
            ),
            d.level,
        )
    } else if v.mode.is_on_for(thread.unwrap_or(NEW_THREAD)) {
        let line = match &v.mode.phase {
            Phase::Starting(status) => status.clone(),
            Phase::Listening => "Listening. Talk, then pause to send it.".to_owned(),
            Phase::Hearing(words) => tail(words, 70),
            Phase::Thinking => "Sent. It's working on it; talk any time.".to_owned(),
            Phase::Speaking => "Reading the reply. Talk to stop it.".to_owned(),
            Phase::Off | Phase::Failed(_) => return None,
        };
        (
            line,
            false,
            matches!(v.mode.phase, Phase::Hearing(_)),
            v.mode.busy(),
            v.mode.level,
        )
    } else {
        return None;
    };
    let dot = div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(24.))
        .rounded_full()
        .bg(if failed { rgb(0xff9500).into() } else { red() });
    let dot = if failed {
        dot.child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight::BOLD)
                .text_color(gpui_kit::white())
                .child("!"),
        )
    } else {
        dot.child(bars(level, busy, px(12.)))
    };
    Some(
        div()
            .absolute()
            .bottom_full()
            .left_0()
            .right_0()
            .mb(px(10.))
            .flex()
            .justify_center()
            .child(
                h_flex()
                    .id("voice-dictation-pill")
                    .gap(px(8.))
                    .items_center()
                    .pl(px(6.))
                    .pr(px(14.))
                    .py(px(6.))
                    .max_w(px(460.))
                    .rounded_full()
                    .bg(cx.theme().popover)
                    .border_1()
                    .border_color(style::hairline(cx))
                    .shadow_md()
                    .child(dot)
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(12.))
                            .text_color(if heard {
                                cx.theme().foreground
                            } else {
                                style::secondary_text(cx)
                            })
                            .child(line),
                    ),
            )
            .into_any_element(),
    )
}

fn tail(words: &str, most: usize) -> String {
    let count = words.chars().count();
    if count <= most {
        return words.to_owned();
    }
    format!("…{}", words.chars().skip(count - most).collect::<String>())
}
