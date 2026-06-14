use tiktoken_rs::{cl100k_base, p50k_base};

pub fn count_tokens(text: &str, model: &str) -> usize {
    if model.contains("gpt-4") || model.contains("gpt-3.5") || model.contains("text-embedding") {
        if let Ok(bpe) = cl100k_base() {
            return bpe.encode_with_special_tokens(text).len();
        }
    }
    if let Ok(bpe) = p50k_base() {
        return bpe.encode_with_special_tokens(text).len();
    }
    text.len() / 4
}

pub fn count_message_tokens(_role: &str, content: &str, model: &str) -> usize {
    let tokens = count_tokens(content, model);
    // Add overhead for message formatting
    tokens + 4 // role prefix + formatting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_short() {
        let count = count_tokens("Hello, world!", "gpt-4");
        assert!(count > 0);
        assert!(count < 10);
    }

    #[test]
    fn test_count_tokens_long() {
        let text = "Hello, world! ".repeat(100);
        let count = count_tokens(&text, "gpt-4");
        assert!(count > 100);
    }
}
