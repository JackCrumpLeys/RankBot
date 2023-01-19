pub fn score_message(message: &str) -> i32 {
    let mut score = 0.0;
    for word in message.split_whitespace() {
        let word_score = score_word(word);
        score += word_score;
    }
    score.round() as i32
}

pub fn score_word(word: &str) -> f32 {
    let mut score = 0.0;
    let mut word = word.to_lowercase();
    if word.ends_with('.') || word.ends_with(',') {
        word = word[..word.len() - 1].to_string();
    }
    if word.ends_with('!') || word.ends_with('?') {
        word = word[..word.len() - 1].to_string();
        score += 1.0;
    }
    if word.starts_with("http") {
        score += 1.0;
    }
    if word.starts_with("www") {
        score += 1.0;
    }
    for c in word.chars() {
        let char_score = score_char(c);
        score += char_score;
    }
    score / word.len() as f32
}

pub fn score_char(c: char) -> f32 {
    match c {
        'a' | 'e' | 'i' | 'o' | 'u' | 'y' => 1.0,
        'b' | 'c' | 'd' | 'f' | 'g' | 'h' | 'j' | 'k' | 'l' | 'm' | 'n' | 'p' | 'q' | 'r' | 's'
        | 't' | 'v' | 'w' | 'x' | 'z' => 0.5,
        _ => 0.0,
    }
}
