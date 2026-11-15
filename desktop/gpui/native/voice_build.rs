// Voice's native parts (desktop/gpui/src/voice/), kept out of build.rs's own
// lines so the rest of it merges as it is: the keyboard shim, and the Swift
// helper for the microphone, speech recognition and the speaker. The helper
// goes in OUT_DIR, and the app carries it inside its binary (voice/helper.rs,
// `include_bytes!`), so the bundle scripts don't change. With no swiftc (or
// THREADBEAR_NO_VOICE_HELPER set) it's left empty, and Voice says it isn't
// built in rather than the build failing.
pub fn build() {
    println!("cargo:rerun-if-changed=native/voice_keys.m");
    println!("cargo:rerun-if-changed=native/voice_helper.swift");
    println!("cargo:rerun-if-env-changed=THREADBEAR_NO_VOICE_HELPER");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    cc::Build::new().file("native/voice_keys.m").flag("-fobjc-arc").compile("threadbear_voice_keys");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("threadbear-voice-helper");
    let _ = std::fs::write(&out, b"");
    if std::env::var_os("THREADBEAR_NO_VOICE_HELPER").is_some() {
        return;
    }
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        _ => return,
    };
    // As old a macOS as the app itself; SpeechAnalyzer's parts check for 26
    // when they run.
    let built = std::process::Command::new("xcrun")
        .args(["swiftc", "-O", "-parse-as-library", "-target", &format!("{arch}-apple-macos13.0"), "native/voice_helper.swift", "-o"])
        .arg(&out)
        .status();
    if !built.is_ok_and(|s| s.success()) {
        let _ = std::fs::write(&out, b"");
        println!("cargo:warning=Voice's helper didn't build (no swiftc with the macOS 26 SDK?), so this app has no Voice.");
    }
}
