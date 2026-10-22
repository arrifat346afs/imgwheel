use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;

pub fn is_ghostscript_available() -> bool {
    which::which("gs").is_ok()
}

pub fn decode_eps(path: &Path, dpi: u32) -> Result<DynamicImage> {
    if !is_ghostscript_available() {
        anyhow::bail!("Ghostscript ('gs') is not installed. Required for reading EPS files.");
    }

    let temp_file = Builder::new()
        .prefix("imgwheel_eps_")
        .suffix(".png")
        .tempfile()
        .context("Failed to create temp file for EPS rasterization")?;
    let temp_png_path = temp_file.path().to_path_buf();
    // Drop file handle so Ghostscript has exclusive write access
    drop(temp_file);

    let output = Command::new("gs")
        .arg("-q")
        .arg("-dNOPAUSE")
        .arg("-dBATCH")
        .arg("-dSAFER")
        .arg("-sDEVICE=pngalpha")
        .arg(format!("-r{}", dpi))
        .arg(format!("-sOutputFile={}", temp_png_path.display()))
        .arg(path)
        .output()
        .with_context(|| format!("Failed to run Ghostscript on {:?}", path))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&temp_png_path);
        anyhow::bail!("Ghostscript failed to rasterize EPS: {}", err_msg);
    }

    let img_res = image::open(&temp_png_path)
        .with_context(|| "Failed to read rasterized PNG from Ghostscript");
    let _ = std::fs::remove_file(&temp_png_path);
    img_res
}

pub fn encode_eps(img: &RgbImage, output_path: &Path) -> Result<()> {
    let width = img.width();
    let height = img.height();

    let mut file = File::create(output_path)
        .with_context(|| format!("Failed to create EPS file at {:?}", output_path))?;

    writeln!(file, "%!PS-Adobe-3.0 EPSF-3.0")?;
    writeln!(file, "%%Creator: ImgWheel")?;
    writeln!(
        file,
        "%%Title: {}",
        output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    )?;
    writeln!(file, "%%BoundingBox: 0 0 {} {}", width, height)?;
    writeln!(file, "%%Pages: 1")?;
    writeln!(file, "%%LanguageLevel: 2")?;
    writeln!(file, "%%EndComments")?;
    writeln!(file, "%%BeginProlog")?;
    writeln!(file, "%%EndProlog")?;
    writeln!(file, "%%Page: 1 1")?;
    writeln!(file, "save")?;
    writeln!(file, "/picstr {} string def", width * 3)?;
    writeln!(file, "0 0 translate")?;
    writeln!(file, "{} {} scale", width, height)?;
    writeln!(
        file,
        "{} {} 8 [ {} 0 0 -{} 0 {} ]",
        width, height, width, height, height
    )?;
    writeln!(
        file,
        "{{ currentfile picstr readhexstring pop }} false 3 colorimage"
    )?;

    let raw = img.as_raw();
    let mut line_buf = String::with_capacity(80);

    for &byte in raw {
        use std::fmt::Write as FmtWrite;
        write!(line_buf, "{:02X}", byte).unwrap();
        if line_buf.len() >= 72 {
            writeln!(file, "{}", line_buf)?;
            line_buf.clear();
        }
    }

    if !line_buf.is_empty() {
        writeln!(file, "{}", line_buf)?;
    }

    writeln!(file, "showpage")?;
    writeln!(file, "restore")?;
    writeln!(file, "%%EOF")?;
    file.flush()?;

    Ok(())
}
