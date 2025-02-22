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
    let position = Point {
        x: x as f32,
        y: y as f32,
    };

    // Draw each character
    let mut x_offset = 0.0;
    for c in text.chars() {
        let glyph_id = font.glyph_id(c);
        let glyph =
            glyph_id.with_scale_and_position(px_scale, point(position.x + x_offset, position.y));

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
    let top_padding = 50;
    let left_padding = if row_labels.iter().any(|l| !l.is_empty()) {
        40
    } else {
        0
    };

    let has_other_labels = !labels.is_empty() || !row_labels.is_empty();
    let canvas_height = if !has_other_labels && !column_labels.is_empty() {
        image_height * rows + top_padding
    } else {
        (image_height + top_padding) * rows
    };

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
        let x = i32::try_from(u32::try_from(i)? * image_width + left_padding)
            .map_err(|_| anyhow::anyhow!("Position overflow"))?;
        draw_text(&mut canvas, label, x, 15, scale, &font, color);
    }

    // Place images and labels
    for (i, img_path) in images.iter().enumerate() {
        let i = u32::try_from(i)?;
        let row = i / cols;
        let col = i % cols;

        // Add row label
        if row < u32::try_from(row_labels.len())? {
            let y = row * (image_height + top_padding) + 30;
            draw_text(
                &mut canvas,
                &row_labels[row as usize],
                5,
                y as i32,
                scale,
                &font,
                color,
            );
        }

        // Load and place image
        let img = image::open(img_path)
            .with_context(|| format!("Failed to open image: {img_path:?}"))?
            .to_rgb8();

        let y_start = if !has_other_labels && !column_labels.is_empty() {
            row * image_height + top_padding
        } else {
            row * (image_height + top_padding) + top_padding
        };

        let x_start = col * image_width + left_padding;

        // Copy image to canvas
        for (x, y, pixel) in img.enumerate_pixels() {
            if x_start + x < canvas_width && y_start + y < canvas_height {
                canvas.put_pixel(x_start + x, y_start + y, *pixel);
            }
        }

        // Add image label if provided
        if i < u32::try_from(labels.len())? {
            let x = i32::try_from(col * image_width + left_padding)
                .map_err(|_| anyhow::anyhow!("Position overflow"))?;
            let y = i32::try_from(y_start + image_height / 2)
                .map_err(|_| anyhow::anyhow!("Position overflow"))?;
            draw_text(&mut canvas, &labels[i as usize], x, y, scale, &font, color);
        }
    }

    // Save the generated plot
    canvas
        .save(&args.output)
        .with_context(|| format!("Failed to save output image: {:?}", args.output))?;

    println!("Generated plot saved as {:?}", args.output);
    Ok(())
}
