use gstreamer as gst;
use gstreamer_pbutils as gst_pbutils;
use gst_pbutils::prelude::*;

use gst_pbutils::DiscovererInfo;
use gst_pbutils::DiscovererStreamInfo;

use anyhow::Error;
use derive_more::{Display, Error};

use std::env;

#[derive(Debug, Display, Error)]
#[display(fmt = "Discoverer error {}", _0)]
struct DiscovererError(#[error(not(source))] &'static str);

fn print_tags(info: &DiscovererInfo) {
    println!("Tags:");

    let tags = info.tags();
    match tags {
        Some(taglist) => {
            println!("  {}", taglist); // FIXME use an iterator
        }
        None => {
            println!("  no tags");
        }
    }
}

fn print_stream_info(stream: &DiscovererStreamInfo) {
    println!("Stream: ");
    if let Some(id) = stream.stream_id() {
        println!("  Stream id: {}", id);
    }
    let caps_str = match stream.caps() {
        Some(caps) => caps.to_string(),
        None => String::from("--"),
    };
    println!("  Format: {}", caps_str);
}

fn print_discoverer_info(info: &DiscovererInfo) -> Result<(), Error> {
    let uri = info
        .uri()
        .ok_or(DiscovererError("URI should not be null"))?;
    println!("URI: {}", uri);
    println!("Duration: {}", info.duration().display());
    print_tags(info);
    print_stream_info(
        &info
            .stream_info()
            .ok_or(DiscovererError("Error while obtaining stream info"))?,
    );

    let children = info.stream_list();
    println!("Children streams:");
    for child in children {
        print_stream_info(&child);
    }

    Ok(())
}

fn run_discoverer() -> Result<(), Error> {
    gst::init()?;

    let args: Vec<_> = env::args().collect();
    let uri: &str = if args.len() == 2 {
        args[1].as_ref()
    } else {
        println!("Usage: discoverer uri");
        std::process::exit(-1)
    };

    let timeout: gst::ClockTime = gst::ClockTime::from_seconds(15);
    let discoverer = gst_pbutils::Discoverer::new(timeout)?;
    let info = discoverer.discover_uri(uri)?;
    print_discoverer_info(&info)?;
    Ok(())
}

fn example_main() {
    match run_discoverer() {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn main() {
    example_main();
}