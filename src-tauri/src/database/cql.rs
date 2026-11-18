/// Split CQL into the same statements used for authorization and JDBC execution.
pub(crate) fn split_statements(script: &str) -> Result<Vec<&str>, &'static str> {
    let bytes = script.as_bytes();
    let mut statements = Vec::new();
    let mut start = None;
    let mut end = 0;
    let mut in_batch = false;
    let mut after_apply = false;
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        let remaining = &bytes[i..];
        if byte.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if remaining.starts_with(b"--") || remaining.starts_with(b"//") {
            while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                i += 1;
            }
            continue;
        }
        if remaining.starts_with(b"/*") {
            i = scan_delimited(script, i + 2, "*/")?;
            continue;
        }
        if byte == b';' && !in_batch {
            if let Some(start) = start.take() {
                statements.push(&script[start..end]);
            }
            i += 1;
            continue;
        }

        let first_token = start.is_none();
        start.get_or_insert(i);
        if matches!(byte, b'\'' | b'"') {
            i = scan_quoted(bytes, i, byte)?;
            after_apply = false;
        } else if remaining.starts_with(b"$$") {
            i = scan_delimited(script, i + 2, "$$")?;
            after_apply = false;
        } else if byte.is_ascii_alphanumeric() || byte == b'_' {
            let word_start = i;
            while bytes
                .get(i)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                i += 1;
            }
            let word = &script[word_start..i];
            if first_token && word.eq_ignore_ascii_case("BEGIN") {
                in_batch = true;
            } else if in_batch && after_apply && word.eq_ignore_ascii_case("BATCH") {
                in_batch = false;
            }
            after_apply = in_batch && word.eq_ignore_ascii_case("APPLY");
        } else {
            i += 1;
            after_apply = false;
        }
        end = i;
    }
    if in_batch {
        return Err("CQL batch is missing APPLY BATCH");
    }
    if let Some(start) = start {
        statements.push(&script[start..end]);
    }
    Ok(statements)
}

fn scan_delimited(script: &str, start: usize, delimiter: &str) -> Result<usize, &'static str> {
    script[start..]
        .find(delimiter)
        .map(|offset| start + offset + delimiter.len())
        .ok_or("Unterminated CQL comment or dollar-quoted literal")
}

fn scan_quoted(bytes: &[u8], start: usize, quote: u8) -> Result<usize, &'static str> {
    let mut i = start + 1;
    while let Some(&byte) = bytes.get(i) {
        i += 1;
        if byte == quote {
            if bytes.get(i) != Some(&quote) {
                return Ok(i);
            }
            i += 1;
        }
    }
    Err("Unterminated CQL quoted identifier or literal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cql_preserves_batches_and_splits_following_statements() {
        let batch = "BEGIN UNLOGGED BATCH\n INSERT INTO events (id) VALUES (1);\n INSERT INTO events (id) VALUES (2);\n APPLY /* end */ BATCH";
        let script = format!("// header\r\n{batch}; -- end\nSELECT * FROM events;");
        assert_eq!(
            split_statements(&script),
            Ok(vec![batch, "SELECT * FROM events"])
        );
        assert!(split_statements("BEGIN BATCH INSERT INTO events (id) VALUES (1);").is_err());
    }

    #[test]
    fn cql_ignores_quoted_separators_and_preserves_internal_comments() {
        let select =
            "SELECT \"part;name\" FROM events WHERE value = $$a;b$$ /* note */ ALLOW FILTERING";
        assert_eq!(
            split_statements(&format!("{select}; // tail")),
            Ok(vec![select])
        );
        assert!(split_statements("SELECT * FROM events /* unfinished").is_err());
    }
}
