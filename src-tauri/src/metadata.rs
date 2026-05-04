use crate::models::ExtractedMetadata;

/// Run exiftool on a directory and return the parsed entries. Used by
/// the legacy `extract_metadata_batch` command and by the Plan 13
/// background worker. Returns an Err only on subprocess-level failures
/// (binary missing, non-zero exit) — partial parses come back as a
/// (possibly-empty) Vec.
pub fn extract_metadata_for_directory(
    directory: &str,
    exiftool: &str,
) -> Result<Vec<ExtractedMetadata>, String> {
    let output = std::process::Command::new(exiftool)
        .args(["-json", "-r", "-fast2", "-q", "--", directory])
        .output()
        .map_err(|e| format!("Failed to run exiftool ({}): {}", exiftool, e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("exiftool exited with error: {}", stderr));
    }
    let json = String::from_utf8_lossy(&output.stdout);
    Ok(parse_exiftool_output(&json))
}

/// Parse a JSON array of ExifTool results into ExtractedMetadata structs.
/// Handles the adapter pattern: maps ExifTool field names to our schema.
fn parse_exiftool_output(json: &str) -> Vec<ExtractedMetadata> {
    let Ok(array) = serde_json::from_str::<Vec<serde_json::Value>>(json) else {
        return Vec::new();
    };

    array.into_iter().filter_map(parse_single_entry).collect()
}

/// Map a single ExifTool JSON object to our ExtractedMetadata struct.
/// This is the "exiftool adapter" — add alternative adapters (csv, json export)
/// as separate functions following the same ExtractedMetadata return type.
fn parse_single_entry(obj: serde_json::Value) -> Option<ExtractedMetadata> {
    let map = obj.as_object()?;

    let file_path = map
        .get("SourceFile")
        .and_then(|v| v.as_str())
        .map(str::to_string)?;

    // Title: prefer IPTC ObjectName, fall back to XMP Title
    let title = first_string(map, &["Title", "ObjectName"]);

    // Description: prefer IPTC Caption-Abstract, fall back to XMP Description
    let description = first_string(map, &["Description", "Caption-Abstract", "ImageDescription"]);

    // Location fields
    let city = first_string(map, &["City"]);
    let state = first_string(map, &["Province-State", "State"]);
    let country = first_string(map, &["Country-PrimaryLocationName", "Country"]);

    // Keywords — may be a string or an array in ExifTool JSON output
    let keywords: Option<Vec<String>> = map.get("Keywords").map(|v| match v {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        serde_json::Value::String(s) => {
            // Comma-separated string
            s.split(',').map(|k| k.trim().to_string()).collect()
        }
        _ => vec![],
    });

    // Date: prefer DateTimeOriginal (EXIF), then CreateDate (EXIF), then DateCreated (IPTC)
    let date_start = first_string(map, &["DateTimeOriginal", "CreateDate", "DateCreated"])
        .map(|d| normalize_date(&d));

    // Photographer / creator
    let photographer = first_string(map, &["Creator", "Artist", "By-line", "Author"]);

    // Usage rights
    let usage_rights = first_string(map, &["CopyrightNotice", "Rights", "Copyright"]);

    Some(ExtractedMetadata {
        file_path,
        title,
        description,
        city,
        state,
        country,
        keywords,
        date_start,
        photographer,
        usage_rights,
    })
}

/// Return the first non-empty string value found among the given keys.
fn first_string(
    map: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    for key in keys {
        if let Some(serde_json::Value::String(s)) = map.get(*key) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

/// Attempt to normalize a date string to ISO 8601 (YYYY-MM-DD).
/// ExifTool dates often look like "2023:06:15 10:30:00" or "2023:06:15".
fn normalize_date(s: &str) -> String {
    // ExifTool format: "YYYY:MM:DD HH:MM:SS"
    let date_part = s.split_whitespace().next().unwrap_or(s);
    date_part.replace(':', "-")
}

// Plan 13: the batch / single Tauri commands have been retired. The
// background_jobs worker now drives metadata extraction directly via
// extract_metadata_for_directory above; nothing in the frontend needs
// to invoke exiftool synchronously anymore.
