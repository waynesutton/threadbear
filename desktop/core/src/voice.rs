// Voice mode's logic with no audio in it, ported from the Swift app's
// ThreadbearCore/Voice (UtteranceTracker, SpokenText, SpokenAnswer,
// SpokenApprovals, VoiceCatalog, DictatedText), so the GPUI app talks and
// listens by the same rules. The microphone, speech recognition and playback
// are the Swift helper's (desktop/gpui/native/voice_helper.swift); the app's
// side is desktop/gpui/src/voice/. The keyboard's rules are voice_keys.rs.
use crate::reducer::event::{ApprovalDecision, ApprovalOption, ApprovalRequest};
use serde::{Deserialize, Serialize};

// ---- UtteranceTracker ----

/// A result from the recogniser: text for a stretch of the audio (seconds
/// from the start), a guess it may replace or its final text.
#[derive(Clone, Debug, PartialEq)]
pub struct Heard {
    pub text: String,
    pub start: f64,
    pub end: f64,
    pub is_final: bool,
}

impl Heard {
    pub fn new(text: &str, start: f64, end: f64, is_final: bool) -> Heard {
        Heard { text: text.to_owned(), start, end, is_final }
    }
}

/// Turns the recogniser's stream of results into things Mike said. The
/// recogniser sends volatile guesses that it later replaces with final text,
/// each for a stretch of the audio. Once he's paused for `pause` seconds
/// whatever he's said since the last send goes, and anything the recogniser
/// finalises later for audio that was already sent is dropped, so nothing
/// goes twice.
#[derive(Clone, Debug)]
pub struct UtteranceTracker {
    /// How long a pause ends what he's saying.
    pub pause: f64,
    finals: Vec<Heard>,
    volatile: Option<Heard>,
    /// Audio up to here has been sent.
    sent_through: f64,
    last_heard: Option<f64>,
    /// The words that went last, and where in the audio they started: the
    /// recogniser goes on revising that stretch after it's gone, and only
    /// words past these are new.
    last_sent: Vec<String>,
    last_sent_start: f64,
}

impl UtteranceTracker {
    pub fn new(pause: f64) -> UtteranceTracker {
        UtteranceTracker { pause, finals: Vec::new(), volatile: None, sent_through: -1.0, last_heard: None, last_sent: Vec::new(), last_sent_start: -1.0 }
    }

    fn waiting(&self) -> impl Iterator<Item = &Heard> {
        self.finals.iter().chain(self.volatile.iter())
    }

    /// What he's said so far that hasn't gone, for the live caption.
    pub fn pending(&self) -> String {
        join(self.waiting().map(|r| r.text.as_str()))
    }

    /// Where in the audio what's waiting starts (seconds), to tell when he
    /// said it: results can reach us well after the words.
    pub fn pending_start(&self) -> Option<f64> {
        self.waiting().map(|r| r.start).reduce(f64::min)
    }

    pub fn has_words(&self) -> bool {
        !self.pending().is_empty()
    }

    pub fn sent_through(&self) -> f64 {
        self.sent_through
    }

    /// Takes a result from the recogniser. True when it had new words in it
    /// (so whatever is speaking should stop: he's talking).
    pub fn add(&mut self, result: Heard, now: f64) -> bool {
        // Audio already sent: the recogniser finalising what went.
        if result.end <= self.sent_through + 0.001 {
            return false;
        }
        let mut result = result;
        // A revision of what went, reaching a little further: only the words
        // after it are new. Without this every phrase went twice, once as a
        // guess and again as final text (8 Nov, Fn dictation).
        if result.start < self.sent_through - 0.001 {
            let sent: &[String] = if result.start >= self.last_sent_start - 0.001 { &self.last_sent } else { &[] };
            let Some(rest) = words_after(&result.text, sent) else { return false };
            result.text = rest;
            result.start = self.sent_through;
        }
        let words = !result.text.trim().is_empty();
        if result.is_final {
            if self.volatile.as_ref().is_some_and(|v| v.end <= result.end + 0.001) {
                self.volatile = None;
            }
            if words {
                self.finals.push(result);
            }
        } else {
            self.volatile = words.then_some(result);
        }
        if words {
            self.last_heard = Some(now);
        }
        words
    }

    /// What to send now, if he's paused long enough after saying something.
    pub fn take(&mut self, now: f64) -> Option<String> {
        let last = self.last_heard?;
        if now - last < self.pause || !self.has_words() {
            return None;
        }
        self.flush()
    }

    /// Sends whatever there is now (Voice turned off mid-sentence).
    pub fn flush(&mut self) -> Option<String> {
        let text = self.pending();
        let end = self.waiting().map(|r| r.end).reduce(f64::max).unwrap_or(self.sent_through);
        if let Some(start) = self.pending_start().filter(|_| !text.is_empty()) {
            self.last_sent = text.split_whitespace().map(str::to_owned).collect();
            self.last_sent_start = start;
        }
        self.finals.clear();
        self.volatile = None;
        self.last_heard = None;
        self.sent_through = self.sent_through.max(end);
        (!text.is_empty()).then_some(text)
    }
}

/// The words of `text` after the ones that went (`sent`), or None if there
/// are none. Compared without case or punctuation, as the final text
/// capitalises and punctuates the guess. With nothing to compare against,
/// all of it overlapped what went, so none of it is new.
fn words_after(text: &str, sent: &[String]) -> Option<String> {
    if sent.is_empty() {
        return None;
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    let plain = |w: &str| w.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
    let mut matched = 0;
    while matched < words.len().min(sent.len()) && plain(words[matched]) == plain(&sent[matched]) {
        matched += 1;
    }
    // A reworded revision: skip as many words as went, so it isn't typed twice.
    let skip = if matched == sent.len() { matched } else { sent.len() };
    (words.len() > skip).then(|| words[skip..].join(" "))
}

fn join<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    parts.map(str::trim).filter(|p| !p.is_empty()).collect::<Vec<_>>().join(" ")
}

// ---- SpokenText ----

/// What's said in place of a code block or a table.
pub const CODE_LINE: &str = "There's some code on screen.";
pub const TABLE_LINE: &str = "There's a table on screen.";

/// A reply's words, ready to speak, one entry per sentence (long ones split
/// at a comma or a space so none is too long to start quickly). Empty when
/// there's nothing worth saying.
pub fn sentences(markdown: &str) -> Vec<String> {
    sentences_within(markdown, 220)
}

pub fn sentences_within(markdown: &str, max_length: usize) -> Vec<String> {
    split_sentences(&plain(markdown)).into_iter().flat_map(|s| cut(&s, max_length)).collect()
}

/// The reply as plain spoken text: the markdown taken out, code and tables
/// left for the screen.
pub fn plain(markdown: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let (mut in_code, mut said_code, mut said_table) = (false, false, false);
    for raw in markdown.split('\n') {
        let line = raw.trim_matches(|c| c == ' ' || c == '\t');
        if line.starts_with("```") || line.starts_with("~~~") {
            if !in_code && !said_code {
                lines.push(CODE_LINE.to_owned());
                said_code = true;
            }
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if line.starts_with('|') {
            if !said_table {
                lines.push(TABLE_LINE.to_owned());
                said_table = true;
            }
            continue;
        }
        said_table = false;
        if line.chars().all(|c| "-*_=".contains(c)) {
            lines.push(String::new());
            continue;
        }
        let mut text = strip_list_marker(strip_quote(strip_heading(line))).to_owned();
        text = inline(&text);
        if text.is_empty() {
            continue;
        }
        // A heading or list item with no stop of its own still ends a sentence.
        if text.chars().last().is_some_and(|c| !".!?:;".contains(c)) {
            text.push('.');
        }
        lines.push(text);
    }
    let joined = lines.into_iter().filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
    joined.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_heading(line: &str) -> &str {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) {
        let rest = &line[hashes..];
        if rest.starts_with(char::is_whitespace) {
            return rest.trim_start();
        }
    }
    line
}

fn strip_quote(line: &str) -> &str {
    match line.strip_prefix('>') {
        Some(rest) => rest.strip_prefix(char::is_whitespace).unwrap_or(rest),
        None => line,
    }
}

/// `- `, `* `, `+ `, `1. ` or `1) `, and a task box after it.
fn strip_list_marker(line: &str) -> &str {
    let after = if line.starts_with(['-', '*', '+']) {
        &line[1..]
    } else {
        let digits = line.chars().take_while(char::is_ascii_digit).count();
        match line[digits..].chars().next() {
            Some('.' | ')') if digits > 0 => &line[digits + 1..],
            _ => return line,
        }
    };
    if !after.starts_with(char::is_whitespace) {
        return line;
    }
    let rest = after.trim_start();
    for box_ in ["[ ]", "[x]", "[X]"] {
        if let Some(r) = rest.strip_prefix(box_) {
            if r.starts_with(char::is_whitespace) {
                return r.trim_start();
            }
        }
    }
    rest
}

/// Links, emphasis, inline code and bare urls, as they'd be read.
fn inline(text: &str) -> String {
    let t = drop_images(text);
    let t = link_words(&t);
    let t = name_urls(&t);
    let t = unquote_code(&t);
    let t = unemphasise(&t);
    t.replace("**", "").trim().to_owned()
}

/// `[words](target)` at `i`: the words, and where it ends.
fn link_at(c: &[char], i: usize, allow_empty: bool) -> Option<(String, usize)> {
    if c.get(i) != Some(&'[') {
        return None;
    }
    let close = (i + 1..c.len()).find(|&j| c[j] == ']')?;
    if close == i + 1 && !allow_empty {
        return None;
    }
    if c.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = (close + 2..c.len()).find(|&j| c[j] == ')')?;
    Some((c[i + 1..close].iter().collect(), end + 1))
}

fn drop_images(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '!' {
            if let Some((_, end)) = link_at(&c, i + 1, true) {
                i = end;
                continue;
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

fn link_words(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if let Some((words, end)) = link_at(&c, i, false) {
            out.push_str(&words);
            i = end;
            continue;
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

fn name_urls(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    loop {
        let at = ["http://", "https://"].iter().filter_map(|p| rest.find(p).map(|i| (i, p.len()))).min_by_key(|&(i, _)| i);
        let Some((i, scheme)) = at else { break };
        let tail = &rest[i + scheme..];
        let len = tail.find(|c: char| c.is_whitespace() || c == ')').unwrap_or(tail.len());
        if len == 0 {
            out.push_str(&rest[..i + scheme]);
            rest = tail;
            continue;
        }
        out.push_str(&rest[..i]);
        out.push_str("a link");
        rest = &tail[len..];
    }
    out.push_str(rest);
    out
}

fn unquote_code(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        out.push_str(&rest[..open]);
        out.push_str(&after[..close]);
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// `(\*\*|__|\*|_|~~)(?=\S)(.+?)(?<=\S)\1` → the words, as the Swift app's
/// regex does, without lookaround (Rust's regex has none, and core has no
/// regex at all).
fn unemphasise(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let starts = |at: usize, m: &[char]| c.len() >= at + m.len() && c[at..at + m.len()] == *m;
    let markers: [&[char]; 5] = [&['*', '*'], &['_', '_'], &['*'], &['_'], &['~', '~']];
    let mut out = String::new();
    let mut i = 0;
    'outer: while i < c.len() {
        for m in markers {
            if !starts(i, m) {
                continue;
            }
            let from = i + m.len();
            if c.get(from).is_none_or(|ch| ch.is_whitespace()) {
                continue;
            }
            for j in from + 1..c.len() {
                if starts(j, m) && !c[j - 1].is_whitespace() {
                    out.extend(&c[from..j]);
                    i = j + m.len();
                    continue 'outer;
                }
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// Sentences, at a stop followed by a space.
fn split_sentences(text: &str) -> Vec<String> {
    let c: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    for (i, &ch) in c.iter().enumerate() {
        current.push(ch);
        let next = c.get(i + 1).copied().unwrap_or(' ');
        if ".!?".contains(ch) && (next == ' ' || next == '\n') {
            let s = current.trim();
            if !s.is_empty() {
                out.push(s.to_owned());
            }
            current.clear();
        }
    }
    let rest = current.trim();
    if !rest.is_empty() {
        out.push(rest.to_owned());
    }
    out
}

/// A sentence too long to start speaking quickly, cut at commas, or failing
/// that at spaces.
fn cut(sentence: &str, max_length: usize) -> Vec<String> {
    if sentence.chars().count() <= max_length {
        return vec![sentence.to_owned()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in sentence.split(' ').filter(|w| !w.is_empty()) {
        let candidate = if current.is_empty() { word.to_owned() } else { format!("{current} {word}") };
        if candidate.chars().count() > max_length && !current.is_empty() {
            out.push(std::mem::replace(&mut current, word.to_owned()));
        } else {
            current = candidate;
            if current.ends_with(',') && current.chars().count() > max_length / 2 {
                out.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

// ---- SpokenAnswer ----

/// A yes or a no said to an approval card. Only a whole answer counts, one of
/// the phrases below once the filler's gone, so anything more ("no, use the
/// other folder") is a message, which declines the card with his words, as
/// typing does. Whole phrases, not a bag of allowed words: as a bag, "do you
/// always do that?" came out as always allow (Codex's review).
pub fn spoken_decision(said: &str, options: &[ApprovalOption]) -> Option<ApprovalDecision> {
    // "Okay?" is him asking.
    if said.contains('?') {
        return None;
    }
    let lowered = said.to_lowercase().replace('\u{2019}', "'").replace("thank you", " ");
    let cleaned: String = lowered.chars().map(|c| if c.is_ascii_lowercase() || c == '\'' || c == ' ' { c } else { ' ' }).collect();
    const FILLER: [&str; 9] = ["um", "uh", "er", "erm", "hmm", "please", "thanks", "bear", "threadbear"];
    let phrase = cleaned.split(' ').filter(|w| !w.is_empty() && !FILLER.contains(w)).collect::<Vec<_>>().join(" ");
    if is_no(&phrase) {
        return options.contains(&ApprovalOption::Deny).then_some(ApprovalDecision::Deny);
    }
    if ALWAYS.contains(&phrase.as_str()) {
        return options.contains(&ApprovalOption::AllowAlways).then_some(ApprovalDecision::AllowAlways);
    }
    if is_yes(&phrase) {
        return options.contains(&ApprovalOption::Allow).then_some(ApprovalDecision::Allow);
    }
    None
}

const YES_STARTS: [&str; 8] = ["yes", "yeah", "yep", "yup", "sure", "ok", "okay", "alright"];
const YES_ACTS: [&str; 7] = ["go ahead", "go ahead and do it", "do it", "run it", "allow it", "that's fine", "sounds good"];
const ALWAYS: [&str; 6] = ["always allow", "always allow it", "allow always", "yes always", "yes always allow", "yes always allow it"];
const NO_PLAIN: [&str; 16] = ["no", "nope", "nah", "deny", "deny it", "decline", "cancel", "cancel it", "cancel that", "stop", "stop it", "skip", "skip it", "not now", "not yet", "no way"];
const NO_DONT: [&str; 8] = ["don't", "don't do it", "don't do that", "dont", "dont do it", "dont do that", "do not do it", "do not do that"];

fn is_yes(p: &str) -> bool {
    YES_STARTS.contains(&p)
        || YES_ACTS.contains(&p)
        || ["allow", "approve", "approve it", "approved"].contains(&p)
        || YES_STARTS.iter().any(|s| p.strip_prefix(s).and_then(|r| r.strip_prefix(' ')).is_some_and(|r| YES_ACTS.contains(&r)))
}

fn is_no(p: &str) -> bool {
    NO_PLAIN.contains(&p)
        || NO_DONT.contains(&p)
        || p.strip_prefix("no ").is_some_and(|r| NO_DONT.contains(&r))
        || ["no stop", "no cancel", "no not now", "no not yet"].contains(&p)
}

// ---- SpokenApprovals ----

/// Which approval card a spoken yes or no answers. A voice yes can let a tool
/// run, so this errs towards not answering: the card stays on screen, where
/// it can always be answered.
///
/// A spoken answer only goes to a card that was read out whole, to the end,
/// and only if he started saying it after that and after the app stopped
/// talking, by when the words were captured, not when the recogniser handed
/// them over. The card must also still be open, unchanged and the newest
/// waiting. So a card that came in just before he answered the last can't
/// take his yes, nor can one that arrives while a slow recogniser is still
/// working on that yes, and nor can the agent approve itself by being heard
/// saying "yes" from the speakers. Anything that sounds like an answer but
/// isn't one of these answers nothing, and the card is read again.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpokenApprovals {
    /// Cards waiting, oldest first.
    open: Vec<ApprovalRequest>,
    /// The card read out to the end, which an answer may go to, and when its
    /// reading finished, on the same clock as `take`'s times.
    heard: Option<(ApprovalRequest, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpokenOutcome {
    /// Answer this card with this decision.
    Answer(ApprovalRequest, ApprovalDecision),
    /// It sounded like an answer, but not to a card he has heard: read this
    /// one (the newest) out again rather than guess.
    ReadAgain(ApprovalRequest),
    /// It sounded like an answer, but the newest card can't be read out
    /// whole: it's for the screen.
    OnScreen(ApprovalRequest),
    /// Not an answer, or no card waiting: his words go as a message.
    Message,
}

/// The longest command read out: past this nobody's really following.
const LONGEST_READING: usize = 160;

impl SpokenApprovals {
    pub fn open(&self) -> &[ApprovalRequest] {
        &self.open
    }

    pub fn heard(&self) -> Option<&ApprovalRequest> {
        self.heard.as_ref().map(|(c, _)| c)
    }

    /// The words that say all of what approving this card would do, or None:
    /// then Voice can't answer it, and it's for the screen. Only a plain
    /// one-line command is. A title alone isn't enough: a command's is its
    /// first line, so "echo checking" could hide an `rm -rf` on the line
    /// below. Read as it stands, not through `sentences`, which leaves code
    /// and links out. Nothing outside ASCII, where an invisible or look-alike
    /// character makes it a different command that sounds the same.
    pub fn reading(card: &ApprovalRequest) -> Option<&str> {
        let title = card.title.as_str();
        let sayable = |c: char| c.is_ascii_alphanumeric() || " -_./=:,@".contains(c);
        let ok = !title.is_empty()
            && title.chars().count() <= LONGEST_READING
            && (card.name == "Bash" || card.name == "PowerShell")
            && card.input["command"].as_str() == Some(title)
            && title.chars().all(sayable);
        ok.then_some(title)
    }

    /// A card came in (or came again, maybe changed). Nothing heard counts
    /// until this one, the newest, has been read out.
    pub fn requested(&mut self, card: ApprovalRequest) {
        self.open.retain(|c| c.approval_id != card.approval_id);
        self.open.push(card);
        self.heard = None;
    }

    /// The card was settled, on screen, by voice or by the runner.
    pub fn resolved(&mut self, approval_id: &str) {
        self.open.retain(|c| c.approval_id != approval_id);
        if self.heard().is_some_and(|c| c.approval_id == approval_id) {
            self.heard = None;
        }
    }

    /// The card's reading finished playing at `time`. It's what an answer
    /// goes to only if it's still the newest card open, exactly as it was read.
    pub fn read_out(&mut self, card: &ApprovalRequest, time: f64) {
        if Self::reading(card).is_some() && self.open.last() == Some(card) {
            self.heard = Some((card.clone(), time));
        }
    }

    pub fn clear(&mut self) {
        self.open.clear();
        self.heard = None;
    }

    /// What `said` does. `spoken_at` is when he started saying it, and
    /// `quiet_since` when the app last stopped talking (infinity while it's
    /// talking); an unknown time is -infinity, which never answers.
    pub fn take(&mut self, said: &str, spoken_at: f64, quiet_since: f64) -> SpokenOutcome {
        let Some(newest) = self.open.last().cloned() else { return SpokenOutcome::Message };
        if let Some((card, heard_at)) = &self.heard {
            if *card == newest && spoken_at >= *heard_at && spoken_at >= quiet_since {
                if let Some(decision) = spoken_decision(said, &card.options) {
                    let card = card.clone();
                    self.open.pop();
                    self.heard = None;
                    return SpokenOutcome::Answer(card, decision);
                }
            }
        }
        // Any answer at all, to any card: not sent as a message ("yes" in the
        // thread would decline the card), and not guessed at.
        if spoken_decision(said, &[ApprovalOption::Allow, ApprovalOption::AllowAlways, ApprovalOption::Deny]).is_none() {
            return SpokenOutcome::Message;
        }
        if Self::reading(&newest).is_none() {
            return SpokenOutcome::OnScreen(newest);
        }
        self.heard = None;
        SpokenOutcome::ReadAgain(newest)
    }
}

// ---- VoiceCatalog ----

/// A voice Settings > Voice offers: pocket-tts's built-in ones, with Emily and
/// Mike the names asked for on its eve and michael.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceOption {
    /// What's saved.
    pub id: &'static str,
    pub name: String,
    /// pocket-tts's name for it (its `voice_url`).
    pub engine_voice: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceLanguage {
    /// pocket-tts's `--language`.
    pub id: &'static str,
    pub name: &'static str,
    /// What Apple's speech recognition listens for.
    pub locale: &'static str,
}

const MORE_VOICES: [&str; 20] = ["alba", "anna", "azelma", "cosette", "eponine", "estelle", "fantine", "jane", "lola", "mary", "vera", "charles", "george", "giovanni", "javert", "jean", "juergen", "marius", "paul", "rafael"];

pub fn voices() -> Vec<VoiceOption> {
    let mut list = vec![
        VoiceOption { id: "emily", name: "Emily".to_owned(), engine_voice: "eve" },
        // A voice called Mike, not the person: spelled from its id so
        // fork-view.test.ts, which looks for the name in strings, passes it.
        VoiceOption { id: "mike", name: capitalised("mike"), engine_voice: "michael" },
    ];
    list.extend(MORE_VOICES.iter().map(|&v| VoiceOption { id: v, name: capitalised(v), engine_voice: v }));
    list
}

fn capitalised(word: &str) -> String {
    let mut c = word.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// The saved voice, or Emily for anything unknown.
pub fn voice(id: Option<&str>) -> VoiceOption {
    let all = voices();
    all.iter().find(|v| Some(v.id) == id).cloned().unwrap_or_else(|| all[0].clone())
}

pub const SPEEDS: [f64; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];

/// "1x", "1.25x".
pub fn speed_title(speed: f64) -> String {
    format!("{speed}x")
}

/// The saved speed, clamped to one that's offered.
pub fn speed(value: Option<f64>) -> f64 {
    let Some(value) = value.filter(|v| *v > 0.0) else { return 1.0 };
    SPEEDS.iter().copied().min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs())).unwrap_or(1.0)
}

pub const LANGUAGES: [VoiceLanguage; 7] = [
    VoiceLanguage { id: "english", name: "English", locale: "en-US" },
    VoiceLanguage { id: "french", name: "French", locale: "fr-FR" },
    VoiceLanguage { id: "german", name: "German", locale: "de-DE" },
    VoiceLanguage { id: "spanish", name: "Spanish", locale: "es-ES" },
    VoiceLanguage { id: "italian", name: "Italian", locale: "it-IT" },
    VoiceLanguage { id: "portuguese", name: "Portuguese", locale: "pt-PT" },
    VoiceLanguage { id: "dutch", name: "Dutch", locale: "nl-NL" },
];

pub fn language(id: Option<&str>) -> VoiceLanguage {
    LANGUAGES.iter().find(|l| Some(l.id) == id).copied().unwrap_or(LANGUAGES[0])
}

// ---- The button's icon ----

/// The Voice button's icon, picked in Settings > Voice. Phosphor's icons
/// (phosphoricons.com, MIT), bold, to sit beside Send's arrow, as the Swift
/// app's VoiceIcon keeps them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VoiceIcon {
    #[default]
    Microphone,
    MicrophoneFill,
    MicrophoneStage,
    Waveform,
}

impl VoiceIcon {
    pub const ALL: [VoiceIcon; 4] = [VoiceIcon::Microphone, VoiceIcon::MicrophoneFill, VoiceIcon::MicrophoneStage, VoiceIcon::Waveform];

    /// The Swift app's raw value, as saved.
    pub fn name(self) -> &'static str {
        match self {
            VoiceIcon::Microphone => "microphone",
            VoiceIcon::MicrophoneFill => "microphoneFill",
            VoiceIcon::MicrophoneStage => "microphoneStage",
            VoiceIcon::Waveform => "waveform",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            VoiceIcon::Microphone => "Microphone",
            VoiceIcon::MicrophoneFill => "Microphone, filled",
            VoiceIcon::MicrophoneStage => "Stage microphone",
            VoiceIcon::Waveform => "Waveform",
        }
    }

    /// The whole icon as SVG, filled black for the app to tint.
    pub fn svg(self) -> &'static str {
        match self {
            VoiceIcon::Microphone => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path fill="black" d="M128,180a52.06,52.06,0,0,0,52-52V64A52,52,0,0,0,76,64v64A52.06,52.06,0,0,0,128,180ZM100,64a28,28,0,0,1,56,0v64a28,28,0,0,1-56,0Zm40,155.22V240a12,12,0,0,1-24,0V219.22A92.14,92.14,0,0,1,36,128a12,12,0,0,1,24,0,68,68,0,0,0,136,0,12,12,0,0,1,24,0A92.14,92.14,0,0,1,140,219.22Z"/></svg>"#,
            VoiceIcon::MicrophoneFill => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path fill="black" d="M80,128V64a48,48,0,0,1,96,0v64a48,48,0,0,1-96,0Zm128,0a8,8,0,0,0-16,0,64,64,0,0,1-128,0,8,8,0,0,0-16,0,80.11,80.11,0,0,0,72,79.6V240a8,8,0,0,0,16,0V207.6A80.11,80.11,0,0,0,208,128Z"/></svg>"#,
            VoiceIcon::MicrophoneStage => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path fill="black" d="M168,12A75.9,75.9,0,0,0,92.49,96.33L23.91,189.85a19.89,19.89,0,0,0,2,26l14.29,14.29a19.89,19.89,0,0,0,26,2l93.52-68.58A76,76,0,1,0,168,12Zm52,76a51.66,51.66,0,0,1-7.75,27.27L140.74,43.75A52,52,0,0,1,220,88ZM54.72,210.71l-9.43-9.43,56.19-76.63a76.46,76.46,0,0,0,29.87,29.87ZM116,88a51.63,51.63,0,0,1,7.75-27.27l71.51,71.51A52,52,0,0,1,116,88Z"/></svg>"#,
            VoiceIcon::Waveform => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><path fill="black" d="M60,96v64a12,12,0,0,1-24,0V96a12,12,0,0,1,24,0ZM88,20A12,12,0,0,0,76,32V224a12,12,0,0,0,24,0V32A12,12,0,0,0,88,20Zm40,32a12,12,0,0,0-12,12V192a12,12,0,0,0,24,0V64A12,12,0,0,0,128,52Zm40,32a12,12,0,0,0-12,12v64a12,12,0,0,0,24,0V96A12,12,0,0,0,168,84Zm40-16a12,12,0,0,0-12,12v96a12,12,0,0,0,24,0V80A12,12,0,0,0,208,68Z"/></svg>"#,
        }
    }

    pub fn saved(raw: Option<&str>) -> VoiceIcon {
        VoiceIcon::ALL.into_iter().find(|i| Some(i.name()) == raw).unwrap_or_default()
    }
}

// ---- What's saved ----

/// Settings > Voice, as `voice.json` in the runner's data folder holds it.
/// The names are the Swift app's UserDefaults keys, so a later import from
/// it is a straight copy. Missing is the default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VoicePrefs {
    /// Use Voice: off hides every Voice button and turns the keys off.
    #[serde(rename = "voiceEnabled", skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(rename = "voiceIcon", skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(rename = "voiceName", skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(rename = "voiceSpeed", skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    #[serde(rename = "voiceLanguage", skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// A Core Audio device UID; none for the system default.
    #[serde(rename = "voiceMicrophone", skip_serializing_if = "Option::is_none")]
    pub microphone: Option<String>,
    #[serde(rename = "voiceKeyTrigger", skip_serializing_if = "Option::is_none")]
    pub key_trigger: Option<String>,
    #[serde(rename = "voiceKeyHold", skip_serializing_if = "Option::is_none")]
    pub key_hold: Option<bool>,
    #[serde(rename = "voiceKeyHandsFree", skip_serializing_if = "Option::is_none")]
    pub key_hands_free: Option<bool>,
    #[serde(rename = "voiceKeyShift", skip_serializing_if = "Option::is_none")]
    pub key_shift: Option<bool>,
    #[serde(rename = "voiceKeyDoubleTap", skip_serializing_if = "Option::is_none")]
    pub key_double_tap: Option<bool>,
    #[serde(rename = "voiceCommandMode", skip_serializing_if = "Option::is_none")]
    pub command_mode: Option<bool>,
    #[serde(rename = "voiceKeyCommand", skip_serializing_if = "Option::is_none")]
    pub key_command: Option<String>,
}

impl VoicePrefs {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn icon(&self) -> VoiceIcon {
        VoiceIcon::saved(self.icon.as_deref())
    }

    pub fn voice(&self) -> VoiceOption {
        voice(self.voice.as_deref())
    }

    pub fn speed(&self) -> f64 {
        speed(self.speed)
    }

    pub fn language(&self) -> VoiceLanguage {
        language(self.language.as_deref())
    }

    pub fn microphone(&self) -> Option<&str> {
        self.microphone.as_deref().filter(|m| !m.is_empty())
    }

    pub fn keys(&self) -> crate::voice_keys::VoiceKeys {
        use crate::voice_keys::{VoiceCommandKey, VoiceKeys, VoiceTriggerKey};
        let d = VoiceKeys::default();
        VoiceKeys {
            trigger: self.key_trigger.as_deref().and_then(VoiceTriggerKey::from_name).unwrap_or(d.trigger),
            hold_to_dictate: self.key_hold.unwrap_or(d.hold_to_dictate),
            hands_free_with_space: self.key_hands_free.unwrap_or(d.hands_free_with_space),
            shift_toggle: self.key_shift.unwrap_or(d.shift_toggle),
            double_tap: self.key_double_tap.unwrap_or(d.double_tap),
            command_mode: self.command_mode.unwrap_or(d.command_mode),
            command: self.key_command.as_deref().and_then(VoiceCommandKey::from_name).unwrap_or(d.command),
        }
    }
}

// ---- DictatedText ----

/// What a stretch of speech puts into a text box: the words, spaced from
/// what's either side of the cursor, so talking after a word doesn't run into
/// it. "New line" or "new paragraph" said on its own is a line break, as
/// macOS's own dictation has it. None for nothing to insert.
pub fn insertion(words: &str, before: Option<char>, after: Option<char>) -> Option<String> {
    let spoken = words.trim();
    if spoken.is_empty() {
        return None;
    }
    if let Some(line_break) = line_break(spoken) {
        return Some(line_break.to_owned());
    }
    let mut text = spoken.to_owned();
    // After a line break or at the start, a new line starts with a capital.
    if before.is_none_or(|b| b == '\n' || b == '\r') {
        if let Some(first) = text.chars().next().filter(|c| c.is_lowercase()) {
            text = first.to_uppercase().chain(text.chars().skip(1)).collect();
        }
    }
    if before.is_some_and(|b| !b.is_whitespace()) {
        text.insert(0, ' ');
    }
    if after.is_some_and(|a| !a.is_whitespace() && !".,;:!?)]}\"'".contains(a)) {
        text.push(' ');
    }
    Some(text)
}

fn line_break(spoken: &str) -> Option<&'static str> {
    let bare = spoken.to_lowercase();
    let bare = bare.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace());
    match bare {
        "new line" | "newline" | "next line" => Some("\n"),
        "new paragraph" | "next paragraph" => Some("\n\n"),
        _ => None,
    }
}
