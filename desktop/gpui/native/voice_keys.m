// Voice's keyboard shortcuts and typing for the GPUI app
// (desktop/gpui/src/voice/keys.rs). A local NSEvent monitor sees Fn and the
// right-hand modifiers, which GPUI's own key events don't tell apart, and
// hands each press to Rust, which says whether to swallow it (Fn-Space's
// space). Inside the app only, so it needs no Accessibility or Input
// Monitoring permission. The Swift app's VoiceKeyMonitor did the same.
//
// What's dictated goes in through the key window's text input client, the way
// macOS's own dictation reaches a GPUI text box: whatever has the keyboard
// takes it as typing, so it saves and undoes as typing does.
#import <AppKit/AppKit.h>

typedef bool (*tb_voice_key_fn)(int kind, unsigned short key_code, unsigned long flags, double timestamp, bool is_repeat);
typedef void (*tb_voice_resign_fn)(void);

static id tb_monitor;
static id tb_resign;

void tb_voice_keys_install(tb_voice_key_fn on_key, tb_voice_resign_fn on_resign) {
    if (tb_monitor) return;
    tb_monitor = [NSEvent addLocalMonitorForEventsMatchingMask:(NSEventMaskFlagsChanged | NSEventMaskKeyDown) handler:^NSEvent *(NSEvent *event) {
        bool flags = event.type == NSEventTypeFlagsChanged;
        bool swallow = on_key(flags ? 0 : 1, event.keyCode, (unsigned long)event.modifierFlags, event.timestamp, flags ? false : event.isARepeat);
        return swallow ? nil : event;
    }];
    // Switching away with the key held: it never sees the key come up.
    tb_resign = [[NSNotificationCenter defaultCenter] addObserverForName:NSApplicationDidResignActiveNotification object:nil queue:[NSOperationQueue mainQueue] usingBlock:^(NSNotification *note) {
        (void)note;
        on_resign();
    }];
}

/// The clock NSEvent's timestamps are on.
double tb_voice_uptime(void) {
    return [NSProcessInfo processInfo].systemUptime;
}

static id<NSTextInputClient> tb_client(void) {
    NSWindow *window = NSApp.keyWindow ?: NSApp.mainWindow;
    id responder = window.firstResponder;
    if (![responder respondsToSelector:@selector(insertText:replacementRange:)] || ![responder respondsToSelector:@selector(selectedRange)]) return nil;
    return (id<NSTextInputClient>)responder;
}

static uint32_t tb_char_at(id<NSTextInputClient> client, NSUInteger at) {
    if (![client respondsToSelector:@selector(attributedSubstringForProposedRange:actualRange:)]) return 0;
    NSAttributedString *text = [client attributedSubstringForProposedRange:NSMakeRange(at, 1) actualRange:NULL];
    if (text.length == 0) return 0;
    NSString *s = text.string;
    NSRange whole = [s rangeOfComposedCharacterSequenceAtIndex:0];
    UTF32Char c = 0;
    if (![[s substringWithRange:whole] getBytes:&c maxLength:sizeof(c) usedLength:NULL encoding:NSUTF32LittleEndianStringEncoding options:0 range:NSMakeRange(0, whole.length) remainingRange:NULL]) return 0;
    return c;
}

/// Whether a text box has the keyboard, and the characters either side of its
/// cursor (0 at an end, or where it won't say).
bool tb_voice_text_context(uint32_t *before, uint32_t *after) {
    *before = 0;
    *after = 0;
    id<NSTextInputClient> client = tb_client();
    if (!client) return false;
    NSRange selected = [client selectedRange];
    if (selected.location == NSNotFound) return false;
    if (selected.location > 0) *before = tb_char_at(client, selected.location - 1);
    *after = tb_char_at(client, NSMaxRange(selected));
    return true;
}

/// Types `utf8` in at the cursor, over the selection, as typing would.
bool tb_voice_insert(const char *utf8) {
    id<NSTextInputClient> client = tb_client();
    if (!client || [client selectedRange].location == NSNotFound) return false;
    NSString *text = [NSString stringWithUTF8String:utf8];
    if (!text) return false;
    [client insertText:text replacementRange:NSMakeRange(NSNotFound, 0)];
    return true;
}

typedef char *(*tb_voice_compose_fn)(const char *words, uint32_t before, uint32_t after);
typedef void (*tb_voice_free_fn)(char *text);
typedef void (*tb_voice_typed_fn)(bool typed);

/// Types `utf8` in on the main queue's next turn, not now: asking a GPUI text
/// box for its cursor needs the app's state, which is locked while Rust is
/// in the middle of an update, so asked from there it says there's no box.
/// `compose` spaces the words from what's either side of the cursor
/// (threadbear_core::voice::insertion); `typed` says whether it went in.
void tb_voice_type_later(const char *utf8, tb_voice_compose_fn compose, tb_voice_free_fn free_text, tb_voice_typed_fn typed) {
    NSString *words = [NSString stringWithUTF8String:utf8];
    if (!words) { typed(false); return; }
    dispatch_async(dispatch_get_main_queue(), ^{
        uint32_t before = 0, after = 0;
        if (!tb_voice_text_context(&before, &after)) { typed(false); return; }
        char *text = compose(words.UTF8String, before, after);
        if (!text) { typed(true); return; }
        bool done = tb_voice_insert(text);
        free_text(text);
        typed(done);
    });
}

/// System Settings > Keyboard, where "Press 🌐 key to" is.
void tb_voice_open_keyboard_settings(void) {
    NSURL *url = [NSURL URLWithString:@"x-apple.systempreferences:com.apple.Keyboard-Settings.extension"];
    if (url) [[NSWorkspace sharedWorkspace] openURL:url];
}
