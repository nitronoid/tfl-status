#[macro_use]
extern crate prettytable;
extern crate reqwest;
extern crate structopt;
extern crate textwrap;

// Make some types and modules less verbose
use prettytable::{color, format, Attr, Row, Table};
use std::string::String;
use structopt::StructOpt;

// Describes our table format style (the characters to decorate with)
fn box_border_format() -> format::TableFormat {
    format::FormatBuilder::new()
        .borders('│')
        .separator(
            format::LinePosition::Top,
            format::LineSeparator::new('─', '─', '┌', '┐'),
        )
        .separator(
            format::LinePosition::Bottom,
            format::LineSeparator::new('─', '─', '└', '┘'),
        )
        .padding(1, 1)
        .build()
}

// Pulls the raw json string from the TFL API via an http request
fn pull_status_data() -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://api.tfl.gov.uk/line/mode/tube,overground,dlr,tflrail/status";
    let body = reqwest::get(url).unwrap().text()?;
    Ok(body)
}

// Determine the status color from a TFL status code
fn status_color(status_code: i64) -> color::Color {
    match status_code {
        // Good service
        10 => color::GREEN,
        // Severe delays
        20 => color::RED,
        // Minor disruption and misc warnings
        8...std::i64::MAX => color::YELLOW,
        // Catch all
        _ => color::RED,
    }
}

// Formats a plain json string into a pretty table with only the relevant
// information stripped out
fn format_status(raw_status_data: &str, opt: &FormatOptions) -> serde_json::Result<Table> {
    // Convert the raw string into a deserialized json object
    let json_data: serde_json::Value = serde_json::from_str(raw_status_data)?;
    // Create a new pretty table and use our custom style format
    let mut table = Table::new();
    table.set_format(box_border_format());
    // Iterate over our json data as an array, where each object corresponds to
    // a tube line. Extract the name, service status, and reason for any delay.
    // Simultaneously keep track of the longest combination of name and status
    // to wrap long reason strings.
    let mut max_len = 0;
    json_data
        .as_array()
        .unwrap()
        .iter()
        .map(|line| {
            // Borrow here to avoid multiple map lookups
            let stat = &line["lineStatuses"][0];
            // Extract the name and status strings
            let name = line["name"].as_str().unwrap();
            let status = stat["statusSeverityDescription"].as_str().unwrap();
            // Select a terminal color based on the status code
            let color = status_color(stat["statusSeverity"].as_i64().unwrap());
            max_len = std::cmp::max(max_len, name.len() + status.len());
            // Generate a row for our table
            Row::new(vec![
                cell!(name),
                cell!(status).with_style(Attr::ForegroundColor(color)),
                cell!(match (opt.no_reason, stat["reason"].as_str()) {
                    (false, Some(reason)) => reason,
                    _ => "",
                }),
            ])
        })
        .for_each(|row| {
            table.add_row(row);
        });

    // Calculate the max length for a reason string and wrap all reason strings
    if !opt.no_reason {
        let wrap_width = match opt.reason_width {
            0 => textwrap::termwidth() - (max_len + 8),
            w => w,
        };
        table.row_iter_mut().for_each(|row| {
            let cell = row.iter_mut().last().unwrap();
            *cell = cell!(textwrap::fill(&cell.get_content(), wrap_width));
        });
    }
    Ok(table)
}

// Derive the program options
#[derive(StructOpt, Debug)]
#[structopt(
    name = "TFL-status",
    about = "View the current tube line status from your shell.",
    author = "Jack Diver <jackdiver@hotmail.co.uk>",
    version = "1.0"
)]
struct FormatOptions {
    #[structopt(
        short = "n",
        long = "no-reason",
        help = "Switch off status reason reporting."
    )]
    no_reason: bool,

    #[structopt(
        short = "w",
        long = "reason-width",
        default_value = "0",
        help = "Override the reason column width. \
                A value of zero will use the automatically calculated width."
    )]
    reason_width: usize,
}

fn main() {
    let raw_data = pull_status_data().unwrap();
    format_status(&raw_data, &FormatOptions::from_args()).unwrap().printstd();
}
