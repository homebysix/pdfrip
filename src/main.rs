mod cli;

use std::process::ExitCode;

use anyhow::Context;
use cli::interface::Method;
use engine::{
    crackers::PDFCracker,
    producers::{
        custom_query::CustomQuery, dates::DateProducer, default_query::DefaultQuery,
        dictionary::LineProducer, number_ranges::RangeProducer, Producer,
    },
};
use log::info;
use pretty_env_logger::env_logger::Env;

use crate::cli::interface;

fn init_logger() {
    let env = Env::default().filter_or("LOG_LEVEL", "info");
    pretty_env_logger::formatted_timed_builder()
        .parse_env(env)
        .init();
}

fn select_producer(subcommand: Method) -> Box<dyn Producer> {
    match subcommand {
        interface::Method::Wordlist(args) => {
            let producer = LineProducer::from(&args.wordlist);
            Box::from(producer)
        }
        interface::Method::Range(args) => {
            let padding: usize = if args.add_preceding_zeros {
                args.upper_bound.checked_ilog10().unwrap() as usize + 1
            } else {
                0
            };
            let producer = RangeProducer::new(padding, args.lower_bound, args.upper_bound);
            Box::from(producer)
        }
        interface::Method::CustomQuery(args) => {
            let producer = CustomQuery::new(&args.custom_query, args.add_preceding_zeros);
            Box::from(producer)
        }
        interface::Method::Date(args) => {
            let producer = DateProducer::new(args.start, args.end);
            Box::from(producer)
        }
        interface::Method::DefaultQuery(args) => {
            let producer = DefaultQuery::new(args.max_length, args.min_length);
            Box::from(producer)
        }
    }
}

pub fn main() -> anyhow::Result<ExitCode> {
    init_logger();

    // print a banner to look cool!
    interface::banner();

    let cli_args = interface::args();

    let producer: Box<dyn Producer> = select_producer(cli_args.subcommand);

    let filename = cli_args.filename;

    let res = engine::crack_file(
        cli_args.number_of_threads,
        PDFCracker::from_file(&filename).context(format!("path: {}", filename))?,
        producer,
    )?;

    match res {
        Some(password) => match std::str::from_utf8(&password) {
            Ok(password) => {
                info!("Success! Found password: {}", password)
            }
            Err(_) => {
                let hex_string: String = password
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" ");
                info!(
                            "Success! Found password, but it contains invalid UTF-8 characters. Displaying as hex: {}",
                            hex_string
                        )
            }
        },
        None => {
            info!("Failed to crack file...");
            return Ok(ExitCode::FAILURE);
        }
    }

    Ok(ExitCode::SUCCESS)
}
