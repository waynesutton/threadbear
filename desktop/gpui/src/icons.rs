// The icons the app draws in place of SF Symbols, which can't ship outside
// Apple's systems. Lucide comes with gpui-kit's assets; main.rs registers
// `AllAssets`, so any Lucide icon can be named here. This is the one place
// that maps Swift's symbol names to a bundled icon: add a line when a view
// needs a new one. A symbol with no match draws a question mark, which is
// easy to spot in a screenshot; a project's own icon falls back to its letter
// instead (see `try_symbol`).
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::{
    img, AnyElement, App, Hsla, Image, ImageFormat, ImageSource, IntoElement as _, ObjectFit,
    Pixels, Styled as _, StyledImage as _,
};
use std::sync::{Arc, OnceLock};

/// `star.fill` has no Lucide twin (Lucide's stars are outlines), so it's drawn
/// here: the star's shape, filled with the current text colour.
const STAR_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M12 2.5l2.9 6.4 7 .7-5.2 4.7 1.5 6.9L12 17.7l-6.2 3.5 1.5-6.9-5.2-4.7 7-.7z" fill="black"/></svg>"#;

/// `circle.fill`, Bear Colour's mark for a colour that isn't the bear's
/// now: Lucide's circle is an outline.
const CIRCLE_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="black"/></svg>"#;
/// A bear's head with a plus: start a thread about this (Settings' sections,
/// Mike, 17 Nov). Drawn in Lucide's line style to sit with the other icons.
const BEAR_PLUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M5.2 7.6A2.6 2.6 0 1 1 8.4 4.6"/><path d="M11.6 4.6A2.6 2.6 0 1 1 14.8 7.6"/><circle cx="10" cy="12" r="6.6"/><circle cx="7.6" cy="10.6" r=".5" fill="black"/><circle cx="12.4" cy="10.6" r=".5" fill="black"/><ellipse cx="10" cy="14.4" rx="1.9" ry="1.4"/><path d="M19.5 15.5v7M16 19h7"/></svg>"#;

/// A filled message bubble distinguishes a board card worked on by this
/// thread from the outline button that opens another thread.
/// Tasks' Doing: a ring half filled, as Linear marks work in progress.
const CIRCLE_HALF: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="none" stroke="black" stroke-width="2"/><path d="M12 6a6 6 0 0 1 0 12z" fill="black"/></svg>"#;

const BUBBLE_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M4 3h16a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H7l-5 3V5a2 2 0 0 1 2-2z" fill="black"/></svg>"#;

/// `hand.raised.fill`, the asks card's mark: Lucide's hand is an outline, and
/// the Swift card's is the filled one, four fingers and the thumb out to the
/// right. Drawn to SF's proportions (about 11 by 15 at the card's 14 pt
/// title), so at 16.5 px it covers what the Swift one does.
const HAND_RAISED_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><g fill="black"><rect x="4" y="5.2" width="3" height="10.4" rx="1.5"/><rect x="7.4" y="2.2" width="3" height="12" rx="1.5"/><rect x="10.8" y="1" width="3" height="13" rx="1.5"/><rect x="14.2" y="2.6" width="3" height="11.6" rx="1.5"/><path d="M4 12h13.2v1.4l1.2-2.6c.5-1 1.5-1.3 2.2-.8.6.4.7 1.2.4 1.9l-2.6 6.6c-.9 2.4-3 4.1-5.8 4.1h-2.2C6.8 22.6 4 19.9 4 16.4z"/></g></svg>"#;

/// `lock.open`, Full access's mark: SF's has the shackle swung up and out to
/// the right of the body, where Lucide's stands over it, so it read as a
/// closed lock beside the Swift one. An outline in the text colour, about as
/// wide as SF's at 12 pt when drawn at 13.
const LOCK_OPEN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="11" width="13" height="11" rx="2.5"/><path d="M12.5 11V7.5a4.5 4.5 0 0 1 9 0V9.5"/></svg>"#;

/// SF's `checkmark` as the done card draws it at its title's size: a short
/// stroke and a long one rising well above it. Lucide's check is wide and
/// low, and beside the Swift card's it read as a different mark.
const CHECKMARK_TALL: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 13.5l5 5.5L20 3.5"/></svg>"#;

/// The list panels' status shapes (issue #67): filled, with the mark cut out,
/// so the shape reads at 13 px whatever its colour. A circle for needs you, a
/// diamond for failed and a triangle for a warning, so a red-green
/// colour-blind eye tells them apart by shape, not by orange against red.
const CIRCLE_ALERT_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><mask id="m"><rect width="24" height="24" fill="white"/><path d="M12 6.8v6.4" stroke="black" stroke-width="2.6" stroke-linecap="round"/><circle cx="12" cy="17.2" r="1.55" fill="black"/></mask><circle cx="12" cy="12" r="10.5" fill="black" mask="url(#m)"/></svg>"#;
const DIAMOND_X_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><mask id="m"><rect width="24" height="24" fill="white"/><path d="M8.9 8.9l6.2 6.2M15.1 8.9l-6.2 6.2" stroke="black" stroke-width="2.5" stroke-linecap="round"/></mask><path d="M12 .8L23.2 12 12 23.2.8 12z" fill="black" stroke="black" stroke-width="1.2" stroke-linejoin="round" mask="url(#m)"/></svg>"#;
const TRIANGLE_ALERT_FILLED: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><mask id="m"><rect width="24" height="24" fill="white"/><path d="M12 9.2v5.2" stroke="black" stroke-width="2.5" stroke-linecap="round"/><circle cx="12" cy="18" r="1.45" fill="black"/></mask><path d="M12 2.2L23 21.4H1z" fill="black" stroke="black" stroke-width="1.6" stroke-linejoin="round" mask="url(#m)"/></svg>"#;

/// The done card's tick (`checkmark` at 14 pt): about 12 by 12 at 16.
pub fn tall_checkmark() -> Icon {
    Icon::default().data(CHECKMARK_TALL)
}

/// The providers' marks (`ProviderLogo`), as single-colour SVGs that take the
/// text colour, as the Swift app's template images do. Claude's is Anthropic's
/// "A\" rather than Claude's starburst: Mike asked for the Anthropic logo, and
/// it stays legible at 12 pt.
const CLAUDE_LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="black" d="M13.827 3.52h3.603L24 20h-3.603l-6.57-16.48zm-7.258 0h3.767L16.906 20h-3.674l-1.343-3.461H5.017l-1.344 3.46H0L6.57 3.522zm4.132 9.959L8.453 7.687 6.205 13.48H10.7z"/></svg>"#;
const CODEX_LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="black" d="M8.086.457a6.105 6.105 0 013.046-.415c1.333.153 2.521.72 3.564 1.7a.117.117 0 00.107.029c1.408-.346 2.762-.224 4.061.366l.063.03.154.076c1.357.703 2.33 1.77 2.918 3.198.278.679.418 1.388.421 2.126a5.655 5.655 0 01-.18 1.631.167.167 0 00.04.155 5.982 5.982 0 011.578 2.891c.385 1.901-.01 3.615-1.183 5.14l-.182.22a6.063 6.063 0 01-2.934 1.851.162.162 0 00-.108.102c-.255.736-.511 1.364-.987 1.992-1.199 1.582-2.962 2.462-4.948 2.451-1.583-.008-2.986-.587-4.21-1.736a.145.145 0 00-.14-.032c-.518.167-1.04.191-1.604.185a5.924 5.924 0 01-2.595-.622 6.058 6.058 0 01-2.146-1.781c-.203-.269-.404-.522-.551-.821a7.74 7.74 0 01-.495-1.283 6.11 6.11 0 01-.017-3.064.166.166 0 00.008-.074.115.115 0 00-.037-.064 5.958 5.958 0 01-1.38-2.202 5.196 5.196 0 01-.333-1.589 6.915 6.915 0 01.188-2.132c.45-1.484 1.309-2.648 2.577-3.493.282-.188.55-.334.802-.438.286-.12.573-.22.861-.304a.129.129 0 00.087-.087A6.016 6.016 0 015.635 2.31C6.315 1.464 7.132.846 8.086.457zm-.804 7.85a.848.848 0 00-1.473.842l1.694 2.965-1.688 2.848a.849.849 0 001.46.864l1.94-3.272a.849.849 0 00.007-.854l-1.94-3.393zm5.446 6.24a.849.849 0 000 1.695h4.848a.849.849 0 000-1.696h-4.848z"/></svg>"#;

/// Grok's, OpenCode's and Cursor's marks, from the same set as the two above
/// (LobeHub's icons), so the Usage panel's cards each carry their provider's logo.
const GROK_LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="black" d="M9.27 15.29l7.978-5.897c.391-.29.95-.177 1.137.272.98 2.369.542 5.215-1.41 7.169-1.951 1.954-4.667 2.382-7.149 1.406l-2.711 1.257c3.889 2.661 8.611 2.003 11.562-.953 2.341-2.344 3.066-5.539 2.388-8.42l.006.007c-.983-4.232.242-5.924 2.75-9.383.06-.082.12-.164.179-.248l-3.301 3.305v-.01L9.267 15.292M7.623 16.723c-2.792-2.67-2.31-6.801.071-9.184 1.761-1.763 4.647-2.483 7.166-1.425l2.705-1.25a7.808 7.808 0 00-1.829-1A8.975 8.975 0 005.984 5.83c-2.533 2.536-3.33 6.436-1.962 9.764 1.022 2.487-.653 4.246-2.34 6.022-.599.63-1.199 1.259-1.682 1.925l7.62-6.815"/></svg>"#;
const OPENCODE_LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="black" d="M16 6H8v12h8V6zm4 16H4V2h16v20z"/></svg>"#;
const CURSOR_LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="black" d="M22.106 5.68L12.5.135a.998.998 0 00-.998 0L1.893 5.68a.84.84 0 00-.419.726v11.186c0 .3.16.577.42.727l9.607 5.547a.999.999 0 00.998 0l9.608-5.547a.84.84 0 00.42-.727V6.407a.84.84 0 00-.42-.726zm-.603 1.176L12.228 22.92c-.063.108-.228.064-.228-.061V12.34a.59.59 0 00-.295-.51l-9.11-5.26c-.107-.062-.063-.228.062-.228h18.55c.264 0 .428.286.296.514z"/></svg>"#;
/// Gemini's four-pointed star, from the same set, for the Antigravity provider
/// (Gemini on a Google plan), which shows as Gemini.
const GEMINI_LOGO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="black" d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"/></svg>"#;

/// Colour versions of the six provider marks above, shown when Appearance's
/// Color model icons switch is on. Provider-level marks keep the same
/// silhouette as their monochrome marks above; Anthropic's own model rows in
/// the picker use LobeHub's Claude starburst (`claude-color`). Rendered at
/// 64x64 so Retina icons stay sharp.
/// Sources: LobeHub icons (`claude-color`, `codex-color`, `gemini-color`),
/// Cursor's official shaded 3D cube (`cursor.com/brand`), and OpenCode's
/// official two-tone mark (`opencode.ai/brand`).
const CLAUDE_COLOR_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill-rule="evenodd"><path fill="#D97757" d="M13.827 3.52h3.603L24 20h-3.603l-6.57-16.48zm-7.258 0h3.767L16.906 20h-3.674l-1.343-3.461H5.017l-1.344 3.46H0L6.57 3.522zm4.132 9.959L8.453 7.687 6.205 13.48H10.7z"/></svg>"##;
const CLAUDE_MODEL_COLOR_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24"><path fill="#D97757" fill-rule="nonzero" d="M4.709 15.955l4.72-2.647.08-.23-.08-.128H9.2l-.79-.048-2.698-.073-2.339-.097-2.266-.122-.571-.121L0 11.784l.055-.352.48-.321.686.06 1.52.103 2.278.158 1.652.097 2.449.255h.389l.055-.157-.134-.098-.103-.097-2.358-1.596-2.552-1.688-1.336-.972-.724-.491-.364-.462-.158-1.008.656-.722.881.06.225.061.893.686 1.908 1.476 2.491 1.833.365.304.145-.103.019-.073-.164-.274-1.355-2.446-1.446-2.49-.644-1.032-.17-.619a2.97 2.97 0 01-.104-.729L6.283.134 6.696 0l.996.134.42.364.62 1.414 1.002 2.229 1.555 3.03.456.898.243.832.091.255h.158V9.01l.128-1.706.237-2.095.23-2.695.08-.76.376-.91.747-.492.584.28.48.685-.067.444-.286 1.851-.559 2.903-.364 1.942h.212l.243-.242.985-1.306 1.652-2.064.73-.82.85-.904.547-.431h1.033l.76 1.129-.34 1.166-1.064 1.347-.881 1.142-1.264 1.7-.79 1.36.073.11.188-.02 2.856-.606 1.543-.28 1.841-.315.833.388.091.395-.328.807-1.969.486-2.309.462-3.439.813-.042.03.049.061 1.549.146.662.036h1.622l3.02.225.79.522.474.638-.079.485-1.215.62-1.64-.389-3.829-.91-1.312-.329h-.182v.11l1.093 1.068 2.006 1.81 2.509 2.33.127.578-.322.455-.34-.049-2.205-1.657-.851-.747-1.926-1.62h-.128v.17l.444.649 2.345 3.521.122 1.08-.17.353-.608.213-.668-.122-1.374-1.925-1.415-2.167-1.143-1.943-.14.08-.674 7.254-.316.37-.729.28-.607-.461-.322-.747.322-1.476.389-1.924.315-1.53.286-1.9.17-.632-.012-.042-.14.018-1.434 1.967-2.18 2.945-1.726 1.845-.414.164-.717-.37.067-.662.401-.589 2.388-3.036 1.44-1.882.93-1.086-.006-.158h-.055L4.132 18.56l-1.13.146-.487-.456.061-.746.231-.243 1.908-1.312-.006.006z"/></svg>"##;
const CODEX_COLOR_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill-rule="evenodd"><defs><linearGradient id="codex" gradientUnits="userSpaceOnUse" x1="12" y1="0" x2="12" y2="24"><stop offset="0" stop-color="#B1A7FF"/><stop offset="0.5" stop-color="#7A9DFF"/><stop offset="1" stop-color="#3941FF"/></linearGradient></defs><path fill="url(#codex)" d="M8.086.457a6.105 6.105 0 013.046-.415c1.333.153 2.521.72 3.564 1.7a.117.117 0 00.107.029c1.408-.346 2.762-.224 4.061.366l.063.03.154.076c1.357.703 2.33 1.77 2.918 3.198.278.679.418 1.388.421 2.126a5.655 5.655 0 01-.18 1.631.167.167 0 00.04.155 5.982 5.982 0 011.578 2.891c.385 1.901-.01 3.615-1.183 5.14l-.182.22a6.063 6.063 0 01-2.934 1.851.162.162 0 00-.108.102c-.255.736-.511 1.364-.987 1.992-1.199 1.582-2.962 2.462-4.948 2.451-1.583-.008-2.986-.587-4.21-1.736a.145.145 0 00-.14-.032c-.518.167-1.04.191-1.604.185a5.924 5.924 0 01-2.595-.622 6.058 6.058 0 01-2.146-1.781c-.203-.269-.404-.522-.551-.821a7.74 7.74 0 01-.495-1.283 6.11 6.11 0 01-.017-3.064.166.166 0 00.008-.074.115.115 0 00-.037-.064 5.958 5.958 0 01-1.38-2.202 5.196 5.196 0 01-.333-1.589 6.915 6.915 0 01.188-2.132c.45-1.484 1.309-2.648 2.577-3.493.282-.188.55-.334.802-.438.286-.12.573-.22.861-.304a.129.129 0 00.087-.087A6.016 6.016 0 015.635 2.31C6.315 1.464 7.132.846 8.086.457zm-.804 7.85a.848.848 0 00-1.473.842l1.694 2.965-1.688 2.848a.849.849 0 001.46.864l1.94-3.272a.849.849 0 00.007-.854l-1.94-3.393zm5.446 6.24a.849.849 0 000 1.695h4.848a.849.849 0 000-1.696h-4.848z"/></svg>"##;
const CURSOR_COLOR_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none"><g transform="translate(12 12) scale(0.08307) translate(-800 -500)"><path fill="#72716D" d="M800.004 500L923.821 571.486C923.061 572.804 921.957 573.929 920.591 574.716L804.863 641.531C801.858 643.266 798.151 643.266 795.146 641.531L679.417 574.716C678.052 573.929 676.948 572.804 676.188 571.486L800.004 500Z"/><path fill="#55544F" d="M800.005 357.168V500L676.188 571.486C675.427 570.168 675.004 568.647 675.004 567.072V432.928C675.004 429.774 676.686 426.865 679.418 425.285L795.141 358.47C796.646 357.602 798.323 357.168 799.999 357.168H800.005Z"/><path fill="#43413C" d="M923.815 428.515C923.055 427.197 921.951 426.072 920.586 425.285L804.857 358.47C803.357 357.602 801.68 357.168 800.004 357.168V500L923.821 571.486C924.581 570.168 925.005 568.647 925.005 567.072V432.928C925.005 431.348 924.587 429.838 923.821 428.515H923.815Z"/><path fill="#D6D5D2" d="M915.156 433.518C915.857 434.728 915.954 436.281 915.156 437.663L802.764 632.323C802.008 633.641 800 633.1 800 631.584V503.311C800 502.287 799.727 501.302 799.229 500.44L915.15 433.512H915.156V433.518Z"/><path fill="#FFFFFF" d="M915.155 433.518L799.233 500.445C798.741 499.588 798.023 498.86 797.134 498.345L686.049 434.209C684.731 433.453 685.272 431.445 686.788 431.445H911.566C913.162 431.445 914.459 432.307 915.155 433.518Z"/></g></svg>"##;
const OPENCODE_COLOR_DARK_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none"><path fill="#6A6868" d="M8 10h8v8H8z"/><path fill="#EEEEEE" fill-rule="evenodd" d="M16 6H8v12h8V6zm4 16H4V2h16v20z"/></svg>"##;
const OPENCODE_COLOR_LIGHT_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none"><path fill="#BDBDBD" d="M8 10h8v8H8z"/><path fill="#1F1F1F" fill-rule="evenodd" d="M16 6H8v12h8V6zm4 16H4V2h16v20z"/></svg>"##;
const GROK_COLOR_DARK_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill-rule="evenodd"><defs><linearGradient id="grokd" gradientUnits="userSpaceOnUse" x1="2" y1="2" x2="22" y2="22"><stop offset="0" stop-color="#FFFFFF"/><stop offset="1" stop-color="#B8C0CC"/></linearGradient></defs><path fill="url(#grokd)" d="M9.27 15.29l7.978-5.897c.391-.29.95-.177 1.137.272.98 2.369.542 5.215-1.41 7.169-1.951 1.954-4.667 2.382-7.149 1.406l-2.711 1.257c3.889 2.661 8.611 2.003 11.562-.953 2.341-2.344 3.066-5.539 2.388-8.42l.006.007c-.983-4.232.242-5.924 2.75-9.383.06-.082.12-.164.179-.248l-3.301 3.305v-.01L9.267 15.292M7.623 16.723c-2.792-2.67-2.31-6.801.071-9.184 1.761-1.763 4.647-2.483 7.166-1.425l2.705-1.25a7.808 7.808 0 00-1.829-1A8.975 8.975 0 005.984 5.83c-2.533 2.536-3.33 6.436-1.962 9.764 1.022 2.487-.653 4.246-2.34 6.022-.599.63-1.199 1.259-1.682 1.925l7.62-6.815"/></svg>"##;
const GROK_COLOR_LIGHT_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill-rule="evenodd"><defs><linearGradient id="grokl" gradientUnits="userSpaceOnUse" x1="2" y1="2" x2="22" y2="22"><stop offset="0" stop-color="#111317"/><stop offset="1" stop-color="#4A5260"/></linearGradient></defs><path fill="url(#grokl)" d="M9.27 15.29l7.978-5.897c.391-.29.95-.177 1.137.272.98 2.369.542 5.215-1.41 7.169-1.951 1.954-4.667 2.382-7.149 1.406l-2.711 1.257c3.889 2.661 8.611 2.003 11.562-.953 2.341-2.344 3.066-5.539 2.388-8.42l.006.007c-.983-4.232.242-5.924 2.75-9.383.06-.082.12-.164.179-.248l-3.301 3.305v-.01L9.267 15.292M7.623 16.723c-2.792-2.67-2.31-6.801.071-9.184 1.761-1.763 4.647-2.483 7.166-1.425l2.705-1.25a7.808 7.808 0 00-1.829-1A8.975 8.975 0 005.984 5.83c-2.533 2.536-3.33 6.436-1.962 9.764 1.022 2.487-.653 4.246-2.34 6.022-.599.63-1.199 1.259-1.682 1.925l7.62-6.815"/></svg>"##;
const GEMINI_COLOR_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24"><defs><linearGradient id="g0" gradientUnits="userSpaceOnUse" x1="7" y1="15.5" x2="11" y2="12"><stop stop-color="#08B962"/><stop offset="1" stop-color="#08B962" stop-opacity="0"/></linearGradient><linearGradient id="g1" gradientUnits="userSpaceOnUse" x1="8" y1="5.5" x2="11.5" y2="11"><stop stop-color="#F94543"/><stop offset="1" stop-color="#F94543" stop-opacity="0"/></linearGradient><linearGradient id="g2" gradientUnits="userSpaceOnUse" x1="3.5" y1="13.5" x2="17.5" y2="12"><stop stop-color="#FABC12"/><stop offset="0.46" stop-color="#FABC12" stop-opacity="0"/></linearGradient></defs><path fill="#3186FF" d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"/><path fill="url(#g0)" d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"/><path fill="url(#g1)" d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"/><path fill="url(#g2)" d="M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"/></svg>"##;

fn svg_image(slot: &'static OnceLock<Arc<Image>>, bytes: &'static [u8]) -> Arc<Image> {
    slot.get_or_init(|| Arc::new(Image::from_bytes(ImageFormat::Svg, bytes.to_vec())))
        .clone()
}

/// Colour image for a known provider logo, cached once per variant.
pub fn provider_color_image(provider: &str, dark: bool) -> Option<Arc<Image>> {
    static CLAUDE: OnceLock<Arc<Image>> = OnceLock::new();
    static CODEX: OnceLock<Arc<Image>> = OnceLock::new();
    static CURSOR: OnceLock<Arc<Image>> = OnceLock::new();
    static OPENCODE_DARK: OnceLock<Arc<Image>> = OnceLock::new();
    static OPENCODE_LIGHT: OnceLock<Arc<Image>> = OnceLock::new();
    static GROK_DARK: OnceLock<Arc<Image>> = OnceLock::new();
    static GROK_LIGHT: OnceLock<Arc<Image>> = OnceLock::new();
    static GEMINI: OnceLock<Arc<Image>> = OnceLock::new();
    match if provider.is_empty() {
        "claude"
    } else {
        provider
    } {
        "claude" => Some(svg_image(&CLAUDE, CLAUDE_COLOR_SVG)),
        "codex" => Some(svg_image(&CODEX, CODEX_COLOR_SVG)),
        "cursor" => Some(svg_image(&CURSOR, CURSOR_COLOR_SVG)),
        "opencode" => Some(if dark {
            svg_image(&OPENCODE_DARK, OPENCODE_COLOR_DARK_SVG)
        } else {
            svg_image(&OPENCODE_LIGHT, OPENCODE_COLOR_LIGHT_SVG)
        }),
        "grok" => Some(if dark {
            svg_image(&GROK_DARK, GROK_COLOR_DARK_SVG)
        } else {
            svg_image(&GROK_LIGHT, GROK_COLOR_LIGHT_SVG)
        }),
        "antigravity" => Some(svg_image(&GEMINI, GEMINI_COLOR_SVG)),
        _ => None,
    }
}

/// Colour image for a model row in the picker: Anthropic's own models show the
/// Claude starburst, while every other provider (including Cursor running a
/// Claude model) keeps its provider logo.
pub fn provider_model_color_image(provider: &str, dark: bool) -> Option<Arc<Image>> {
    static CLAUDE_MODEL: OnceLock<Arc<Image>> = OnceLock::new();
    match if provider.is_empty() {
        "claude"
    } else {
        provider
    } {
        "claude" => Some(svg_image(&CLAUDE_MODEL, CLAUDE_MODEL_COLOR_SVG)),
        other => provider_color_image(other, dark),
    }
}

/// A provider's logo ("claude", "codex", "grok", "opencode", "cursor", "antigravity"), or its symbol where there's no logo
/// (`ProviderMark`, with `ModelPicker.providerSymbol`'s fallbacks).
pub fn provider_mark(provider: &str) -> Icon {
    match if provider.is_empty() {
        "claude"
    } else {
        provider
    } {
        "claude" => Icon::default().data(CLAUDE_LOGO),
        "codex" => Icon::default().data(CODEX_LOGO),
        "grok" => Icon::default().data(GROK_LOGO),
        "opencode" => Icon::default().data(OPENCODE_LOGO),
        "cursor" => Icon::default().data(CURSOR_LOGO),
        "antigravity" => Icon::default().data(GEMINI_LOGO),
        _ => symbol("cpu"),
    }
}

fn render_color_or_mark(
    provider: &str,
    size: Pixels,
    color: Hsla,
    color_image: Option<Arc<Image>>,
) -> AnyElement {
    if let Some(image) = color_image {
        return img(ImageSource::Image(image))
            .flex_shrink_0()
            .size(size)
            .object_fit(ObjectFit::Contain)
            .into_any_element();
    }
    provider_mark(provider)
        .with_size(size)
        .text_color(color)
        .into_any_element()
}

/// Draws a provider's logo at `size`: in colour when Appearance's Color model
/// icons switch is on, else as the existing monochrome symbol tinted with
/// `color`. Unknown providers keep the monochrome `cpu` symbol either way.
pub fn provider_icon(provider: &str, size: Pixels, color: Hsla, cx: &App) -> AnyElement {
    let image = crate::style::color_model_icons()
        .then(|| provider_color_image(provider, cx.theme().mode.is_dark()))
        .flatten();
    render_color_or_mark(provider, size, color, image)
}

/// Draws a model row's logo in the model picker: Anthropic models show LobeHub's
/// Claude starburst when Color model icons is on, while other providers (such as
/// Cursor running an Anthropic model) keep their own provider icon. When the
/// switch is off, every provider keeps its existing monochrome mark.
pub fn provider_model_icon(provider: &str, size: Pixels, color: Hsla, cx: &App) -> AnyElement {
    let image = crate::style::color_model_icons()
        .then(|| provider_model_color_image(provider, cx.theme().mode.is_dark()))
        .flatten();
    render_color_or_mark(provider, size, color, image)
}

/// `RunnerIcon.symbol`: a runner's own icon name as a symbol this app has a
/// drawing of, or the plain desktop.
pub fn runner_symbol(icon: Option<&str>) -> &'static str {
    match icon {
        Some("macmini") => "macmini",
        Some("laptopcomputer") => "laptopcomputer",
        Some("cloud") => "cloud",
        Some("server.rack") => "server.rack",
        Some("smartphone") | Some("iphone") => "smartphone",
        _ => "desktopcomputer",
    }
}

/// A virtual runner's symbol: its Runs on entry and its threads' tiles.
pub const VIRTUAL_RUNNER: &str = "cloud.bolt";

/// The bundled icon for one of the app's SF Symbol names.
pub fn symbol(name: &str) -> Icon {
    try_symbol(name).unwrap_or_else(|| Icon::new(IconName::CircleQuestionMark))
}

/// The bundled icon for an SF Symbol name, if there is one yet.
pub fn try_symbol(name: &str) -> Option<Icon> {
    let lucide = match name {
        "star" => IconName::Star,
        "star.fill" => return Some(Icon::default().data(STAR_FILLED)),
        "bear.plus" => return Some(Icon::default().data(BEAR_PLUS)),
        "bubble.left.fill" => return Some(Icon::default().data(BUBBLE_FILLED)),
        "circle.fill" => return Some(Icon::default().data(CIRCLE_FILLED)),
        "circle.lefthalf.filled" => return Some(Icon::default().data(CIRCLE_HALF)),
        // Bears (bears.rs): the head bear's crown, any other's paw, Bear
        // Colour's palette, and a table's three people.
        "crown" | "crown.fill" => IconName::Crown,
        "pawprint.fill" => IconName::PawPrint,
        "paintpalette" | "paintpalette.fill" => IconName::Palette,
        "person.crop.circle" => IconName::UserRound,
        "person.3" | "person.3.fill" => IconName::Users,
        "star.slash" => IconName::StarOff,
        "tray" => IconName::Inbox,
        "tray.and.arrow.up" => IconName::ArchiveRestore,
        "bolt" => IconName::Zap,
        "moon" | "moon.zzz" => IconName::Moon,
        "archivebox" => IconName::Archive,
        "arrow.uturn.backward" => IconName::Undo2,
        "trash" => IconName::Trash,
        "sun.max" => IconName::Sun,
        "folder" => IconName::Folder,
        "square.and.pencil" => IconName::SquarePen,
        "magnifyingglass" => IconName::Search,
        "xmark.circle.fill" | "xmark.circle" => IconName::CircleX,
        "hand.raised" => IconName::Hand,
        "hand.raised.fill" => return Some(Icon::default().data(HAND_RAISED_FILLED)),
        "arrow.down" => IconName::ArrowDown,
        "arrow.up" => IconName::ArrowUp,
        "arrow.up.arrow.down" => IconName::ArrowUpDown,
        "ellipsis" => IconName::Ellipsis,
        "chevron.down" => IconName::ChevronDown,
        "chevron.right" => IconName::ChevronRight,
        "checkmark" => IconName::Check,
        "xmark" => IconName::X,
        "minus" => IconName::Minus,
        "clock" => IconName::Clock,
        "sunset" => IconName::Sunset,
        "sunrise" => IconName::Sunrise,
        "briefcase" => IconName::Briefcase,
        "calendar" => IconName::Calendar,
        "text.bubble" => IconName::MessageSquareText,
        "arrow.clockwise" => IconName::RotateCw,
        "pawprint" => IconName::PawPrint,
        "books.vertical" => IconName::Library,
        "bell" => IconName::Bell,
        "wrench.and.screwdriver" => IconName::Wrench,
        "hammer" => IconName::Hammer,
        "number" => IconName::Hash,
        "pencil" => IconName::Pencil,
        "rectangle.split.2x1" => IconName::Columns2,
        "square.stack" => IconName::Layers,
        // The project menu's Make Its Own Project and Merge into, and a task
        // card's thread chip (projects.rs, project_page.rs).
        "square.on.square" => IconName::Copy,
        "arrow.triangle.merge" => IconName::Merge,
        "bubble.left" => IconName::MessageSquare,
        // A Notion card's chip in Mike's message (CardChip).
        "square.text.square" => IconName::SquareMenu,
        // A message from another thread's agent (agentMessageText).
        "arrow.turn.down.right" => IconName::CornerDownRight,
        "plus" => IconName::Plus,
        "desktopcomputer" => IconName::Monitor,
        "macmini" => IconName::Server,
        // The symbols a runner can name for itself (`RunnerInfo.icon`).
        "laptopcomputer" => IconName::Laptop,
        "cloud" => IconName::Cloud,
        // A virtual runner (Runs on, a box thread's row): a machine made on
        // demand, so not the plain cloud a Hetzner runner names for itself.
        "cloud.bolt" => IconName::CloudLightning,
        "server.rack" => IconName::Server,
        "smartphone" => IconName::Smartphone,
        "arrow.right.square" => IconName::SquareArrowRight,
        "ladybug" | "ant" => IconName::Bug,
        "externaldrive" => IconName::HardDrive,
        // The symbols Settings › Pools and the librarian offer for a project
        // (`Pools.iconChoices`).
        "photo" => IconName::Image,
        "sparkle" => IconName::Sparkle,
        "chevron.left.forwardslash.chevron.right" => IconName::CodeXml,
        "house" => IconName::House,
        "paintbrush" => IconName::Paintbrush,
        "film" => IconName::Film,
        "book" => IconName::Book,
        "graduationcap" => IconName::GraduationCap,
        "leaf" => IconName::Leaf,
        "flame" => IconName::Flame,
        "heart" => IconName::Heart,
        "flag" => IconName::Flag,
        "bookmark" => IconName::Bookmark,
        "tag" => IconName::Tag,
        "globe" => IconName::Globe,
        "building.2" => IconName::Building2,
        "cart" => IconName::ShoppingCart,
        "gamecontroller" => IconName::Gamepad2,
        "music.note" => IconName::Music,
        "camera" => IconName::Camera,
        "airplane" => IconName::Plane,
        "cup.and.saucer" => IconName::Coffee,
        // The window shell's (WindowDock, ActivityBar, DetailView).
        "bubble.left.and.bubble.right" => IconName::MessagesSquare,
        "square.stack.3d.up" => IconName::Layers,
        "gearshape" => IconName::Settings,
        "gauge.with.dots.needle.33percent" | "gauge.with.dots.needle.100percent" => IconName::Gauge,
        "arrow.down.circle" => IconName::CircleArrowDown,
        "checklist" => IconName::ListChecks,
        // Prints' kinds and their menus (prints.rs, `Page.symbol`).
        "chart.bar" => IconName::ChartColumn,
        "doc.text.magnifyingglass" => IconName::FileSearch,
        "point.3.connected.trianglepath.dotted" => IconName::Workflow,
        "rectangle.split.3x1" => IconName::SquareKanban,
        "macwindow" => IconName::AppWindow,
        "powerplug" => IconName::Plug,
        // The Runners, Updates and Connectors panels'.
        "line.3.horizontal" => IconName::Menu,
        "arrow.up.forward.square" => IconName::SquareArrowOutUpRight,
        "arrow.up.forward" => IconName::ArrowUpRight,
        "nosign" => IconName::Ban,
        "terminal" => IconName::Terminal,
        "pin" => IconName::Pin,
        "square.dashed" => IconName::SquareDashed,
        "text.alignleft" => IconName::TextAlignStart,
        "sidebar.left" => IconName::PanelLeft,
        "sidebar.right" => IconName::PanelRight,
        "chevron.left" => IconName::ChevronLeft,
        "square.and.arrow.up" => IconName::Share,
        "square.and.arrow.down" => IconName::Download,
        "doc.plaintext" => IconName::FileText,
        "safari" => IconName::Compass,
        "exclamationmark.triangle" => IconName::TriangleAlert,
        "exclamationmark.triangle.fill" => {
            return Some(Icon::default().data(TRIANGLE_ALERT_FILLED))
        }
        "exclamationmark.circle.fill" => return Some(Icon::default().data(CIRCLE_ALERT_FILLED)),
        "xmark.diamond.fill" => return Some(Icon::default().data(DIAMOND_X_FILLED)),
        "bolt.horizontal" | "bolt.horizontal.circle" => IconName::Zap,
        "lock" | "lock.fill" => IconName::Lock,
        "note.text" => IconName::NotepadText,
        "lock.open" => return Some(Icon::default().data(LOCK_OPEN)),
        "cpu" => IconName::Cpu,
        // The trust chip over the composer, and the origin badges' marks.
        "lock.shield" | "lock.shield.fill" => IconName::ShieldLock,
        "shippingbox" | "shippingbox.fill" => IconName::Box,
        "house.fill" => IconName::House,
        "folder.fill" => IconName::Folder,
        "wrench.and.screwdriver.fill" => IconName::Wrench,
        "person.crop.circle.badge.exclamationmark" => IconName::UserRoundX,
        "checkmark.circle.fill" => IconName::CircleCheck,
        // Finish setting up's item still to do.
        "circle" => IconName::Circle,
        "arrow.up.right" => IconName::ArrowUpRight,
        "arrow.up.left.and.arrow.down.right" => IconName::Maximize2,
        "key" => IconName::Key,
        "arrow.triangle.2.circlepath" => IconName::RefreshCw,
        "arrow.right.circle" => IconName::CircleArrowRight,
        "minus.circle" => IconName::CircleMinus,
        "plus.square.on.square" => IconName::CopyPlus,
        // The pane beside the thread: Highlights, previews, Patches, folders.
        "doc.text" => IconName::FileText,
        "doc.richtext" => IconName::FileText,
        "doc" => IconName::File,
        "waveform" => IconName::AudioLines,
        "tablecells" => IconName::Table,
        "curlybraces.square" => IconName::FileBraces,
        "arrow.triangle.pull" => IconName::GitPullRequest,
        "smallcircle.filled.circle" => IconName::GitCommitHorizontal,
        "link" => IconName::Link,
        "circle.dashed" => IconName::CircleDashed,
        "pause.circle" => IconName::CirclePause,
        "person.2" => IconName::Users,
        "pin.slash" => IconName::PinOff,
        "arrow.triangle.branch" => IconName::GitBranch,
        "cylinder.split.1x2" => IconName::Database,
        "iphone" => IconName::Smartphone,
        "bell.slash" => IconName::BellOff,
        "questionmark.circle" => IconName::CircleQuestionMark,
        "list.bullet" => IconName::List,
        "doc.on.doc" => IconName::Copy,
        "arrow.up.forward.app" => IconName::SquareArrowOutUpRight,
        "cursorarrow.click" => IconName::MousePointerClick,
        "chevron.up" => IconName::ChevronUp,
        "info.circle" => IconName::Info,
        "eye" => IconName::Eye,
        "eye.slash" => IconName::EyeOff,
        "textformat" => IconName::Type,
        "scalemass" => IconName::Scale,
        "square.grid.2x2" => IconName::LayoutGrid,
        "checkmark.square.fill" => IconName::SquareCheck,
        "square" => IconName::Square,
        "exclamationmark.circle" => IconName::CircleAlert,
        "paperplane" => IconName::Send,
        // The Scheduled list's row buttons and menu, and the editor's Repeat menu.
        "pause" => IconName::Pause,
        "play" | "playpause" => IconName::Play,
        "calendar.badge.clock" => IconName::CalendarClock,
        "slider.horizontal.3" => IconName::SlidersHorizontal,
        "folder.badge.plus" => IconName::FolderPlus,
        // Settings' sidebar.
        "sparkles" => IconName::Sparkles,
        "keyboard" => IconName::Keyboard,
        // The Librarian view's headings.
        "hourglass" => IconName::Hourglass,
        "clock.arrow.circlepath" => IconName::RotateCcwClock,
        // A reply's links' menus (reply_links.rs), the selection bar and the
        // Explain pane (selection_bar.rs, explain_pane.rs).
        "wand.and.stars" => IconName::WandSparkles,
        "questionmark.folder" => IconName::FileQuestionMark,
        // Lucide has no magnifier with a sparkle in it: the plain one is nearest.
        "sparkle.magnifyingglass" => IconName::Search,
        "text.quote" => IconName::TextQuote,
        "arrow.up.circle.fill" => IconName::CircleArrowUp,
        // A list panel row's switch when its patch names no icon.
        "power" => IconName::Power,
        _ => return None,
    };
    Some(Icon::new(lucide))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_provider_svgs_render_for_every_known_provider() {
        let renderer = gpui_kit::SvgRenderer::new(Arc::new(()));
        for provider in [
            "",
            "claude",
            "codex",
            "cursor",
            "opencode",
            "grok",
            "antigravity",
        ] {
            for dark in [false, true] {
                let image = provider_color_image(provider, dark)
                    .unwrap_or_else(|| panic!("missing color icon for {provider:?} dark={dark}"));
                let rendered = renderer
                    .render_single_frame(image.bytes(), 1.0)
                    .unwrap_or_else(|e| panic!("bad SVG for {provider:?} dark={dark}: {e}"));
                let size = rendered.size(0);
                assert!(size.width.0 >= 64, "{provider:?} width {}", size.width.0);
                assert_eq!(size.width.0, size.height.0, "{provider:?} square");

                let model_image = provider_model_color_image(provider, dark).unwrap_or_else(|| {
                    panic!("missing model color icon for {provider:?} dark={dark}")
                });
                let model_rendered = renderer
                    .render_single_frame(model_image.bytes(), 1.0)
                    .unwrap_or_else(|e| panic!("bad model SVG for {provider:?} dark={dark}: {e}"));
                let model_size = model_rendered.size(0);
                assert!(
                    model_size.width.0 >= 64,
                    "{provider:?} model width {}",
                    model_size.width.0
                );
                assert_eq!(
                    model_size.width.0, model_size.height.0,
                    "{provider:?} model square"
                );
            }
        }
        // Anthropic model rows use the Claude starburst while the provider rail
        // keeps Anthropic's A\; Cursor keeps its own cube even for Anthropic models.
        assert_ne!(
            provider_model_color_image("claude", true).unwrap().bytes(),
            provider_color_image("claude", true).unwrap().bytes()
        );
        assert_eq!(
            provider_model_color_image("cursor", true).unwrap().bytes(),
            provider_color_image("cursor", true).unwrap().bytes()
        );
        assert!(provider_color_image("unknown", true).is_none());
        assert!(provider_color_image("unknown", false).is_none());
        assert!(provider_model_color_image("unknown", true).is_none());
    }
}
