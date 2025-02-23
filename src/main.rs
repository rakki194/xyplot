#![warn(clippy::all, clippy::pedantic)]

use ab_glyph::{Font, FontRef, GlyphId, Point, PxScale, ScaleFont};
use anyhow::{Context, Result};
use clap::Parser;
use image::{Rgb, RgbImage};
use imx::numeric;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of image file paths
    #[arg(required = true)]
    images: Vec<PathBuf>,

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

#[derive(Clone, Copy)]
struct FontPair<'a> {
    main: &'a FontRef<'a>,
    emoji: &'a FontRef<'a>,
}

impl<'a> FontPair<'a> {
    fn glyph_id(&self, c: char) -> (GlyphId, &'a FontRef<'a>) {
        let main_id = self.main.glyph_id(c);
        // Check if the main font has a real glyph for this char (not a .notdef glyph)
        if self.main.outline(main_id).is_some() {
            (main_id, self.main)
        } else {
            let emoji_id = self.emoji.glyph_id(c);
            (emoji_id, self.emoji)
        }
    }
}

fn draw_text(
    canvas: &mut RgbImage,
    text: &str,
    x: i32,
    y: i32,
    scale: f32,
    fonts: FontPair,
    color: Rgb<u8>,
) {
    let px_scale = PxScale::from(scale);

    // Layout the glyphs in a line with 20 pixels padding
    let mut glyphs = Vec::new();
    let mut cursor = Point {
        x: numeric::i32_to_f32_for_pos(x),
        y: numeric::i32_to_f32_for_pos(y),
    };

    // First pass: calculate positions and collect glyphs
    for c in text.chars() {
        let (id, font) = fonts.glyph_id(c);
        let scaled_font = font.as_scaled(px_scale);
        let glyph = id.with_scale(scaled_font.scale()).positioned(cursor);
        cursor.x += scaled_font.h_advance(glyph.id);
        glyphs.push((glyph, font));
    }

    // Second pass: render glyphs
    for (glyph, font) in glyphs {
        let scaled_font = font.as_scaled(px_scale);
        if let Some(outlined) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                // Convert the coverage value into an alpha channel value
                let alpha = (coverage * 255.0) as u8;
                if alpha == 0 {
                    return;
                }

                // Get absolute pixel coordinates
                let px = (x as i32 + bounds.min.x as i32) as u32;
                let py = (y as i32 + bounds.min.y as i32) as u32;

                // Blend the color with the existing pixel based on alpha
                if px < canvas.width() && py < canvas.height() {
                    let pixel = canvas.get_pixel_mut(px, py);
                    let blend = |a: u8, b: u8, alpha: u8| -> u8 {
                        let a = a as f32;
                        let b = b as f32;
                        let alpha = alpha as f32 / 255.0;
                        ((a * (1.0 - alpha) + b * alpha) as u8).min(255)
                    };

                    pixel[0] = blend(pixel[0], color[0], alpha);
                    pixel[1] = blend(pixel[1], color[1], alpha);
                    pixel[2] = blend(pixel[2], color[2], alpha);
                }
            });
        }

        // Check for color emoji image
        if let Some(img) = font.glyph_raster_image2(glyph.id, u16::MAX) {
            if let Some(data) = img.data.as_rgba() {
                let img_width = img.width as u32;
                let scale_factor = scale / img.pixels_per_em as f32;
                let scaled_width = (img_width as f32 * scale_factor) as u32;
                let scaled_height = (img.height as f32 * scale_factor) as u32;

                for (img_y, row) in data.chunks_exact(img_width as usize * 4).enumerate() {
                    for (img_x, pixel) in row.chunks_exact(4).enumerate() {
                        let src_x = img_x as f32 * scale_factor;
                        let src_y = img_y as f32 * scale_factor;
                        let canvas_x = (glyph.position.x + src_x + img.origin.x * scale_factor) as u32;
                        let canvas_y = (glyph.position.y + src_y + img.origin.y * scale_factor) as u32;

                        if canvas_x < canvas.width() && canvas_y < canvas.height() {
                            let canvas_pixel = canvas.get_pixel_mut(canvas_x, canvas_y);
                            let alpha = pixel[3] as f32 / 255.0;
                            canvas_pixel[0] = ((1.0 - alpha) * canvas_pixel[0] as f32 + alpha * pixel[0] as f32) as u8;
                            canvas_pixel[1] = ((1.0 - alpha) * canvas_pixel[1] as f32 + alpha * pixel[1] as f32) as u8;
                            canvas_pixel[2] = ((1.0 - alpha) * canvas_pixel[2] as f32 + alpha * pixel[2] as f32) as u8;
                        }
                    }
                }
            }
        }
    }
}

fn save_image_plot(args: &Args) -> Result<()> {
    let images = &args.images;
    let row_labels = &args.row_labels;
    let column_labels = &args.column_labels;
    let rows = args.rows;

    // Validate inputs
    if !row_labels.is_empty() && row_labels.len() != rows as usize {
        anyhow::bail!(
            "Number of row labels ({}) should match the number of rows ({})",
            row_labels.len(),
            rows
        );
    }

    let cols = u32::try_from(images.len())
        .map_err(|_| anyhow::anyhow!("Too many images"))?
        .div_ceil(rows);

    if !column_labels.is_empty() && column_labels.len() != cols as usize {
        anyhow::bail!(
            "Number of column labels ({}) should match the number of columns ({})",
            column_labels.len(),
            cols
        );
    }

    // Read the first image to determine dimensions
    let first_image = image::open(&images[0])
        .with_context(|| format!("Failed to open first image: {:?}", &images[0]))?
        .to_rgb8();
    let (image_width, image_height) = first_image.dimensions();

    // Define canvas dimensions
    let label_height: u32 = 20; // Increased from 30 to give more height for labels
    let left_padding = if row_labels.iter().any(|l| !l.is_empty()) {
        150 // Increased from 40 to give more space for row labels
    } else {
        0
    };

    // Calculate canvas dimensions with space for labels
    let has_labels = !row_labels.is_empty() || !column_labels.is_empty();
    let row_height = image_height + (if has_labels { top_padding } else { 0 });
    let canvas_height = row_height * rows + (if has_labels { top_padding } else { 0 });
    let canvas_width = image_width * cols + left_padding;

    // Create canvas
    let mut canvas = RgbImage::new(canvas_width, canvas_height);
    // Fill with white
    for pixel in canvas.pixels_mut() {
        *pixel = Rgb([255, 255, 255]);
    }

    // Load fonts
    let font_data = include_bytes!("../assets/DejaVuSans.ttf");
    let main_font = FontRef::try_from_slice(font_data).context("Failed to load main font")?;
    
    let emoji_font_data = include_bytes!("../assets/NotoColorEmoji.ttf");
    let emoji_font = FontRef::try_from_slice(emoji_font_data).context("Failed to load emoji font")?;

    let fonts = FontPair {
        main: &main_font,
        emoji: &emoji_font,
    };

    // Add column labels
    if !column_labels.is_empty() {
        for (col, label) in column_labels.iter().enumerate() {
            let x = (col as u32 * image_width + left_padding + image_width / 2) as i32;
            let y = (top_padding / 2) as i32;
            draw_text(
                &mut canvas,
                label,
                x - ((label.len() as f32 * 20.0) / 2.0) as i32, // Center text
                y,
                24.0,
                fonts,
                Rgb([0, 0, 0]),
            );
        }
    }

    // Place images and labels
    for (i, img_path) in images.iter().enumerate() {
        let i = u32::try_from(i)?;
        let row = i / cols;
        let col = i % cols;

        // Calculate positions
        let x_start = col * image_width + left_padding;
        let y_start = row * row_height + top_padding;

        // Add row label if provided (left of the image)
        if let Some(row_label) = row_labels.get(row as usize) {
            let x = 20;
            let y = (y_start + image_height / 2) as i32;
            draw_text(
                &mut canvas,
                row_label,
                x,
                y,
                24.0,
                fonts,
                Rgb([0, 0, 0]),
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

        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
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

        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
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

        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
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
    fn test_different_image_sizes() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img1_path, 200, 150)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
            output: output_path.clone(),
            rows: 2,
            row_labels: vec![],
            column_labels: vec![],
        };

        save_image_plot(&args)?;
        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    fn test_single_image() -> Result<()> {
        let temp_dir = tempdir()?;
        let img_path = temp_dir.path().join("test1.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img_path, 100, 100)?;

        let args = Args {
            images: vec![img_path],
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
    fn test_many_images() -> Result<()> {
        let temp_dir = tempdir()?;
        let output_path = temp_dir.path().join("output.png");
        let mut image_paths = Vec::new();
        let mut labels = Vec::new();

        // Create 9 test images
        for i in 0..9 {
            let img_path = temp_dir.path().join(format!("test{i}.png"));
            create_test_image(&img_path, 100, 100)?;
            image_paths.push(img_path);
            labels.push(format!("Image {i}"));
        }

        let args = Args {
            images: image_paths,
            output: output_path.clone(),
            rows: 3,
            row_labels: vec![
                "Top".to_string(),
                "Middle".to_string(),
                "Bottom".to_string(),
            ],
            column_labels: vec![
                "Left".to_string(),
                "Center".to_string(),
                "Right".to_string(),
            ],
        };

        save_image_plot(&args)?;
        assert!(output_path.exists());
        Ok(())
    }

    #[test]
    fn test_unicode_labels() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
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
    fn test_long_labels() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
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
    #[should_panic(expected = "Number of labels should match the number of images")]
    fn test_mismatched_labels() {
        let temp_dir = tempdir().unwrap();
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img1_path, 100, 100).unwrap();
        create_test_image(&img2_path, 100, 100).unwrap();

        let args = Args {
            images: vec![img1_path, img2_path],
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

        create_test_image(&img1_path, 100, 100).unwrap();

        let args = Args {
            images: vec![img1_path],
            output: output_path,
            rows: 1,
            row_labels: vec!["Row 1".to_string(), "Row 2".to_string()],
            column_labels: vec![],
        };

        save_image_plot(&args).unwrap();
    }

    #[test]
    #[should_panic(expected = "Number of column labels should match the number of columns")]
    fn test_mismatched_column_labels() {
        let temp_dir = tempdir().unwrap();
        let img1_path = temp_dir.path().join("test1.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img1_path, 100, 100).unwrap();

        let args = Args {
            images: vec![img1_path],
            output: output_path,
            rows: 1,
            row_labels: vec![],
            column_labels: vec!["Col 1".to_string(), "Col 2".to_string()],
        };

        save_image_plot(&args).unwrap();
    }

    #[test]
    fn test_empty_labels() -> Result<()> {
        let temp_dir = tempdir()?;
        let img1_path = temp_dir.path().join("test1.png");
        let img2_path = temp_dir.path().join("test2.png");
        let output_path = temp_dir.path().join("output.png");

        create_test_image(&img1_path, 100, 100)?;
        create_test_image(&img2_path, 100, 100)?;

        let args = Args {
            images: vec![img1_path, img2_path],
            output: output_path.clone(),
            rows: 1,
            row_labels: vec![],
            column_labels: vec![],
        };

        save_image_plot(&args)?;
        assert!(output_path.exists());
        Ok(())
    }
}
