use std::fs;
use std::path::Path;

/// Parse Oracle tnsnames.ora from a directory and return the list of TNS alias names.
///
/// The parser looks for lines that start a TNS entry: a word followed by `=`.
/// Lines starting with `#` (comments) or whitespace are skipped.
/// Tries common filename variants: `tnsnames.ora`, `TNSNAMES.ORA`.
pub fn parse_tns_aliases(tns_admin_dir: &str) -> Vec<String> {
    let dir = Path::new(tns_admin_dir);
    
    // Try common filename variants
    let filenames = ["tnsnames.ora", "TNSNAMES.ORA", "Tnsnames.ora"];
    let content = filenames.iter()
        .find_map(|name| fs::read_to_string(dir.join(name)).ok());

    let content = match content {
        Some(c) => c,
        None => return Vec::new(),
    };

    let mut aliases: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Match "alias_name =" or "alias_name=" at the start of a line (after trimming)
        if let Some(eq_pos) = trimmed.find('=') {
            let before_eq = trimmed[..eq_pos].trim();
            if !before_eq.is_empty()
                && !before_eq.contains(' ')
                && !before_eq.contains('(')
                && !before_eq.contains(')')
            {
                let alias = before_eq.to_string();
                if !aliases.contains(&alias) {
                    aliases.push(alias);
                }
            }
        }
    }

    aliases.sort();
    aliases
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn with_temp_tnsnames(id: &str, content: &str) -> String {
        let dir = std::env::temp_dir().join(format!("tns_test_{}", id));
        let _ = fs::create_dir_all(&dir);
        let file_path = dir.join("tnsnames.ora");
        let mut file = fs::File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        dir.to_string_lossy().to_string()
    }

    #[test]
    fn test_parse_tns_aliases_from_file() {
        let dir_path = with_temp_tnsnames(
            "from_file",
            r#"dbname_medium =
  (DESCRIPTION =
    (ADDRESS = (PROTOCOL = tcps)(HOST = adb.example.com)(PORT = 1522))
    (CONNECT_DATA =
      (SERVER = DEDICATED)
      (SERVICE_NAME = dbname_medium.adb.example.com)
    )
  )

dbname_low =
  (DESCRIPTION =
    (ADDRESS = (PROTOCOL = tcps)(HOST = adb.example.com)(PORT = 1522))
    (CONNECT_DATA =
      (SERVICE_NAME = dbname_low.adb.example.com)
    )
  )

# This is a comment
dbname_high = (DESCRIPTION=(ADDRESS=...))"#,
        );

        let aliases = parse_tns_aliases(&dir_path);
        assert_eq!(aliases.len(), 3);
        assert!(aliases.contains(&"dbname_high".to_string()));
        assert!(aliases.contains(&"dbname_low".to_string()));
        assert!(aliases.contains(&"dbname_medium".to_string()));
        let _ = fs::remove_dir_all(std::env::temp_dir().join("tns_test_from_file"));
    }

    #[test]
    fn test_parse_tns_aliases_missing_file() {
        let aliases = parse_tns_aliases("/tmp/nonexistent_dir_tns_test_99999");
        assert!(aliases.is_empty());
    }

    #[test]
    fn test_parse_tns_aliases_empty_file() {
        let dir_path = with_temp_tnsnames("empty_file", "");
        let aliases = parse_tns_aliases(&dir_path);
        assert!(aliases.is_empty());
        let _ = fs::remove_dir_all(std::env::temp_dir().join("tns_test_empty_file"));
    }
}
