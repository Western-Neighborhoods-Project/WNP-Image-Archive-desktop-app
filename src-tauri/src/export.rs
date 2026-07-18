/// export.rs — Image resizing and zip archive creation for order fulfillment.
use image::imageops::FilterType;
use image::codecs::jpeg::JpegEncoder;
use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

/// Resize a source image to fit within `max_dimension` (longest side), writing
/// a JPEG at `dest_path`. If the source image is already smaller than max_dimension
/// in both dimensions it is still re-encoded as JPEG to ensure a consistent output format.
pub fn resize_image_to_path(
    src_path: &Path,
    dest_path: &Path,
    max_dimension: u32,
    quality: u8,
) -> Result<(), String> {
    // `image::open` defaults to a 512MB allocation cap that uncompressed
    // archival TIFFs blow past routinely — the same reason thumbnails.rs runs
    // with limits disabled. Without this, the exact files that thumbnail fine
    // fail order fulfillment and share-link creation. Source files come from
    // the user's own archive (no untrusted-input vector), so disable the cap.
    let mut reader = image::ImageReader::open(src_path)
        .map_err(|e| format!("Failed to open image {:?}: {}", src_path, e))?
        .with_guessed_format()
        .map_err(|e| format!("Failed to guess format for {:?}: {}", src_path, e))?;
    reader.no_limits();
    let img = reader
        .decode()
        .map_err(|e| format!("Failed to decode image {:?}: {}", src_path, e))?;

    let (w, h) = (img.width(), img.height());
    let resized = if w > max_dimension || h > max_dimension {
        img.resize(max_dimension, max_dimension, FilterType::Lanczos3)
    } else {
        // Encode at original size — avoid upscaling
        img
    };

    let file = fs::File::create(dest_path)
        .map_err(|e| format!("Failed to create output file {:?}: {}", dest_path, e))?;
    let writer = BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(writer, quality);
    encoder
        .encode_image(&resized)
        .map_err(|e| format!("Failed to encode JPEG {:?}: {}", dest_path, e))?;

    Ok(())
}

/// Bundle all files in `file_paths` into a single zip at `zip_dest`.
/// Each entry inside the zip uses the filename component of the source path.
pub fn create_zip(file_paths: &[PathBuf], zip_dest: &Path) -> Result<(), String> {
    let file = fs::File::create(zip_dest)
        .map_err(|e| format!("Failed to create zip file {:?}: {}", zip_dest, e))?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for src in file_paths {
        let entry_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("Invalid filename in path {:?}", src))?;

        let data = fs::read(src)
            .map_err(|e| format!("Failed to read file {:?}: {}", src, e))?;

        archive
            .start_file(entry_name, options)
            .map_err(|e| format!("Failed to start zip entry {}: {}", entry_name, e))?;

        use std::io::Write;
        archive
            .write_all(&data)
            .map_err(|e| format!("Failed to write zip entry {}: {}", entry_name, e))?;
    }

    archive
        .finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;

    Ok(())
}
