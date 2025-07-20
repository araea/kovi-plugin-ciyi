use kovi::serde_json;
use once_cell::sync::Lazy;
use std::collections::HashSet;

const ALL_WORDS_JSON: &str = include_str!("../res/all_words.json");
const QUESTION_WORDS_JSON: &str = include_str!("../res/question_words.json");

pub static ALL_WORDS: Lazy<HashSet<String>> = Lazy::new(|| {
    let words: Vec<String> =
        serde_json::from_str(ALL_WORDS_JSON).expect("Failed to parse all_words.json");
    words.into_iter().collect()
});

pub static QUESTION_WORDS: Lazy<Vec<String>> = Lazy::new(|| {
    serde_json::from_str(QUESTION_WORDS_JSON).expect("Failed to parse question_words.json")
});
