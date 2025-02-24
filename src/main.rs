#![warn(clippy::all, clippy::pedantic)]

use anyhow::Result;
use clap::Parser;
use imx::xyplot::{PlotConfig, LabelAlignment, DEFAULT_TOP_PADDING, DEFAULT_LEFT_PADDING};
use std::path::PathBuf;
use std::str::FromStr;

/// Wrapper type for LabelAlignment to implement FromStr
#[derive(Debug, Clone, Copy)]
struct AlignmentArg(LabelAlignment);

impl FromStr for AlignmentArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "start" => Ok(Self(LabelAlignment::Start)),
            "center" => Ok(Self(LabelAlignment::Center)),
            "end" => Ok(Self(LabelAlignment::End)),
            _ => Err(format!("Invalid alignment: {s}. Valid values are: start, center, end")),
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of image file paths
    #[arg(required = true, trailing_var_arg = true)]
    images: Vec<PathBuf>,

    /// Output file name for the generated plot
    #[arg(long, default_value = "output.jpg")]
    output: PathBuf,

    /// Number of rows to display the images
    #[arg(long, default_value_t = 1)]
    rows: u32,

    /// List of labels for each row. Provide multiple labels after a single --row-labels flag.
    /// Example: --row-labels "Row 1" "Row 2" "Row 3"
    #[arg(long, num_args = 1.., value_delimiter = ' ')]
    row_labels: Vec<String>,

    /// List of labels for each column. Provide multiple labels after a single --column-labels flag.
    /// Example: --column-labels "Col 1" "Col 2" "Col 3"
    #[arg(long, num_args = 1.., value_delimiter = ' ')]
    column_labels: Vec<String>,

    /// Alignment of column labels (start, center, end)
    #[arg(long, default_value = "center")]
    column_label_alignment: AlignmentArg,

    /// Alignment of row labels (start, center, end)
    #[arg(long, default_value = "center")]
    row_label_alignment: AlignmentArg,

    /// Enable debug mode to visualize layout
    #[arg(long)]
    debug: bool,

    /// Space reserved at the top of the plot for labels and padding
    #[arg(long, default_value_t = DEFAULT_TOP_PADDING)]
    top_padding: u32,

    /// Space reserved at the left of the plot for labels and padding
    #[arg(long, default_value_t = DEFAULT_LEFT_PADDING)]
    left_padding: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    let config = PlotConfig {
        images: args.images,
        output: args.output,
        rows: args.rows,
        row_labels: args.row_labels,
        column_labels: args.column_labels,
        column_label_alignment: args.column_label_alignment.0,
        row_label_alignment: args.row_label_alignment.0,
        debug_mode: args.debug,
        top_padding: args.top_padding,
        left_padding: args.left_padding,
    };

    imx::xyplot::create_plot(&config)
}
