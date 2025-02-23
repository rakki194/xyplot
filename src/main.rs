#![warn(clippy::all, clippy::pedantic)]

use anyhow::Result;
use clap::Parser;
use imx::xyplot::PlotConfig;
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
    
    let config = PlotConfig {
        images: args.images,
        output: args.output,
        rows: args.rows,
        row_labels: args.row_labels,
        column_labels: args.column_labels,
    };

    imx::create_plot(&config)
}
