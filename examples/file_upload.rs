// ============================================================
// Example: File Upload
// مثال: رفع الملفات
// ============================================================

use noor::core::upload::{FileUploader, UploadConfig};
use noor::NoorResult;

fn main() -> NoorResult<()> {
    println!("{}", noor::banner());
    println!("\n📤 File Upload Demo\n");

    // Create a temp upload dir for the demo so we don't litter the workspace.
    let upload_dir = "/tmp/noor_upload_demo";
    std::fs::remove_dir_all(upload_dir).ok();

    let mut config = UploadConfig::default();
    config.upload_dir = upload_dir.to_string();
    let uploader = FileUploader::new(config)?;

    // Validate different file types. `validate(name, mime, size)` checks
    // size and extension without writing anything to disk.
    let test_files: Vec<(&str, &str, usize)> = vec![
        ("photo.jpg", "image/jpeg", 2_500_000),
        ("document.pdf", "application/pdf", 5_000_000),
        ("video.mp4", "video/mp4", 50_000_000),
        ("malware.exe", "application/octet-stream", 1_000_000),
        ("huge.jpg", "image/jpeg", 200_000_000),
    ];

    for (filename, mime, size) in test_files {
        let result = uploader.validate(filename, mime, size);
        let status = if result.is_ok() { "✓ Valid" } else { "✗ Invalid" };
        let size_str = FileUploader::format_size(size as u64);
        println!("  {} {} ({}, {})", status, filename, mime, size_str);

        if let Err(e) = result {
            println!("    Error: {}", e);
        }
    }

    // Save a file (uses `store(name, content)`).
    println!("\n  Saving test file...");
    let uploaded = uploader.store("test.txt", b"Hello, Upload!")?;
    println!("  ✓ Saved: {}", uploaded.stored_name);
    println!("    Path: {}", uploaded.path);
    println!("    Size: {} bytes", uploaded.size);

    // Clean up. `delete(stored_name)` takes the stored name (not the full path).
    uploader.delete(&uploaded.stored_name)?;
    println!("\n  ✓ File deleted");

    std::fs::remove_dir_all(upload_dir).ok();
    println!("\n✅ Upload demo completed!");
    Ok(())
}
