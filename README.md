# xyplot

A Rust command-line tool for plotting images in a grid layout with optional labels. Built on top of the [imx](https://github.com/rakki194/imx) library.

## Features

- Plot multiple images in a grid layout
- Add row and column labels
- Configurable number of rows
- Configurable column label alignment (left, center, right)
- Adjustable top padding for label spacing
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

# With row labels
xyplot image1.jpg image2.jpg image3.jpg image4.jpg --rows 2 --row-labels "Row 1" "Row 2"

# With column labels
xyplot image1.jpg image2.jpg image3.jpg image4.jpg --rows 2 --column-labels "Col 1" "Col 2"

# Configure column label alignment
xyplot image1.jpg image2.jpg --column-labels "Col 1" "Col 2" --column-label-alignment left

# Adjust top padding for labels
xyplot image1.jpg image2.jpg --column-labels "Col 1" "Col 2" --top-padding 60

# Specify output file
xyplot image1.jpg image2.jpg --output result.jpg

# Enable layout debugging
xyplot image1.jpg image2.jpg --debug
```

### Column Label Alignment

The `--column-label-alignment` option controls how column labels are positioned relative to their images:

- `left`: Align labels with the left edge of the image
- `center`: Center labels over the image (default)
- `right`: Align labels with the right edge of the image

### Top Padding

The `--top-padding` option controls the vertical space reserved for labels:

- Default value is 40 pixels
- Can be increased for larger labels or decreased for compact layouts
- Only affects the layout when labels are present

## Layout Debugging

The `--debug` flag enables a powerful layout visualization feature that helps understand and debug how images and labels are positioned in the grid.

### Debug Output

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
# Basic debug visualization
xyplot image1.jpg image2.jpg --debug

# Debug complex layout with labels
xyplot image1.jpg image2.jpg image3.jpg image4.jpg \
    --rows 2 \
    --row-labels "Row 1" "Row 2" \
    --column-labels "Col 1" "Col 2" \
    --debug
```

### Understanding the Debug Output

The debug visualization helps you:

1. **Verify Grid Layout**: See how images are arranged in the grid
2. **Check Alignment**: Ensure images and labels are properly aligned
3. **Inspect Spacing**: View padding and margins between elements
4. **Debug Label Placement**: Verify label positions relative to images
5. **Understand Dimensions**: See the exact size and position of each element

This is particularly useful when:

- Developing new layout features
- Fixing alignment issues
- Understanding why elements are positioned in certain ways
- Verifying that labels and images don't overlap

## Using as a Library

If you want to use the plotting functionality in your own Rust project, consider using the [imx](https://github.com/rakki194/imx) library directly:

```rust
use imx::{PlotConfig, create_plot};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let config = PlotConfig {
        images: vec![PathBuf::from("image1.jpg"), PathBuf::from("image2.jpg")],
        output: PathBuf::from("output.jpg"),
        rows: 1,
        row_labels: vec!["Row 1".to_string()],
        column_labels: vec!["Col 1".to_string(), "Col 2".to_string()],
        debug_mode: false, // Disables debug visualization
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
