#![warn(clippy::all, clippy::pedantic)]
// Allow certain clippy lints that are acceptable for our image processing use case
#![allow(
    clippy::cast_possible_truncation, // Acceptable for image processing
    clippy::cast_sign_loss,           // Acceptable for coordinate conversions
    clippy::cast_precision_loss,      // Acceptable for font rendering
    clippy::cast_possible_wrap        // Acceptable for image dimensions
)]

use ab_glyph::{Font, FontRef, Point, PxScale, point};
use anyhow::{Context, Result};
use clap::Parser;
use image::{Rgb, RgbImage};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of image file paths
    #[arg(required = true)]
    images: Vec<PathBuf>,

    /// List of optional labels for each image
    #[arg(long)]
    labels: Vec<String>,

    /// Output file name for the generated plot
    #[arg(long, default_value = "output.jpg")]
    output: PathBuf,

    /// Number of rows to display the images
    #[arg(long, default_value_t = 1)]
    rows: u32,

    /// List of optional labels for each row
    #[arg(long)]
    row_labels: Vec<String>,

    /// List of optional labels for each column
    #[arg(long)]
    column_labels: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    save_image_plot(&args)
}

fn draw_text(
    canvas: &mut RgbImage,
    text: &str,
    x: i32,
    y: i32,
    scale: f32,
    font: &FontRef,
    color: Rgb<u8>,
) {
    let px_scale = PxScale::from(scale);

    // Calculate total text width for centering
    let mut total_width = 0.0;
    let glyphs: Vec<_> = text.chars().map(|c| {
        let glyph_id = font.glyph_id(c);
        let advance = font.h_advance_unscaled(glyph_id);
        total_width += advance;
        glyph_id
    }).collect();

    // Start position, accounting for total width to center the text
    let start_x = x - ((total_width * scale) / 2.0) as i32;
    let mut x_offset = 0.0;

    // Draw each character
    for glyph_id in glyphs {
        let position = Point {
            x: (start_x as f32 + x_offset),
            y: y as f32,
        };

        let glyph = glyph_id.with_scale_and_position(px_scale, position);

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, coverage| {
                let x = bounds.min.x as i32 + gx as i32;
                let y = bounds.min.y as i32 + gy as i32;
                if x >= 0 && y >= 0 && x < canvas.width() as i32 && y < canvas.height() as i32 {
                    let pixel = canvas.get_pixel_mut(x as u32, y as u32);
                    let coverage = (coverage * 255.0) as u8;
                    *pixel = Rgb([
                        ((255 - coverage) + coverage * color[0] / 255),
                        ((255 - coverage) + coverage * color[1] / 255),
                        ((255 - coverage) + coverage * color[2] / 255),
                    ]);
                }
            });
        }
        x_offset += font.h_advance_unscaled(glyph_id) * scale;
    }
}

fn save_image_plot(args: &Args) -> Result<()> {
    let images = &args.images;
    let labels = &args.labels;
    let row_labels = &args.row_labels;
    let column_labels = &args.column_labels;
    let rows = args.rows;

    // Validate inputs
    if !labels.is_empty() && labels.len() != images.len() {
        anyhow::bail!("Number of labels should match the number of images");
    }

    if !row_labels.is_empty() && row_labels.len() != rows as usize {
        anyhow::bail!("Number of row labels should match the number of rows");
    }

    let cols = u32::try_from(images.len())
        .map_err(|_| anyhow::anyhow!("Too many images"))?
        .div_ceil(rows);

    if !column_labels.is_empty() && column_labels.len() != cols as usize {
        anyhow::bail!("Number of column labels should match the number of columns");
    }

    // Read the first image to determine dimensions
    let first_image = image::open(&images[0])
        .with_context(|| format!("Failed to open first image: {:?}", &images[0]))?
        .to_rgb8();
    let (image_width, image_height) = first_image.dimensions();

    // Define canvas dimensions
    let top_padding: u32 = 50; // Space for labels above images
    let label_height: u32 = 30; // Height for each label
    let left_padding = if row_labels.iter().any(|l| !l.is_empty()) {
        40
    } else {
        0
    };

    // Calculate canvas dimensions with space for labels
    let has_labels = !labels.is_empty() || !row_labels.is_empty() || !column_labels.is_empty();
    let row_height = image_height + (if has_labels { top_padding } else { 0 });
    let canvas_height = row_height * rows + (if has_labels { top_padding } else { 0 });
    let canvas_width = image_width * cols + left_padding;

    // Create canvas
    let mut canvas = RgbImage::new(canvas_width, canvas_height);
    // Fill with white
    for pixel in canvas.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }

    // Load font
    let font_data = include_bytes!("../assets/DejaVuSans.ttf");
    let font = FontRef::try_from_slice(font_data).context("Failed to load font")?;
    let scale = 20.0;
    let color = Rgb([0, 0, 0]);

    // Add column labels
    for (i, label) in column_labels.iter().enumerate() {
        let x = i32::try_from(u32::try_from(i)? * image_width + left_padding + image_width / 2)
            .map_err(|_| anyhow::anyhow!("Position overflow"))?;
        draw_text(&mut canvas, label, x, label_height as i32 / 2, scale, &font, color);
    }

    // Place images and labels
    for (i, img_path) in images.iter().enumerate() {
        let i = u32::try_from(i)?;
        let row = i / cols;
        let col = i % cols;

        // Calculate positions
        let x_start = col * image_width + left_padding;
        let y_start = row * row_height + top_padding;

        // Add image label if provided (above the image)
        if i < u32::try_from(labels.len())? {
            let x = i32::try_from(x_start + image_width / 2)
                .map_err(|_| anyhow::anyhow!("Position overflow"))?;
            let y = i32::try_from(y_start - label_height)
                .map_err(|_| anyhow::anyhow!("Position overflow"))?;
            draw_text(&mut canvas, &labels[i as usize], x, y, scale, &font, color);
        }

        // Add row label
        if row < u32::try_from(row_labels.len())? {
            draw_text(
                &mut canvas,
                &row_labels[row as usize],
                5,
                y_start as i32 + (image_height / 2) as i32,
                scale,
                &font,
                color,
            );
        }

        // Load and place image
        let img = image::open(img_path)
            .with_context(|| format!("Failed to open image: {img_path:?}"))?
            .to_rgb8();

        // Copy image to canvas
        for (x, y, pixel) in img.enumerate_pixels() {
            if x_start + x < canvas_width && y_start + y < canvas_height {
                canvas.put_pixel(x_start + x, y_start + y, *pixel);
            }
        }
    }

    // Save the generated plot
    canvas
        .save(&args.output)
        .with_context(|| format!("Failed to save output image: {:?}", args.output))?;

    println!("Generated plot saved as {:?}", args.output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_image(path: &std::path::Path, width: u32, height: u32) -> Result<()> {
        let mut img = RgbImage::new(width, height);
        // Fill with a test pattern
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([((x * 255) / width) as u8, ((y * 255) / height) as u8, 128u8]);
        }
        img.save(path)?;
        Ok(())
    }

    #[test]
    fn test_basic_plot() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        // Create test images
        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
            labels: vec![],
            output: output_path.clone(),
            rows: 1,
            row_labels: vec![],
            column_labels: vec![],
        };

        save_image_plot(&args)?;
        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    fn test_with_labels() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        // Create test images
        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
            labels: vec!["Label 1".to_string(), "Label 2".to_string()],
            output: output_path.clone(),
            rows: 1,
            row_labels: vec![],
            column_labels: vec![],
        };

        save_image_plot(&args)?;
        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    fn test_with_row_and_column_labels() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        // Create test images
        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
            labels: vec!["Label 1".to_string(), "Label 2".to_string()],
            output: output_path.clone(),
            rows: 2,
            row_labels: vec!["Row 1".to_string(), "Row 2".to_string()],
            column_labels: vec!["Col 1".to_string()],
        };

        save_image_plot(&args)?;
        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    #[should_panic(expected = "Number of labels should match the number of images")]
    fn test_mismatched_labels() {
        let temp_dir = tempdir().unwrap();
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        // Create test images
        create_test_image(&img1_path, 100, 100).unwrap();
        create_test_image(&img2_path, 100, 100).unwrap();

        let args = Args {
            images: vec![img1_path, img2_path],
            labels: vec!["Label 1".to_string()], // Only one label for two images
            output: output_path,
            rows: 1,
            row_labels: vec![],
            column_labels: vec![],
        };

        save_image_plot(&args).unwrap();
    }

    #[test]
    #[should_panic(expected = "Number of row labels should match the number of rows")]
    fn test_mismatched_row_labels() {
        let temp_dir = tempdir().unwrap();
        let img1_path = temp_dir.path().join("test1.png");
        let output_path = temp_dir.path().join("output.png");

        // Create test image
        create_test_image(&img1_path, 100, 100).unwrap();

        let args = Args {
            images: vec![img1_path],
            labels: vec![],
            output: output_path,
            rows: 1,
            row_labels: vec!["Row 1".to_string(), "Row 2".to_string()], // Two row labels for one row
            column_labels: vec![],
        };

        save_image_plot(&args).unwrap();
    }
}
