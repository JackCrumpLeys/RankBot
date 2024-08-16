use serenity::model::prelude::Message;
use std::collections::HashMap;

/// Function to score a message based on the word count and # of unique words (Non-spammy score)
fn count_words(message: &str) -> (u32, u32) {
    let mut words = HashMap::new();
    let mut num_words = 0;
    for word in message.split_whitespace() {
        *words.entry(word).or_insert(0) += 1;
        num_words += 1;
    }
    (num_words, words.len() as u32)
}

/// Score discord messages based on how constructive they are
pub async fn score_message(message: &Message, recent_messages: &Vec<String>) -> f32 {
    // If there's any repetition in the recent messages, the score is lowered
    if recent_messages
        .iter()
        .any(|recent_message| recent_message == &message.content)
    {
        return 0.;
    }

    // Score based on the word count and # of unique words (lower repetition = higher score)
    let (num_words, num_unique_words) = count_words(&message.content);
    let word_score = if num_words == 0 {
        0.0
    } else {
        num_unique_words as f32 / num_words as f32
    } * 50.;

    // Longer messages score higher, with diminishing returns
    let length_score = (message.content.len() as f32).sqrt() * 50.;

    // Final score is a combination of word score and length score
    (word_score * 0.7) + (length_score * 0.3)
}
