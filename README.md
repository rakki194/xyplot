# xyplot

A Rust command-line tool for plotting images in a grid layout with optional labels. Built on top of the [imx](https://github.com/rakki194/imx) library.

## Features

- Plot multiple images in a grid layout
- Add row and column labels
- Configurable number of rows
- White background with black text labels
- Unicode and emoji support in labels
- Automatic grid layout calculation

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

# Specify output file
xyplot image1.jpg image2.jpg --output result.jpg
```

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
