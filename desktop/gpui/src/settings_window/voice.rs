// Settings › Voice & Audio (the Swift app's VoiceSettings.swift): Use Voice,
// the button's icon, Command Mode, the key the shortcuts are built on and
// which of them are on, the voice, its speed and language, and the
// microphone. Only while the voiceMode experiment is on. All of it is this
// Mac's own, in desktop-voice.json (voice/mod.rs), under the Swift app's
// UserDefaults names.
use super::{row_with, section, SettingsWindow};
use crate::style;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::*;
use gpui_kit::*;
use std::rc::Rc;
use threadbear_core::voice::{self as rules, VoiceIcon, VoicePrefs};
use threadbear_core::voice_keys::{VoiceCommandKey, VoiceTriggerKey};

type Edit = Rc<dyn Fn(&mut VoicePrefs)>;

fn label(text: impl Into<SharedString>) -> AnyElement {
    div().child(text.into()).into_any_element()
}

impl SettingsWindow {
    pub(super) fn render_voice(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let voice = crate::voice::get(&self.app, cx);
        if voice.read(cx).available(cx) {
            voice.update(cx, |v, cx| v.load_microphones(cx));
        }
        let v = voice.read(cx);
        let prefs = v.prefs.clone();
        let keys = prefs.keys();
        let available = v.available(cx);
        let mics = v.microphones.clone();
        let preview = v.preview_status.clone();

        let switch = |this: &SettingsWindow,
                      id: &'static str,
                      on: bool,
                      tip: String,
                      apply: fn(&mut VoicePrefs, bool),
                      cx: &mut Context<SettingsWindow>| {
            let voice = voice.clone();
            this.switch_owned(id.to_owned(), on, tip, cx, move |_, on, cx| {
                voice.update(cx, |v, cx| v.set_prefs(|p| apply(p, on), cx));
            })
        };

        let mut first = vec![row_with(label("Use Voice"), switch(self, "settings-voice-enabled", prefs.enabled(), "Show the microphone button beside Send to talk with threads out loud, and turn on Voice's keyboard shortcuts".to_owned(), |p, on| p.enabled = Some(on), cx), cx)];
        if !available && prefs.enabled() {
            first.push(row_with(
                label("This build has no Voice: it was built without the macOS 26 SDK."),
                div().into_any_element(),
                cx,
            ));
        }
        first.push(row_with(
            label("Pause before sending"),
            self.voice_picker(
                "settings-voice-pause",
                rules::pause_title(prefs.pause()),
                "How long you can pause mid-sentence before what you said goes to the thread",
                rules::PAUSES
                    .iter()
                    .map(|&s| {
                        (
                            rules::pause_title(s),
                            s == prefs.pause(),
                            Rc::new(move |p: &mut VoicePrefs| p.pause = Some(s)) as Edit,
                        )
                    })
                    .collect(),
                &voice,
                cx,
            ),
            cx,
        ));
        first.push(row_with(
            label("Button icon"),
            self.voice_picker(
                "settings-voice-icon",
                prefs.icon().title().to_owned(),
                "The icon on the Voice button beside Send",
                VoiceIcon::ALL
                    .iter()
                    .map(|&i| {
                        (
                            i.title().to_owned(),
                            i == prefs.icon(),
                            Rc::new(move |p: &mut VoicePrefs| p.icon = Some(i.name().to_owned()))
                                as Edit,
                        )
                    })
                    .collect(),
                &voice,
                cx,
            ),
            cx,
        ));
        first.push(row_with(label("Command Mode"), switch(self, "settings-voice-command-mode", keys.command_mode, format!("Hold {} and talk: when you let go, what you said goes to the open thread as your message", keys.command_display()), |p, on| p.command_mode = Some(on), cx), cx));
        let voice_section = section(
            "settings-voice",
            "Voice",
            "Talk with a thread out loud: what you say goes to the thread as your message, and its replies are read back to you. Speech is turned into text on this Mac, and pocket-tts reads replies on this Mac too. It downloads about 240 MB the first time.",
            "explain how Voice works, or turn it on or off for you.",
            first,
            cx,
        );

        let mut key_rows = vec![row_with(
            label("Key"),
            self.voice_picker(
                "settings-voice-key",
                keys.trigger.title().to_owned(),
                "The key Voice's shortcuts are built on",
                VoiceTriggerKey::ALL
                    .iter()
                    .map(|&k| {
                        (
                            k.title().to_owned(),
                            k == keys.trigger,
                            Rc::new(move |p: &mut VoicePrefs| {
                                p.key_trigger = Some(k.name().to_owned())
                            }) as Edit,
                        )
                    })
                    .collect(),
                &voice,
                cx,
            ),
            cx,
        )];
        key_rows.push(row_with(
            label(format!(
                "{} to dictate, let go to type it in",
                keys.hold_display()
            )),
            switch(
                self,
                "settings-voice-hold",
                keys.hold_to_dictate,
                "Hold the key and talk; let go and what you said is typed where the cursor is"
                    .to_owned(),
                |p, on| p.key_hold = Some(on),
                cx,
            ),
            cx,
        ));
        key_rows.push(row_with(
            label(format!("{} for hands-free dictation", keys.shift_display())),
            switch(
                self,
                "settings-voice-shift",
                keys.shift_toggle,
                "Starts and stops dictation that types in at each pause".to_owned(),
                |p, on| p.key_shift = Some(on),
                cx,
            ),
            cx,
        ));
        key_rows.push(row_with(
            label(format!(
                "{} for hands-free dictation",
                keys.hands_free_display()
            )),
            switch(
                self,
                "settings-voice-space",
                keys.hands_free_with_space,
                "Starts and stops dictation that types in at each pause".to_owned(),
                |p, on| p.key_hands_free = Some(on),
                cx,
            ),
            cx,
        ));
        key_rows.push(row_with(label(format!("{} (double tap) for hands-free dictation", keys.double_tap_display())), switch(self, "settings-voice-double-tap", keys.double_tap, "Two quick taps start hands-free dictation. Off by default, as macOS uses a double tap of Fn too".to_owned(), |p, on| p.key_double_tap = Some(on), cx), cx));
        if keys.command_mode {
            key_rows.push(row_with(
                label(format!("Command Mode: {}", keys.command_display())),
                self.voice_picker(
                    "settings-voice-command-key",
                    keys.command.title().to_owned(),
                    "The key held with the trigger for Command Mode",
                    VoiceCommandKey::ALL
                        .iter()
                        .map(|&k| {
                            (
                                k.title().to_owned(),
                                k == keys.command,
                                Rc::new(move |p: &mut VoicePrefs| {
                                    p.key_command = Some(k.name().to_owned())
                                }) as Edit,
                            )
                        })
                        .collect(),
                    &voice,
                    cx,
                ),
                cx,
            ));
        }
        if keys.trigger == VoiceTriggerKey::Fn {
            // The note over its button, as it's too long to share a row.
            let open = self.button(
                "settings-voice-keyboard",
                "Open Keyboard Settings",
                "Open System Settings > Keyboard, where Press 🌐 key to is",
                cx,
                |_, _, cx| crate::open_gate::open_keyboard_settings(cx),
            );
            key_rows.push(
                gpui_kit::component::v_flex()
                    .px(crate::style::px(10.))
                    .py(crate::style::px(8.))
                    .gap(crate::style::px(8.))
                    .items_start()
                    .text_size(crate::style::px(13.))
                    .line_height(crate::style::px(17.))
                    .child(div().text_color(style::secondary_text(cx)).child("macOS also uses Fn, so a tap or double tap can open emoji. To use it for Voice alone, set Press 🌐 key to Do Nothing in System Settings > Keyboard."))
                    .child(open)
                    .into_any_element(),
            );
        }
        let keys_section = section("settings-voice-keys", "Keys", "Voice's keyboard shortcuts work anywhere in Threadbear while Use Voice is on. Esc cancels what's listening, with nothing typed or sent.", "change which keys start dictation.", key_rows, cx);

        let current = prefs.voice();
        let speaking = vec![
            row_with(
                label("Voice"),
                self.voice_picker(
                    "settings-voice-voice",
                    current.name.clone(),
                    "Who the agent sounds like",
                    rules::voices()
                        .into_iter()
                        .map(|o| {
                            let id = o.id;
                            (
                                o.name.clone(),
                                o.id == current.id,
                                Rc::new(move |p: &mut VoicePrefs| p.voice = Some(id.to_owned()))
                                    as Edit,
                            )
                        })
                        .collect(),
                    &voice,
                    cx,
                ),
                cx,
            ),
            row_with(
                label("Speed"),
                self.voice_picker(
                    "settings-voice-speed",
                    rules::speed_title(prefs.speed()),
                    "How fast replies are read",
                    rules::SPEEDS
                        .iter()
                        .map(|&s| {
                            (
                                rules::speed_title(s),
                                s == prefs.speed(),
                                Rc::new(move |p: &mut VoicePrefs| p.speed = Some(s)) as Edit,
                            )
                        })
                        .collect(),
                    &voice,
                    cx,
                ),
                cx,
            ),
            row_with(
                label("Language"),
                self.voice_picker(
                    "settings-voice-language",
                    prefs.language().name.to_owned(),
                    "The language you speak and replies are read in",
                    rules::LANGUAGES
                        .iter()
                        .map(|l| {
                            let id = l.id;
                            (
                                l.name.to_owned(),
                                l.id == prefs.language().id,
                                Rc::new(move |p: &mut VoicePrefs| p.language = Some(id.to_owned()))
                                    as Edit,
                            )
                        })
                        .collect(),
                    &voice,
                    cx,
                ),
                cx,
            ),
            row_with(
                label(preview.unwrap_or_default()),
                {
                    let voice = voice.clone();
                    self.button(
                        "settings-voice-sample",
                        "Play a Sample",
                        "Hear this voice at this speed",
                        cx,
                        move |_, _, cx| voice.update(cx, |v, cx| v.preview(cx)),
                    )
                },
                cx,
            ),
        ];
        let speaking_section = section(
            "settings-voice-speaking",
            "Speaking",
            "How replies are read out in Voice mode, by pocket-tts on this Mac.",
            "pick a voice or speed for you.",
            speaking,
            cx,
        );

        let chosen = prefs
            .microphone()
            .and_then(|uid| mics.iter().find(|(u, _)| u == uid))
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| "System Default".to_owned());
        let mut choices: Vec<(String, bool, Edit)> = vec![(
            "System Default".to_owned(),
            prefs.microphone().is_none(),
            Rc::new(|p: &mut VoicePrefs| p.microphone = None),
        )];
        for (uid, name) in mics {
            let selected = prefs.microphone() == Some(uid.as_str());
            choices.push((
                name,
                selected,
                Rc::new(move |p: &mut VoicePrefs| p.microphone = Some(uid.clone())),
            ));
        }
        let system = section(
            "settings-voice-system",
            "System",
            "The microphone Voice and dictation listen on.",
            "change the microphone for you.",
            vec![row_with(
                label("Microphone"),
                self.voice_picker(
                    "settings-voice-microphone",
                    chosen,
                    "The microphone Voice listens on",
                    choices,
                    &voice,
                    cx,
                ),
                cx,
            )],
            cx,
        );

        vec![voice_section, keys_section, speaking_section, system]
    }

    /// A pop-up menu of Voice's choices, each a change to what's saved.
    fn voice_picker(
        &self,
        id: &'static str,
        chosen: String,
        tip: &'static str,
        choices: Vec<(String, bool, Edit)>,
        voice: &Entity<crate::voice::Voice>,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        let voice = voice.clone();
        div()
            .id(SharedString::from(format!("{id}-holder")))
            .flex_none()
            .child(
                Button::new(id)
                    .label(chosen)
                    .small()
                    .secondary()
                    .dropdown_caret(true)
                    .tooltip(tip)
                    .dropdown_menu(move |menu: PopupMenu, _, _| {
                        let mut menu = menu;
                        for (title, checked, edit) in &choices {
                            let (voice, edit) = (voice.clone(), edit.clone());
                            menu = menu.item(
                                PopupMenuItem::new(title.clone())
                                    .checked(*checked)
                                    .on_click(move |_, _, cx| {
                                        let edit = edit.clone();
                                        voice.update(cx, |v, cx| v.set_prefs(|p| edit(p), cx));
                                    }),
                            );
                        }
                        menu
                    }),
            )
            .into_any_element()
    }
}
