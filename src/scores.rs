/// Struct representing a user's score
pub struct UserScore {
    score: f32,
}

fn get_formatted_num_and_suffix(num: f32) -> (f32, String) {
    let (suffix, diviser) = match num {
        0.0..=999.9 => ("", 1.0),
        1000.0..=999_999.9 => ("K", 1_000.0),
        1_000_000.0..=999_999_999.9 => ("M", 1_000_000.0),
        1_000_000_000.0..=999_999_999_999.9 => ("B", 1_000_000_000.0),
        1_000_000_000_000.0..=999_999_999_999_999.9 => ("T", 1_000_000_000_000.0),
        1_000_000_000_000_000.0..=999_999_999_999_999_999.9 => ("Q", 1_000_000_000_000_000.0),
        _ => ("", 1.0),
    };
    let formatted_num = num / diviser;
    (formatted_num, suffix.to_string())
}

impl UserScore {
    pub fn new(score: f32) -> Self {
        Self { score }
    }

    /// Function that outputs a formatted score level and progress bar.
    pub fn display_score(&self) -> String {
        let progress = self.get_progress();
        let level = self.get_level();
        let (formatted_score, suffix) = get_formatted_num_and_suffix(self.score);
        let next_level_score = (level + 1.0).powf(1.5) * 1000.0;
        let (next_level_score, next_suffix) = get_formatted_num_and_suffix(next_level_score);

        format!("Level: {:.1} - {:.1}{} / {:.1}{} [{:.2}%]", level, formatted_score, suffix, next_level_score, next_suffix, progress)
    }

    /// Function that determines the user's level based on their score.
    /// The higher the score, the higher the level.
    fn get_level(&self) -> f32 {
        (self.score / 1000.0).powf(1./1.5)
    }

    /// Function that determines the user's progress towards the next level
    /// based on their score. The progress is represented as a percentage.
    fn get_progress(&self) -> f32 {
        let level = self.get_level();
        let progress = ((level - level.floor()) * 100.0);
        progress
    }


    /// Function that displays the user's progress towards the next level
    /// as a progress bar made of equal signs (`=`) and spaces. Each
    /// equal sign represents 10% progress.
    fn get_progress_bar(&self) -> String {
        let progress = self.get_progress();
        let completed = progress / 10.0;
        let remaining = 10.0 - completed;
        format!(
            "[{}>{}]",
            "=".repeat(completed as usize),
            " ".repeat(remaining as usize)
        )
    }
}