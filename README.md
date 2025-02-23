# xyplot

A Rust tool for plotting images in a grid layout with optional labels. This is a port of the original Python xyplot tool.

## Features

- Plot multiple images in a grid layout
- Add row and column labels
- Configurable number of rows
- White background with black text labels

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
