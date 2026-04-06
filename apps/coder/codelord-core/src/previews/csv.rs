//! CSV parser for preview rendering.
//!
//! Parses CSV content into structured data for table rendering.
//! Follows DOD principles: data is transformed once and cached.

// maybe move to swisskit-renderer.

use csv::ReaderBuilder;

/// Maximum rows to display in table view (performance limit).
const MAX_DISPLAY_ROWS: usize = 1000;

/// Parsed CSV data ready for rendering.
#[derive(Debug, Clone)]
pub struct CsvData {
  /// Column headers (first row or auto-generated).
  pub headers: Vec<String>,
  /// Data rows (limited to MAX_DISPLAY_ROWS).
  pub rows: Vec<Vec<String>>,
  /// Total row count before limiting (for displaying "showing X of Y rows").
  pub total_rows: usize,
  /// Parse error message if CSV is malformed.
  pub parse_error: Option<String>,
}

impl CsvData {
  /// Creates an empty CSV data structure with an error message.
  pub fn error(message: String) -> Self {
    Self {
      headers: Vec::new(),
      rows: Vec::new(),
      total_rows: 0,
      parse_error: Some(message),
    }
  }

  /// Checks if there was a parse error.
  pub fn has_error(&self) -> bool {
    self.parse_error.is_some()
  }

  /// Returns true if the table was truncated (more rows than displayed).
  pub fn is_truncated(&self) -> bool {
    self.total_rows > self.rows.len()
  }
}

/// Parses CSV content into structured data.
///
/// This function:
/// - Detects if first row is headers (all non-numeric strings)
/// - Limits to MAX_DISPLAY_ROWS for performance
/// - Handles malformed CSV gracefully
/// - Pads rows to match column count
pub fn parse_csv(content: &str) -> CsvData {
  // Preprocess: remove spaces before quotes to make non-compliant CSVs work
  // This handles cases like: `, "value"` -> `,"value"`
  let normalized_content = content.replace(", \"", ",\"");

  // Create CSV reader with flexible settings
  let mut reader = ReaderBuilder::new()
    .has_headers(false) // We'll detect headers manually
    .flexible(true) // Allow variable column counts
    .trim(csv::Trim::All) // Trim whitespace around fields
    .from_reader(normalized_content.as_bytes());

  let mut all_rows: Vec<Vec<String>> = Vec::new();

  // Parse all rows
  for result in reader.records() {
    match result {
      Ok(record) => {
        let row: Vec<String> = record
          .iter()
          .map(|field| {
            // Strip surrounding quotes if present (for non-compliant CSV files)
            let trimmed = field.trim();
            if trimmed.len() >= 2
              && trimmed.starts_with('"')
              && trimmed.ends_with('"')
            {
              trimmed[1..trimmed.len() - 1].to_string()
            } else {
              field.to_string()
            }
          })
          .collect();
        all_rows.push(row);
      }
      Err(error) => {
        return CsvData::error(format!("CSV parse error: {error}"));
      }
    }
  }

  if all_rows.is_empty() {
    return CsvData {
      headers: vec!["Empty CSV".to_string()],
      rows: Vec::new(),
      total_rows: 0,
      parse_error: None,
    };
  }

  // Determine column count (max columns in any row)
  let column_count = all_rows.iter().map(|row| row.len()).max().unwrap_or(0);

  if column_count == 0 {
    return CsvData::error("No columns detected in CSV".to_string());
  }

  // Auto-detect headers: check if first row looks like headers
  // Headers are typically non-numeric strings
  let first_row_is_headers = if let Some(first_row) = all_rows.first() {
    first_row.iter().all(|cell| {
      // Consider it a header if it's not a pure number
      cell.parse::<f64>().is_err() && !cell.is_empty()
    })
  } else {
    false
  };

  let (headers, data_rows) = if first_row_is_headers && all_rows.len() > 1 {
    // Use first row as headers
    let mut headers = all_rows[0].clone();
    // Pad headers if needed
    while headers.len() < column_count {
      headers.push(format!("Column {}", headers.len() + 1));
    }
    (headers, &all_rows[1..])
  } else {
    // Generate headers
    let headers: Vec<String> =
      (1..=column_count).map(|i| format!("Column {i}")).collect();
    (headers, all_rows.as_slice())
  };

  let total_rows = data_rows.len();

  // Limit rows for performance
  let display_rows: Vec<Vec<String>> = data_rows
    .iter()
    .take(MAX_DISPLAY_ROWS)
    .map(|row| {
      let mut padded_row = row.clone();
      // Pad rows to match column count
      while padded_row.len() < column_count {
        padded_row.push(String::new());
      }
      // Truncate if row is too long
      padded_row.truncate(column_count);
      padded_row
    })
    .collect();

  CsvData {
    headers,
    rows: display_rows,
    total_rows,
    parse_error: None,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_simple_csv() {
    let content = "Name,Age,City\nAlice,30,NYC\nBob,25,London";
    let data = parse_csv(content);

    assert!(!data.has_error());
    assert_eq!(data.headers, vec!["Name", "Age", "City"]);
    assert_eq!(data.rows.len(), 2);
    assert_eq!(data.rows[0], vec!["Alice", "30", "NYC"]);
  }

  #[test]
  fn test_parse_no_headers() {
    let content = "1,2,3\n4,5,6";
    let data = parse_csv(content);

    assert!(!data.has_error());
    assert_eq!(data.headers, vec!["Column 1", "Column 2", "Column 3"]);
    assert_eq!(data.rows.len(), 2);
  }

  #[test]
  fn test_parse_empty() {
    let content = "";
    let data = parse_csv(content);

    assert!(!data.has_error());
    assert_eq!(data.rows.len(), 0);
  }

  #[test]
  fn test_truncation() {
    // Create CSV with more than MAX_DISPLAY_ROWS
    let mut content = "A,B,C\n".to_string();
    for i in 0..1500 {
      content.push_str(&format!("{},{},{}\n", i, i + 1, i + 2));
    }

    let data = parse_csv(&content);

    assert!(!data.has_error());
    assert_eq!(data.rows.len(), MAX_DISPLAY_ROWS);
    assert_eq!(data.total_rows, 1500);
    assert!(data.is_truncated());
  }

  #[test]
  fn test_quoted_fields_with_commas() {
    // Test that quoted fields containing commas are parsed correctly
    let content = r#"Name,Address,Phone
Alice,"123 Main St, Apt 4",555-1234
Bob,"456 Oak Ave, Suite 200",555-5678"#;

    let data = parse_csv(content);

    assert!(!data.has_error());
    assert_eq!(data.headers, vec!["Name", "Address", "Phone"]);
    assert_eq!(data.rows.len(), 2);
    assert_eq!(
      data.rows[0],
      vec!["Alice", "123 Main St, Apt 4", "555-1234"]
    );
    assert_eq!(
      data.rows[1],
      vec!["Bob", "456 Oak Ave, Suite 200", "555-5678"]
    );
  }

  #[test]
  fn test_quoted_fields_with_quotes() {
    // Test that escaped quotes within quoted fields work
    let content = r#"Name,Quote
Alice,"She said ""Hello"""
Bob,"He said ""Goodbye"""#;

    let data = parse_csv(content);

    assert!(!data.has_error());
    assert_eq!(data.headers, vec!["Name", "Quote"]);
    assert_eq!(data.rows.len(), 2);
    assert_eq!(data.rows[0], vec!["Alice", r#"She said "Hello""#]);
    assert_eq!(data.rows[1], vec!["Bob", r#"He said "Goodbye""#]);
  }

  #[test]
  fn test_oscar_format() {
    // Test with the exact format from oscar_age_male.csv
    // This file has spaces before quotes (`, "value"`), which is non-RFC-4180
    // compliant but common in real-world CSV files.
    let content = r#""Index", "Year", "Age", "Name", "Movie"
 1, 1928, 44, "Emil Jannings", "The Last Command, The Way of All Flesh"
 2, 1929, 41, "Warner Baxter", "In Old Arizona""#;

    let data = parse_csv(content);

    assert!(!data.has_error());
    assert_eq!(data.headers, vec!["Index", "Year", "Age", "Name", "Movie"]);
    assert_eq!(data.rows.len(), 2);
    // Check that the movie field with comma is parsed as a single field
    assert_eq!(
      data.rows[0],
      vec![
        "1",
        "1928",
        "44",
        "Emil Jannings",
        "The Last Command, The Way of All Flesh"
      ]
    );
    assert_eq!(
      data.rows[1],
      vec!["2", "1929", "41", "Warner Baxter", "In Old Arizona"]
    );
  }
}
