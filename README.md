# xyplot

A Rust command-line tool for plotting images in a grid layout with optional labels. Built on top of the [imx](https://github.com/rakki194/imx) library.

## Features

- Plot multiple images in a grid layout
- Add row and column labels
- Support for multiline text in labels
- Configurable number of rows
- Configurable label alignments (start, center, end)
- Independent row and column label alignment
- Adjustable padding for both row and column labels
- White background with black text labels
- Unicode and emoji support in labels
- Automatic grid layout calculation
- Layout debugging visualization

## Installation

```bash
cargo install xyplot
```

## Usage

```bash
# Basic usage with just images
xyplot image1.jpg image2.jpg image3.jpg

# Specify number of rows
xyplot image1.jpg image2.jpg image3.jpg image4.jpg --rows 2

# With row labels (supports multiline text with \n)
xyplot image1.jpg image2.jpg image3.jpg image4.jpg \
    --rows 2 \
    --row-labels "Row 1\nDetails" "Row 2\nMore Info"

# With column labels (supports multiline text with \n)
xyplot image1.jpg image2.jpg image3.jpg image4.jpg \
    --rows 2 \
    --column-labels "Col 1\nFirst" "Col 2\nSecond"

# Configure column label alignment
xyplot image1.jpg image2.jpg \
    --column-labels "Col 1" "Col 2" \
    --column-label-alignment start

# Configure row label alignment
xyplot image1.jpg image2.jpg \
    --row-labels "Row 1" "Row 2" \
    --row-label-alignment end

# Adjust padding for labels
xyplot image1.jpg image2.jpg \
    --column-labels "Header\nSubheader" "Col 2\nDetails" \
    --top-padding 80 \
    --left-padding 100

# Specify output file
xyplot image1.jpg image2.jpg --output result.jpg

# Enable layout debugging
xyplot image1.jpg image2.jpg --debug
```

### Label Alignments

Both row and column labels can be aligned independently using the `--column-label-alignment` and `--row-label-alignment` options:

- `start`: Align labels at the start (left for columns, top for rows)
- `center`: Center labels (default)
- `end`: Align labels at the end (right for columns, bottom for rows)

### Multiline Text

You can use `\n` in your labels to create multiple lines:

```bash
# Two-line column labels
xyplot image1.jpg image2.jpg \
    --column-labels "Title\nSubtitle" "Header\nDetails"

# Three-line row labels
xyplot image1.jpg image2.jpg \
    --rows 2 \
    --row-labels "Section 1\nDetails\nMore Info" "Section 2\nNotes\nExtra"
```

The padding will automatically adjust to accommodate the multiline text, but you can also specify custom padding:

```bash
# Custom padding for multiline labels
xyplot image1.jpg image2.jpg \
    --column-labels "Title\nSubtitle" \
    --row-labels "Section\nDetails" \
    --top-padding 80 \
    --left-padding 100
```

### Layout Debugging

The `--debug` flag enables a powerful layout visualization feature that helps understand and debug how images and labels are positioned in the grid.

When you use the `--debug` flag, xyplot will generate two files:

1. The normal output image (e.g., `output.jpg`)
2. A debug visualization (e.g., `output_debug.jpg`)

The debug visualization uses color coding to show different elements:

- **Light Blue**: Image areas
- **Light Red**: Row label areas
- **Light Green**: Column label areas
- **Light Gray**: Padding areas
- **Dark Gray**: Borders around each element

### Example Debug Usage

```bash
# Debug visualization with multiline labels
xyplot image1.jpg image2.jpg image3.jpg image4.jpg \
    --rows 2 \
    --row-labels "Row 1\nDetails" "Row 2\nMore Info" \
    --column-labels "Col 1\nFirst" "Col 2\nSecond" \
    --debug
```

### Understanding the Debug Output

The debug visualization helps you:

1. **Verify Grid Layout**: See how images are arranged in the grid
2. **Check Alignment**: Ensure labels are properly aligned
3. **Inspect Spacing**: View padding and margins between elements
4. **Debug Label Placement**: Verify label positions relative to images
5. **Understand Dimensions**: See the exact size and position of each element
6. **Verify Multiline Text**: Check spacing for multiline labels

This is particularly useful when:

- Developing new layout features
- Fixing alignment issues
- Understanding why elements are positioned in certain ways
- Verifying that labels and images don't overlap
- Debugging multiline text layout

## Using as a Library

If you want to use the plotting functionality in your own Rust project, consider using the [imx](https://github.com/rakki194/imx) library directly:

```rust
use imx::{PlotConfig, create_plot, LabelAlignment};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let config = PlotConfig {
        images: vec![PathBuf::from("image1.jpg"), PathBuf::from("image2.jpg")],
        output: PathBuf::from("output.jpg"),
        rows: 1,
        row_labels: vec!["Row 1\nDetails".to_string()],
        column_labels: vec!["Col 1\nFirst".to_string(), "Col 2\nSecond".to_string()],
        column_label_alignment: LabelAlignment::Center,
        row_label_alignment: LabelAlignment::Start,
        top_padding: 60,
        left_padding: 80,
        debug_mode: false,
    };

    create_plot(&config)?;
    Ok(())
}
```

## Dependencies

- Rust 1.56 or later
- Required system libraries for image processing

## Building from Source

1. Clone the repository
2. Install dependencies:

   ```bash
   cargo build
   ```

3. Run the tests:

   ```bash
   cargo test
   ```

## License

MIT License
