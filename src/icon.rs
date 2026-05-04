use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat};
use regex::Regex;
use std::fs;
use std::io::Read;
use std::path::Path;
use url::Url;

pub fn write_rgba_png(img: DynamicImage, output_path: &Path) -> Result<()> {
    let rgba = img.to_rgba8();
    let rgba_img = DynamicImage::ImageRgba8(rgba);
    rgba_img
        .save_with_format(output_path, ImageFormat::Png)
        .with_context(|| format!("Failed to write RGBA PNG icon to {}", output_path.display()))?;
    Ok(())
}

pub fn download_icon_from_url(icon_url: &str, output_path: &Path) -> Result<bool> {
    let response = match ureq::get(icon_url).call() {
        Ok(resp) => resp,
        Err(_) => return Ok(false),
    };
    let mut bytes = Vec::new();
    if response.into_reader().read_to_end(&mut bytes).is_err() || bytes.is_empty() {
        return Ok(false);
    }
    let img = match image::load_from_memory(&bytes) {
        Ok(img) => img,
        Err(_) => return Ok(false),
    };
    write_rgba_png(img, output_path)?;
    Ok(true)
}

pub fn extract_icon_candidates_from_html(base_url: &Url, html: &str) -> Vec<String> {
    let link_re = Regex::new(r#"(?is)<link\s+[^>]*rel\s*=\s*["'][^"']*icon[^"']*["'][^>]*href\s*=\s*["']([^"']+)["'][^>]*>"#).unwrap();
    let mut candidates = Vec::new();
    for cap in link_re.captures_iter(html) {
        if let Some(href) = cap.get(1)
            && let Ok(joined) = base_url.join(href.as_str())
        {
            let candidate = joined.to_string();
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }
    candidates
}

pub fn try_download_site_icon(website_url: &str, output_path: &Path) -> Result<bool> {
    let base_url = match Url::parse(website_url) {
        Ok(url) => url,
        Err(_) => return Ok(false),
    };

    let mut html = String::new();
    if let Ok(response) = ureq::get(website_url).call()
        && response.into_reader().read_to_string(&mut html).is_ok()
    {
        for icon_url in extract_icon_candidates_from_html(&base_url, &html) {
            if download_icon_from_url(&icon_url, output_path)? {
                return Ok(true);
            }
        }
    }

    if let Ok(favicon_url) = base_url.join("/favicon.ico")
        && download_icon_from_url(favicon_url.as_str(), output_path)?
    {
        return Ok(true);
    }

    Ok(false)
}

pub fn fetch_or_create_icon(
    website_url: &str,
    custom_icon: Option<&String>,
    output_path: &Path,
) -> Result<()> {
    if let Some(icon_path) = custom_icon {
        if Path::new(icon_path).exists() {
            let img = image::open(icon_path)
                .with_context(|| format!("Failed to read custom icon image from {}", icon_path))?;
            write_rgba_png(img, output_path)?;
            return Ok(());
        } else {
            eprintln!(
                "Warning: Custom icon path '{}' does not exist, falling back to downloaded icon.",
                icon_path
            );
        }
    }

    if try_download_site_icon(website_url, output_path)? {
        return Ok(());
    }

    if let Ok(parsed_url) = Url::parse(website_url)
        && let Some(host) = parsed_url.host_str()
    {
        println!("Falling back to Google favicon API for {}...", host);
        let api_url = format!("https://www.google.com/s2/favicons?domain={}&sz=128", host);
        if download_icon_from_url(&api_url, output_path)? {
            return Ok(());
        }
    }

    let dummy_png: [u8; 67] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    fs::write(output_path, dummy_png).context("Failed to write dummy icon fallback")?;

    Ok(())
}
