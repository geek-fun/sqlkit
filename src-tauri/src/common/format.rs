const TOOL_OUTPUT_MAX_BYTES: usize = 32 * 1024;

pub(crate) fn truncate_tool_output(output: String) -> String {
    if output.len() <= TOOL_OUTPUT_MAX_BYTES {
        return output;
    }
    let boundary = (0..=TOOL_OUTPUT_MAX_BYTES)
        .rev()
        .find(|&i| output.is_char_boundary(i))
        .unwrap_or(0);
    let omitted = output.len() - boundary;
    format!(
        "{}\n\n[Output truncated: {} bytes omitted. Consider refining your query to return fewer results.]",
        &output[..boundary], omitted
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        let s = "short".to_string();
        assert_eq!(truncate_tool_output(s.clone()), s);
    }

    #[test]
    fn test_truncate_long() {
        let result = truncate_tool_output("a".repeat(40000));
        assert!(result.contains("truncated"));
        assert!(result.len() < 40000);
    }

    #[test]
    fn test_truncate_utf8_boundary() {
        let result = truncate_tool_output("🦀".repeat(20000));
        assert!(result.contains("truncated"));
    }
}
