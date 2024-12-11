pub struct ContentFilter {
    pub max_message_size: usize,
    pub forbidden_words: Vec<String>,
}

impl ContentFilter {
    pub fn new(max_message_size: usize, forbidden_words: Vec<String>) -> Self {
        Self {
            max_message_size,
            forbidden_words,
        }
    }

    pub fn is_valid_message(&self, message: &str) -> bool {
        if message.len() > self.max_message_size {
            return false;
        }

        for word in &self.forbidden_words {
            if message.to_lowercase().contains(&word.to_lowercase()) {
                return false;
            }
        }
        true
    }
}
