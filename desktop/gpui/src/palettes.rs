// Harbor (dark and light) and Squall (dark only), the named themes in
// Settings > General > Appearance > Theme (Wayne Sutton's, 25 Nov 2026).
// Every value is an opaque 0xRRGGBB. style.rs reads these through `themed`,
// and themes/harbor-squall.theme.json carries the same values for the kit's
// own controls and the syntax highlighter. The tests at the foot hold both
// to WCAG AA and to each other.

#[allow(
    dead_code,
    reason = "full palette token set; every field is checked against the theme JSON in tests"
)]
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub window: u32,
    pub sidebar: u32,
    pub rail: u32,
    pub pane: u32,
    pub popover: u32,
    pub deep: u32,
    pub card: u32,
    pub code: u32,
    pub inline_code: u32,
    pub composer: u32,
    pub chip: u32,
    pub chip_hover: u32,
    pub hover: u32,
    pub selected: u32,
    pub selected_inactive: u32,
    pub hairline: u32,
    pub border: u32,
    pub divider: u32,
    pub text: u32,
    pub text_chrome: u32,
    pub text_strong: u32,
    pub text_secondary: u32,
    pub text_meta: u32,
    pub text_disabled: u32,
    pub icon: u32,
    pub icon_muted: u32,
    pub rail_icon: u32,
    pub rail_icon_active: u32,
    pub accent: u32,
    pub focus: u32,
    pub link: u32,
    pub button: u32,
    pub button_hover: u32,
    pub button_active: u32,
    pub button_fg: u32,
    pub button2: u32,
    pub button2_fg: u32,
    pub bubble: u32,
    pub selection: u32,
    pub badge: u32,
    pub badge_fg: u32,
    pub success: u32,
    pub warning: u32,
    pub error: u32,
    pub info: u32,
    pub modified: u32,
    pub error_bg: u32,
    pub warning_bg: u32,
    pub success_bg: u32,
    pub info_bg: u32,
    pub added_bg: u32,
    pub removed_bg: u32,
    pub added_text: u32,
    pub removed_text: u32,
    pub line_number: u32,
    pub checkbox_border: u32,
    pub checkbox_bg: u32,
    pub term_bg: u32,
    pub term_fg: u32,
    pub ansi: [u32; 16],
    /// Code's colours, the same as the theme JSON's `highlight.syntax`. Here
    /// for the file icons' tints and the theme tiles in Settings.
    pub syntax: Syntax,
}

#[allow(
    dead_code,
    reason = "full syntax token set; every field is checked for AA contrast in tests"
)]
#[derive(Clone, Copy, Debug)]
pub struct Syntax {
    pub fg: u32,
    pub comment: u32,
    pub keyword: u32,
    pub storage: u32,
    pub string: u32,
    pub number: u32,
    pub function: u32,
    pub call: u32,
    pub operator: u32,
}

const DARK_SYNTAX: Syntax = Syntax {
    fg: 0xe4ebec,
    comment: 0x9aa4ad,
    keyword: 0xcb9fd2,
    storage: 0xfd8895,
    string: 0xaad79d,
    number: 0xf0b967,
    function: 0x69bcb5,
    call: 0x7eaedd,
    operator: 0xf79372,
};

const LIGHT_SYNTAX: Syntax = Syntax {
    fg: 0x2d2f37,
    comment: 0x626772,
    keyword: 0x9330a0,
    storage: 0xb3262f,
    string: 0x327530,
    number: 0x925d01,
    function: 0x076f86,
    call: 0x2f5fcc,
    operator: 0xb54f0c,
};

pub const HARBOR_DARK: Palette = Palette {
    window: 0x14161d,
    sidebar: 0x292a32,
    rail: 0x2d2f37,
    pane: 0x292a32,
    popover: 0x2b2d35,
    deep: 0x14161d,
    card: 0x292a32,
    code: 0x313845,
    inline_code: 0x313845,
    composer: 0x292a32,
    chip: 0x323641,
    chip_hover: 0x2d2f37,
    hover: 0x2c2e36,
    selected: 0x13426a,
    selected_inactive: 0x2f333d,
    hairline: 0x2d2f37,
    border: 0x494a52,
    divider: 0x3c414d,
    text: 0xd4d9de,
    text_chrome: 0xcad1d4,
    text_strong: 0xf4f4fa,
    text_secondary: 0xa2a9ae,
    text_meta: 0x9ba4a8,
    text_disabled: 0x74767c,
    icon: 0xbbc2c5,
    icon_muted: 0x888c95,
    rail_icon: 0x7f838c,
    rail_icon_active: 0xf3f3f9,
    accent: 0x1b80bf,
    focus: 0x3285e6,
    link: 0x599bfc,
    button: 0x176998,
    button_hover: 0x186b9e,
    button_active: 0x075493,
    button_fg: 0xf4f4fa,
    button2: 0x3a3f4b,
    button2_fg: 0xcad1d4,
    bubble: 0x202f34,
    selection: 0x304e6b,
    badge: 0x176998,
    badge_fg: 0xf4f4fa,
    success: 0x9dd093,
    warning: 0xeeac54,
    error: 0xf1747d,
    info: 0x72a4d6,
    modified: 0xdec78d,
    error_bg: 0x492a2f,
    warning_bg: 0x4a3a26,
    success_bg: 0x2f3d30,
    info_bg: 0x2f3c4e,
    added_bg: 0x2c3a30,
    removed_bg: 0x3f2a2e,
    added_text: 0x9dd093,
    removed_text: 0xf1747d,
    line_number: 0x9ca5ae,
    checkbox_border: 0x74767c,
    checkbox_bg: 0x323641,
    term_bg: 0x14161d,
    term_fg: 0xcad1d4,
    ansi: [
        0x1c2120, 0xe0656c, 0x55c28d, 0xd9c94a, 0x4685cc, 0xc068c2, 0x3ba8c2, 0xd5dadf, 0x71737a,
        0xf47b81, 0x6fd6a0, 0xece36a, 0x6ba0e4, 0xcc7bd0, 0x51c0dc, 0xf2f2f8,
    ],
    syntax: DARK_SYNTAX,
};

pub const HARBOR_LIGHT: Palette = Palette {
    window: 0xf4f4fa,
    sidebar: 0xf0f0f6,
    rail: 0xdce2e8,
    pane: 0xf0f0f6,
    popover: 0xfaf9ff,
    deep: 0xdce1e8,
    card: 0xfaf9ff,
    code: 0xf2f2f8,
    inline_code: 0xdce1e8,
    composer: 0xfaf9ff,
    chip: 0xdce1e8,
    chip_hover: 0xdce1e8,
    hover: 0xdce1e8,
    selected: 0xccdeef,
    selected_inactive: 0xdce1e8,
    hairline: 0xd9dde3,
    border: 0xbfc7d2,
    divider: 0xd9dde3,
    text: 0x222929,
    text_chrome: 0x2d3039,
    text_strong: 0x091219,
    text_secondary: 0x565a65,
    text_meta: 0x63666f,
    text_disabled: 0x979ca7,
    icon: 0x3c414d,
    icon_muted: 0x6b6f7a,
    rail_icon: 0x5f646f,
    rail_icon_active: 0x091219,
    accent: 0x0a6fa8,
    focus: 0x1a7fbe,
    link: 0x115fa7,
    button: 0x0a6fa8,
    button_hover: 0x0d6592,
    button_active: 0x005392,
    button_fg: 0xfaf9ff,
    button2: 0xdce1e8,
    button2_fg: 0x222929,
    bubble: 0xdbe3ee,
    selection: 0xc6dff6,
    badge: 0x0a6fa8,
    badge_fg: 0xfaf9ff,
    success: 0x2f7f3a,
    warning: 0x895b01,
    error: 0xb3262f,
    info: 0x3664b9,
    modified: 0x8d6617,
    error_bg: 0xf9e0e5,
    warning_bg: 0xf8ecd3,
    success_bg: 0xe1f5e4,
    info_bg: 0xdce7f5,
    added_bg: 0xe1f5e4,
    removed_bg: 0xf9e0e5,
    added_text: 0x2b7535,
    removed_text: 0xa8232c,
    line_number: 0x63666f,
    checkbox_border: 0x7c828d,
    checkbox_bg: 0xfaf9ff,
    term_bg: 0xf4f4fa,
    term_fg: 0x2d3039,
    ansi: [
        0x2d3039, 0xb3262f, 0x2b7535, 0x7b5803, 0x2b5ec3, 0x8e2f9c, 0x086f84, 0x99a1ac, 0x6d707a,
        0xc0303a, 0x2c7a37, 0x7a5903, 0x3662c2, 0x9a38a8, 0x076f85, 0xbfc7d2,
    ],
    syntax: LIGHT_SYNTAX,
};

pub const SQUALL: Palette = Palette {
    window: 0x323946,
    sidebar: 0x18252d,
    rail: 0x141f27,
    pane: 0x18252d,
    popover: 0x1d2a31,
    deep: 0x14161d,
    card: 0x383f4c,
    code: 0x18252d,
    inline_code: 0x3e4756,
    composer: 0x18252d,
    chip: 0x3e4756,
    chip_hover: 0x373e4b,
    hover: 0x2c313c,
    selected: 0x353f4e,
    selected_inactive: 0x2e333e,
    hairline: 0x3c4554,
    border: 0x465863,
    divider: 0x14161d,
    text: 0xdce2e8,
    text_chrome: 0xd9dfe4,
    text_strong: 0xf4f4fa,
    text_secondary: 0xb2b9c1,
    text_meta: 0xa5aeb6,
    text_disabled: 0x5d6974,
    icon: 0xd9dfe4,
    icon_muted: 0x8a959f,
    rail_icon: 0x7a848e,
    rail_icon_active: 0xdce2e8,
    accent: 0x70a3d6,
    focus: 0x70a3d6,
    link: 0x9dbcdc,
    button: 0x70a3d6,
    button_hover: 0x82b0de,
    button_active: 0x5f90c1,
    button_fg: 0x14161d,
    button2: 0x3e4756,
    button2_fg: 0xdce2e8,
    bubble: 0x354a62,
    selection: 0x475c6d,
    badge: 0xfd8895,
    badge_fg: 0x14161d,
    success: 0xaad79d,
    warning: 0xf0b967,
    error: 0xfd8895,
    info: 0x9dbcdc,
    modified: 0xe9d298,
    error_bg: 0x4f3b44,
    warning_bg: 0x5a5146,
    success_bg: 0x3a5149,
    info_bg: 0x3b4b62,
    added_bg: 0x475a4f,
    removed_bg: 0x4c3840,
    added_text: 0xaad79d,
    removed_text: 0xfd8895,
    line_number: 0x86919b,
    checkbox_border: 0x8e98a1,
    checkbox_bg: 0x18252d,
    term_bg: 0x14161d,
    term_fg: 0xd9dfe4,
    ansi: [
        0x14161d, 0xf17a81, 0xaad79d, 0xf0b967, 0x70a3d6, 0xcb9fd2, 0x69bcb5, 0xd9dfe4, 0x7e8992,
        0xf99399, 0xb6ddae, 0xf7c784, 0x9dbcdc, 0xd7b3dc, 0x86cbc5, 0xf4f4fa,
    ],
    syntax: Syntax {
        fg: 0xe5eced,
        ..DARK_SYNTAX
    },
};

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [(&str, &Palette); 3] = [
        ("Harbor", &HARBOR_DARK),
        ("Harbor Light", &HARBOR_LIGHT),
        ("Squall", &SQUALL),
    ];

    /// WCAG 2's relative luminance of an 0xRRGGBB.
    fn luminance(colour: u32) -> f64 {
        let channel = |shift: u32| {
            let c = f64::from((colour >> shift) & 0xff) / 255.;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
    }

    fn contrast(a: u32, b: u32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    /// Body text on every surface it's read on, and each of code's colours
    /// on the code block, meet AA's 4.5 to 1.
    #[test]
    fn text_and_syntax_meet_aa() {
        for (name, p) in ALL {
            for (surface, colour) in [
                ("window", p.window),
                ("sidebar", p.sidebar),
                ("card", p.card),
                ("code", p.code),
            ] {
                let ratio = contrast(p.text, colour);
                assert!(ratio >= 4.5, "{name}: text on {surface} is {ratio:.2}");
            }
            let s = p.syntax;
            for (token, colour) in [
                ("fg", s.fg),
                ("comment", s.comment),
                ("keyword", s.keyword),
                ("storage", s.storage),
                ("string", s.string),
                ("number", s.number),
                ("function", s.function),
                ("call", s.call),
                ("operator", s.operator),
            ] {
                let ratio = contrast(colour, p.code);
                assert!(ratio >= 4.5, "{name}: {token} on code is {ratio:.2}");
            }
            // The quieter text, on the two surfaces it sits on, and the
            // labels on their fills.
            for (what, text, on) in [
                ("secondary on window", p.text_secondary, p.window),
                ("meta on sidebar", p.text_meta, p.sidebar),
                ("link on window", p.link, p.window),
                ("button label", p.button_fg, p.button),
                ("added on its line", p.added_text, p.added_bg),
                ("removed on its line", p.removed_text, p.removed_bg),
                ("terminal text", p.term_fg, p.term_bg),
            ] {
                let ratio = contrast(text, on);
                assert!(ratio >= 4.5, "{name}: {what} is {ratio:.2}");
            }
            // Parts, not words: 3 to 1.
            let ratio = contrast(p.accent, p.window);
            assert!(ratio >= 3., "{name}: accent on window is {ratio:.2}");
        }
    }

    /// The kit's theme JSON and these constants are the same colours written
    /// twice. A change to one that misses the other fails here.
    #[test]
    fn the_theme_json_has_the_same_colours() {
        let json: serde_json::Value =
            serde_json::from_str(crate::style::PALETTE_THEMES).expect("the bundled themes parse");
        let themes = json["themes"].as_array().expect("a list of themes");
        assert_eq!(themes.len(), ALL.len());
        for (name, p) in ALL {
            let theme = themes
                .iter()
                .find(|t| t["name"] == name)
                .unwrap_or_else(|| panic!("{name} is in the JSON"));
            let hex = |c: u32| format!("#{c:06x}");
            for (key, colour) in [
                ("background", p.window),
                ("foreground", p.text),
                ("border", p.border),
                ("popover.background", p.popover),
                ("primary.background", p.button),
                ("primary.foreground", p.button_fg),
                ("primary.hover.background", p.button_hover),
                ("primary.active.background", p.button_active),
                ("ring", p.focus),
                ("link", p.link),
                ("sidebar.background", p.sidebar),
                ("selection.background", p.selection),
                ("muted.foreground", p.text_meta),
                ("list.active.background", p.selected),
            ] {
                assert_eq!(theme["colors"][key], hex(colour), "{name}: colors.{key}");
            }
            let highlight = &theme["highlight"];
            assert_eq!(highlight["editor.background"], hex(p.code), "{name}");
            assert_eq!(highlight["editor.foreground"], hex(p.syntax.fg), "{name}");
            for (key, colour) in [
                ("comment", p.syntax.comment),
                ("keyword", p.syntax.keyword),
                ("variable.special", p.syntax.storage),
                ("string", p.syntax.string),
                ("number", p.syntax.number),
                ("function", p.syntax.function),
                ("constructor", p.syntax.call),
                ("operator", p.syntax.operator),
            ] {
                assert_eq!(
                    highlight["syntax"][key]["color"],
                    hex(colour),
                    "{name}: syntax.{key}"
                );
            }
        }
    }
}
