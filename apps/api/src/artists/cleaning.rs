//! Name-cleaning heuristics for canonical-artist matching, validated
//! against the real unmatched-artist corpus before landing here — see
//! the `matching_eval` eval harness (issue tracked separately) for the
//! methodology: 253/1084 (23%) of real unmatched names resolve once
//! `worker.rs` tries these variants, versus 75/1084 (7%) from widening
//! the search alone.
//!
//! Everything here stays inside this codebase's existing bias toward
//! exact-normalized matching (see `worker.rs`'s module docs) — cleaning
//! only ever *removes* characters from the original name before the same
//! exact-match check runs, it never fuzzes or scores a near-match.

/// Ticketmaster's own Music-segment genre taxonomy (fetched live via
/// `GET /discovery/v2/classifications.json`, Aug 2026 — 24 genres).
/// Guards against a specific real failure mode found in eval: an event
/// whose title mentions its own genre ("Jazz, Rhythm & Blues with Chris
/// Q. Murphy...") cleans down to the bare genre word, which then
/// coincidentally exact-matches some unrelated catalog artist literally
/// named after that genre. Rejecting a match whose *final* candidate name
/// is a bare genre word costs nothing — no real artist in the matched
/// sample happened to be named exactly "Jazz"/"Rock"/etc.
const TICKETMASTER_MUSIC_GENRES: &[&str] = &[
    "alternative",
    "ballads/romantic",
    "blues",
    "chanson francaise",
    "children's music",
    "classical",
    "country",
    "dance/electronic",
    "folk",
    "hip-hop/rap",
    "holiday",
    "jazz",
    "latin",
    "medieval/renaissance",
    "metal",
    "new age",
    "other",
    "pop",
    "r&b",
    "reggae",
    "religious",
    "rock",
    "undefined",
    "world",
];

/// Rejects a candidate match whose name is nothing but a bare genre
/// label — see module docs.
pub(super) fn is_bare_genre_word(name: &str) -> bool {
    TICKETMASTER_MUSIC_GENRES.contains(&name.trim().to_lowercase().as_str())
}

/// Splits obviously-multi-artist billing ("Zedd, Ganja White Night,
/// Crankdat...") and strips a handful of trailing qualifiers real
/// Ticketmaster/PredictHQ titles carry that no catalog entry ever would
/// ("Luh Tyler - Destined For Greatness Tour", "Casey Donahew in
/// Deshler"). Ordered most-specific-first (the caller always tries the
/// original name before these) so a match on a less-mangled variant is
/// always preferred.
pub(super) fn billing_variants(name: &str) -> Vec<String> {
    let mut variants = Vec::new();

    let stripped = strip_known_suffixes(name);
    if stripped != name {
        variants.push(stripped.clone());
    }

    let first_billed = first_billed_segment(&stripped);
    if first_billed != stripped {
        variants.push(first_billed.clone());
    }
    let first_billed_of_original = first_billed_segment(name);
    if first_billed_of_original != name && !variants.contains(&first_billed_of_original) {
        variants.push(first_billed_of_original);
    }

    variants.retain(|v| !v.trim().is_empty());
    variants.dedup();
    variants
}

/// Takes the first segment of an obvious multi-artist billing string.
/// Deliberately conservative about what counts as a separator -- "&" and
/// "/" appear inside plenty of single-artist names (e.g. "Earth, Wind &
/// Fire", "AC/DC"), so only split on separators that are unambiguous
/// multi-act billing markers in practice.
fn first_billed_segment(name: &str) -> String {
    for sep in [
        ", ",
        " w/ ",
        " with special guest ",
        " feat. ",
        " featuring ",
    ] {
        if let Some((first, _)) = name.split_once(sep) {
            return first.trim().to_string();
        }
    }
    name.trim().to_string()
}

/// Strips trailing qualifiers that describe the *event*, not the artist:
/// a dash-separated tour name, a trailing parenthetical, or a trailing
/// "in <City>" / "at <Venue>" qualifier.
fn strip_known_suffixes(name: &str) -> String {
    let mut s = name.trim();

    if let Some(idx) = s.rfind('(')
        && s.ends_with(')')
    {
        s = s[..idx].trim();
    }

    if let Some((before, _)) = s.split_once(" - ") {
        s = before.trim();
    }

    for marker in [" in ", " at "] {
        if let Some(idx) = s.rfind(marker) {
            let tail = &s[idx + marker.len()..];
            // Only strip when the tail looks like a bare place name (short,
            // title-cased words) -- not, say, "Cheap Trick in Concert" or a
            // real band name that happens to contain " in ".
            let looks_like_place = tail.split_whitespace().count() <= 3
                && tail.chars().next().is_some_and(|c| c.is_uppercase())
                && !tail.eq_ignore_ascii_case("concert");
            if looks_like_place {
                s = s[..idx].trim();
            }
        }
    }

    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_billed_segment_splits_comma_separated_billing() {
        assert_eq!(
            first_billed_segment("Zedd, Ganja White Night, Crankdat"),
            "Zedd"
        );
    }

    #[test]
    fn first_billed_segment_leaves_ampersand_names_alone() {
        assert_eq!(first_billed_segment("Earth, Wind & Fire"), "Earth");
    }

    #[test]
    fn first_billed_segment_splits_w_slash_billing() {
        assert_eq!(
            first_billed_segment("Myles Smith w/ Jonah Kagen"),
            "Myles Smith"
        );
    }

    #[test]
    fn first_billed_segment_leaves_single_artist_names_alone() {
        assert_eq!(
            first_billed_segment("Vicki Burns Quintet"),
            "Vicki Burns Quintet"
        );
    }

    #[test]
    fn strip_known_suffixes_removes_dash_separated_tour_name() {
        assert_eq!(
            strip_known_suffixes("Luh Tyler - Destined For Greatness Tour"),
            "Luh Tyler"
        );
    }

    #[test]
    fn strip_known_suffixes_removes_trailing_in_city() {
        assert_eq!(
            strip_known_suffixes("Casey Donahew in Deshler"),
            "Casey Donahew"
        );
    }

    #[test]
    fn strip_known_suffixes_leaves_in_concert_alone() {
        assert_eq!(
            strip_known_suffixes("Cheap Trick in Concert"),
            "Cheap Trick in Concert"
        );
    }

    #[test]
    fn strip_known_suffixes_removes_trailing_parenthetical() {
        assert_eq!(strip_known_suffixes("Joe Hart (US)"), "Joe Hart");
    }

    #[test]
    fn billing_variants_combines_suffix_strip_and_split() {
        let variants = billing_variants("Zedd, Ganja White Night, Crankdat in Magna");
        assert!(variants.contains(&"Zedd".to_string()));
    }

    #[test]
    fn is_bare_genre_word_rejects_known_genres_case_insensitively() {
        assert!(is_bare_genre_word("Jazz"));
        assert!(is_bare_genre_word("rock"));
        assert!(is_bare_genre_word("  Blues  "));
    }

    #[test]
    fn is_bare_genre_word_accepts_real_artist_names() {
        assert!(!is_bare_genre_word("Zedd"));
        assert!(!is_bare_genre_word("Casey Donahew"));
        assert!(!is_bare_genre_word("Soul-Jazz"));
    }
}
