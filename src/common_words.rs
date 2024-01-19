use std::collections::HashSet;

const WORDS: &str = include_str!("../common_words.txt");

pub fn get_common_words() -> HashSet<String> {
    WORDS
        .lines()
        .map(|s| s.to_string())
        .collect::<HashSet<String>>()
}

