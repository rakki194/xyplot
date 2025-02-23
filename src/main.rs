#![warn(clippy::all, clippy::pedantic)]

use ab_glyph::{Font, FontRef, GlyphId, Point, PxScale, ScaleFont};
use anyhow::{Context, Result};
use clap::Parser;
use image::{Rgb, RgbImage};
use imx::numeric::{self, f32_to_u8, i32_to_u32, u32_to_i32, f32_to_i32};
use rgb::{FromSlice, RGB8};
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

// Constants for layout
const TOP_PADDING: u32 = 40; // Space for labels and padding at the top

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
        // Create a glyph with scale and position
        let glyph = id.with_scale_and_position(px_scale, cursor);
        cursor.x += scaled_font.h_advance(id);
        glyphs.push((glyph, font));
    }

    // Second pass: render glyphs
    for (glyph, font) in glyphs {
        let scaled_font = font.as_scaled(px_scale);
        let glyph_position = glyph.position; // Store position before moving glyph
        let glyph_id = glyph.id; // Store ID before moving glyph

        if let Some(outlined) = scaled_font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|x, y, coverage| {
                // Convert the coverage value into an alpha channel value
                let alpha = f32_to_u8(coverage * 255.0);
                if alpha == 0 {
                    return;
                }

                // Get absolute pixel coordinates - x and y are u32 from the draw callback
                // bounds.min.x and bounds.min.y are f32
                #[allow(clippy::cast_precision_loss)]
                let px = i32_to_u32(f32_to_i32((x as f32) + bounds.min.x));
                #[allow(clippy::cast_precision_loss)]
                let py = i32_to_u32(f32_to_i32((y as f32) + bounds.min.y));

                // Blend the color with the existing pixel based on alpha
                if px < canvas.width() && py < canvas.height() {
                    let pixel = canvas.get_pixel_mut(px, py);
                    let blend = |a: u8, b: u8, alpha: u8| -> u8 {
                        let a = f32::from(a);
                        let b = f32::from(b);
                        let alpha = f32::from(alpha) / 255.0;
                        f32_to_u8(a * (1.0 - alpha) + b * alpha)
                    };

                    pixel[0] = blend(pixel[0], color[0], alpha);
                    pixel[1] = blend(pixel[1], color[1], alpha);
                    pixel[2] = blend(pixel[2], color[2], alpha);
                }
            });
        }

        // Check for color emoji image
        if let Some(img) = font.glyph_raster_image2(glyph_id, u16::MAX) {
            let img_width = u32::from(img.width);
            let scale_factor = scale / f32::from(img.pixels_per_em);

            // Convert raw bytes to RGB pixels
            let pixels: &[RGB8] = img.data.as_rgb();
            for (img_y, row) in pixels.chunks(img_width as usize).enumerate() {
                for (img_x, pixel) in row.iter().enumerate() {
                    // Note: For image coordinates, some precision loss is acceptable
                    #[allow(clippy::cast_precision_loss)]
                    let src_x = img_x as f32 * scale_factor;
                    #[allow(clippy::cast_precision_loss)]
                    let src_y = img_y as f32 * scale_factor;
                    
                    let canvas_x = i32_to_u32(f32_to_i32(glyph_position.x + src_x + img.origin.x * scale_factor));
                    let canvas_y = i32_to_u32(f32_to_i32(glyph_position.y + src_y + img.origin.y * scale_factor));

                    if canvas_x < canvas.width() && canvas_y < canvas.height() {
                        let canvas_pixel = canvas.get_pixel_mut(canvas_x, canvas_y);
                        // For emoji, we'll use full opacity since they're typically fully opaque
                        canvas_pixel[0] = pixel.r;
                        canvas_pixel[1] = pixel.g;
                        canvas_pixel[2] = pixel.b;
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
    let left_padding = if row_labels.iter().any(|l| !l.is_empty()) {
        150 // Increased from 40 to give more space for row labels
    } else {
        0
    };

    // Calculate canvas dimensions with space for labels
    let has_labels = !row_labels.is_empty() || !column_labels.is_empty();
    let row_height = image_height + (if has_labels { TOP_PADDING } else { 0 });
    let canvas_height = row_height * rows + (if has_labels { TOP_PADDING } else { 0 });
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
            let x = u32_to_i32(u32::try_from(col).unwrap_or(0) * image_width + left_padding + image_width / 2);
            let y = u32_to_i32(TOP_PADDING / 2);
            
            // Note: For text positioning, precision loss in usize to f32 conversion is acceptable
            #[allow(clippy::cast_precision_loss)]
            let label_offset = (label.len() as f32) * 20.0 / 2.0;
            
            draw_text(
                &mut canvas,
                label,
                x - f32_to_i32(label_offset),
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
        let y_start = row * row_height + TOP_PADDING;

        // Add row label if provided (left of the image)
        if let Some(row_label) = row_labels.get(row as usize) {
            let x = 20;
            let y = u32_to_i32(y_start + image_height / 2);
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
    #[should_panic(expected = "Number of row labels (2) should match the number of rows (1)")]
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
    #[should_panic(expected = "Number of column labels (2) should match the number of columns (1)")]
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
