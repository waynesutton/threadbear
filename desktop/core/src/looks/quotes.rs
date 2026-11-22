//! A quote for the Librarian page (Customize > Quote of the day), a port of
//! the Swift app's Quotes.swift: one a day, the same all day, changing at
//! 4 am on this machine's clock, with a refresh button for another. The list
//! ships with the app, so nothing is fetched and nothing is sent. The days
//! walk the same fixed shuffle as the Swift app's, so both show the same
//! quote on the same day.
//!
//! Who's in it: writers, poets, scientists, engineers, designers, artists,
//! athletes and secular philosophers, each with where they said it. Not
//! religious figures, politicians, heads of state or military leaders, nor
//! tech founders and CEOs, and nothing political. The tests hold the list
//! to these rules.

use chrono::{Datelike, Duration, NaiveDateTime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quote {
    pub text: &'static str,
    pub author: &'static str,
    /// Writer, physicist, artist...
    pub role: &'static str,
    /// Where it was said or written, and when.
    pub source: &'static str,
}

impl Quote {
    /// "author: text", as the Swift `id`.
    pub fn id(&self) -> String {
        format!("{}: {}", self.author, self.text)
    }
}

const fn q(
    text: &'static str,
    author: &'static str,
    role: &'static str,
    source: &'static str,
) -> Quote {
    Quote {
        text,
        author,
        role,
        source,
    }
}

/// The day a quote belongs to: the local date after taking 4 hours off, so
/// a quote lasts through a late night and changes at 4 am.
pub fn day_key(local: NaiveDateTime) -> String {
    let shifted = local - Duration::hours(4);
    format!(
        "{:04}-{:02}-{:02}",
        shifted.year(),
        shifted.month(),
        shifted.day()
    )
}

/// Today's quote, moved on `skips` times by the refresh button. The same
/// day always gives the same quote, and the whole list goes round before
/// one comes back: the days walk a fixed shuffle of it.
pub fn pick(day_key: &str, skips: i64) -> Option<&'static Quote> {
    pick_from(day_key, skips, QUOTES)
}

fn pick_from<'a>(day_key: &str, skips: i64, list: &'a [Quote]) -> Option<&'a Quote> {
    if list.is_empty() {
        return None;
    }
    let n = list.len() as i64;
    let order = shuffled(list.len());
    let index = (days(day_key) + skips.max(0)) % n;
    Some(&list[order[((index + n) % n) as usize]])
}

/// Days since 1 Jan 1970 for a yyyy-MM-dd key, by the calendar alone (no
/// time zone), so the count is the same everywhere.
pub(crate) fn days(key: &str) -> i64 {
    let parts: Vec<i64> = key.split('-').filter_map(|p| p.parse().ok()).collect();
    if parts.len() != 3 {
        return 0;
    }
    // Days from civil (Howard Hinnant's algorithm), with Swift's truncating division.
    let (y0, m, d) = (parts[0], parts[1], parts[2]);
    let y = if m <= 2 { y0 - 1 } else { y0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A fixed shuffle of 0..count (a seeded Fisher-Yates), the same on every
/// machine and every launch, and the same as the Swift app's.
pub(crate) fn shuffled(count: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..count).collect();
    let mut seed: u64 = 0x7468_7265_6164_6265; // "threadbe"
    for i in (1..count).rev() {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = ((seed >> 33) % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

/// Text that may never be shown, whoever it's put down to: slogans tied to
/// tech founders, and quotes usually pinned on the wrong person.
pub const BLOCKED_TEXT: &[&str] = &[
    "done is better than perfect",
    "move fast and break things",
    "stay hungry, stay foolish",
    "insanity is doing the same thing",
    "be yourself; everyone else is already taken",
    "simplicity is the ultimate sophistication",
    "never too late to be what you might have been",
    "the secret of getting ahead is getting started",
    "culture eats strategy",
    "what gets measured gets managed",
    "10,000 ways that won't work",
];

/// People who may never be quoted.
pub const BLOCKED_AUTHORS: &[&str] = &[
    "Elon Musk",
    "Mark Zuckerberg",
    "Steve Jobs",
    "Bill Gates",
    "Jeff Bezos",
    "Peter Thiel",
    "Reid Hoffman",
    "Paul Graham",
    "Naval Ravikant",
    "Marcus Aurelius",
    "Seneca",
    "Buddha",
    "Rumi",
    "Lao Tzu",
    "Confucius",
    "Winston Churchill",
    "Abraham Lincoln",
    "Grace Hopper",
];

pub const QUOTES: &[Quote] = &[
    // Building
    q("The purpose of business is to create a customer.", "Peter Drucker", "Management writer", "The Practice of Management, 1954"),
    q("There is nothing so useless as doing efficiently that which should not be done at all.", "Peter Drucker", "Management writer", "Harvard Business Review, 1963"),
    q("Plan to throw one away; you will, anyhow.", "Fred Brooks", "Computer scientist", "The Mythical Man-Month, 1975"),
    q("A complex system that works is invariably found to have evolved from a simple system that worked.", "John Gall", "Writer", "Systemantics, 1975"),
    q("The best is the enemy of the good.", "Voltaire", "Writer", "1772, in translation"),
    q("Good is the enemy of great.", "Jim Collins", "Writer", "Good to Great, 2001"),
    q("People don't buy what you do; they buy why you do it.", "Simon Sinek", "Writer", "Start With Why, 2009"),
    q("The best way to predict the future is to invent it.", "Alan Kay", "Computer scientist", "Xerox PARC, 1971"),
    q("It is possible to fly without motors, but not without knowledge and skill.", "Wilbur Wright", "Inventor", "Letter to Octave Chanute, 1900"),
    q("Any sufficiently advanced technology is indistinguishable from magic.", "Arthur C. Clarke", "Writer", "Profiles of the Future, 1973"),
    q("Be so good they can't ignore you.", "Steve Martin", "Comedian and writer", "Born Standing Up, 2007"),
    q("If you do not work on an important problem, it's unlikely you'll do important work.", "Richard Hamming", "Mathematician", "You and Your Research, 1986"),
    q("A ship in harbor is safe, but that is not what ships are built for.", "John A. Shedd", "Writer", "Salt from My Attic, 1928"),
    // Craft
    q("Simplicity is prerequisite for reliability.", "Edsger W. Dijkstra", "Computer scientist", "EWD498, 1975"),
    q("Premature optimization is the root of all evil.", "Donald Knuth", "Computer scientist", "Structured Programming with go to Statements, 1974"),
    q("Debugging is twice as hard as writing the code in the first place.", "Brian Kernighan and P. J. Plauger", "Computer scientists", "The Elements of Programming Style, 1978"),
    q("Any fool can write code that a computer can understand. Good programmers write code that humans can understand.", "Martin Fowler", "Engineer", "Refactoring, 1999"),
    q("Programs must be written for people to read, and only incidentally for machines to execute.", "Harold Abelson and Gerald Jay Sussman", "Computer scientists", "Structure and Interpretation of Computer Programs, 1985"),
    q("Talk is cheap. Show me the code.", "Linus Torvalds", "Engineer", "Linux kernel mailing list, 2000"),
    q("Good design is as little design as possible.", "Dieter Rams", "Designer", "Ten principles of good design"),
    q("Perfection is achieved, not when there is nothing more to add, but when there is nothing left to take away.", "Antoine de Saint-Exupéry", "Writer", "Terre des Hommes, 1939, in translation"),
    q("What I cannot create, I do not understand.", "Richard Feynman", "Physicist", "His Caltech blackboard, 1988"),
    q("If I have seen further it is by standing on the shoulders of Giants.", "Isaac Newton", "Scientist", "Letter to Robert Hooke, 1676"),
    // Creativity and habits
    q("Inspiration is for amateurs. The rest of us just show up and get to work.", "Chuck Close", "Artist", "Interview, 2006"),
    q("Genius is one percent inspiration and ninety-nine percent perspiration.", "Thomas Edison", "Inventor", "Harper's Monthly, 1932"),
    q("You can't use up creativity. The more you use, the more you have.", "Maya Angelou", "Writer", "Bell Telephone Magazine, 1982"),
    q("Creativity is a habit, and the best creativity is the result of good work habits.", "Twyla Tharp", "Choreographer", "The Creative Habit, 2003"),
    q("It's only by going through a volume of work that you will close that gap.", "Ira Glass", "Radio host", "Interview, 2009"),
    q("First forget inspiration. Habit is more dependable.", "Octavia E. Butler", "Writer", "Furor Scribendi, 1995"),
    q("Almost all good writing begins with terrible first efforts.", "Anne Lamott", "Writer", "Bird by Bird, 1994"),
    q("If there's a book that you want to read, but it hasn't been written yet, then you must write it.", "Toni Morrison", "Writer", "Speech, 1981"),
    q("How we spend our days is, of course, how we spend our lives.", "Annie Dillard", "Writer", "The Writing Life, 1989"),
    q("You do not rise to the level of your goals. You fall to the level of your systems.", "James Clear", "Writer", "Atomic Habits, 2018"),
    q("We are what we repeatedly do. Excellence, then, is not an act, but a habit.", "Will Durant", "Historian", "The Story of Philosophy, 1926"),
    q("Great things are done by a series of small things brought together.", "Vincent van Gogh", "Artist", "Letter to Theo, 1882"),
    q("If you hear a voice within you say you cannot paint, then by all means paint, and that voice will be silenced.", "Vincent van Gogh", "Artist", "Letter to Theo, 1883"),
    q("Have no fear of perfection, you'll never reach it.", "Salvador Dalí", "Artist", "50 Secrets of Magic Craftsmanship, 1948"),
    // Mindset
    q("Life is like riding a bicycle. To keep your balance you must keep moving.", "Albert Einstein", "Physicist", "Letter to his son Eduard, 1930"),
    q("I have no special talents. I am only passionately curious.", "Albert Einstein", "Physicist", "Letter to Carl Seelig, 1952"),
    q("I've failed over and over and over again in my life. And that is why I succeed.", "Michael Jordan", "Athlete", "Nike advert, 1997"),
    q("Try again. Fail again. Fail better.", "Samuel Beckett", "Writer", "Worstward Ho, 1983"),
    q("The best way out is always through.", "Robert Frost", "Poet", "A Servant to Servants, 1914"),
    q("I am not afraid of storms, for I am learning how to sail my ship.", "Louisa May Alcott", "Writer", "Little Women, 1868"),
    q("Men are disturbed not by things, but by the opinions about things.", "Epictetus", "Philosopher", "Enchiridion 5, in translation"),
    q("The first principle is that you must not fool yourself, and you are the easiest person to fool.", "Richard Feynman", "Physicist", "Caltech commencement, 1974"),
    q("All we have to decide is what to do with the time that is given us.", "J. R. R. Tolkien", "Writer", "The Fellowship of the Ring, 1954"),
    q("Tell me, what is it you plan to do with your one wild and precious life?", "Mary Oliver", "Poet", "The Summer Day, 1990"),
    q("It is good to have an end to journey toward; but it is the journey that matters, in the end.", "Ursula K. Le Guin", "Writer", "The Left Hand of Darkness, 1969"),
    q("One never notices what has been done; one can only see what remains to be done.", "Marie Curie", "Scientist", "Letter to her brother, 1894"),
    q("I love deadlines. I love the whooshing noise they make as they go by.", "Douglas Adams", "Writer", "The Salmon of Doubt, 2002"),
    q("No human is limited.", "Eliud Kipchoge", "Athlete", "INEOS 1:59 Challenge, 2019"),
    q("Pressure is a privilege.", "Billie Jean King", "Athlete", "Pressure Is a Privilege, 2008"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn at(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn the_day_turns_over_at_4am_local() {
        assert_eq!(day_key(at("2026-11-09 03:59")), "2026-11-08");
        assert_eq!(day_key(at("2026-11-09 04:00")), "2026-11-09");
    }

    #[test]
    fn same_day_same_quote_and_the_list_goes_round_before_repeating() {
        assert_eq!(pick("2026-11-09", 0), pick("2026-11-09", 0));
        assert_eq!(days("1970-01-01"), 0);
        assert_eq!(days("2026-11-09") - days("2026-11-08"), 1);
        assert_eq!(days("2025-01-01") - days("2024-12-31"), 1);
        assert!(days("2026-01-01") > 0);
        let seen: HashSet<String> = (0..QUOTES.len() as i64)
            .map(|s| pick("2026-01-01", s).unwrap().id())
            .collect();
        assert_eq!(seen.len(), QUOTES.len());
        assert_ne!(pick("2026-11-09", 1), pick("2026-11-09", 0));
        assert_eq!(pick_from("2026-11-09", 0, &[]), None);
    }

    #[test]
    fn every_quote_keeps_the_rules() {
        assert!(QUOTES.len() >= 40);
        let ids: HashSet<String> = QUOTES.iter().map(Quote::id).collect();
        assert_eq!(ids.len(), QUOTES.len(), "no quote twice");
        for quote in QUOTES {
            assert!(!quote.text.is_empty() && !quote.author.is_empty() && !quote.role.is_empty());
            assert!(
                !quote.source.is_empty(),
                "every quote says where it's from: {}",
                quote.text
            );
            let author = quote.author.to_lowercase();
            assert!(
                !BLOCKED_AUTHORS
                    .iter()
                    .any(|b| author.contains(&b.to_lowercase())),
                "{}",
                quote.author
            );
            let text = quote.text.to_lowercase();
            assert!(
                !BLOCKED_TEXT.iter().any(|b| text.contains(b)),
                "{}",
                quote.text
            );
            assert!(!quote.text.contains('\u{2014}'), "no em dashes in UI copy");
        }
    }
}
