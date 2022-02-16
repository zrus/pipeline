// This example demonstrates how to use GStreamer's iteration APIs.
// This is used at multiple occassions - for example to iterate an
// element's pads.

use gst::element_error;
use gst::glib;
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;

use anyhow::Error;
use byte_slice_cast::*;
use derive_more::{Display, Error};

use std::env;

const URI: &'static str = "rtsp://10.50.13.240/1/h264major";

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

fn create_pipeline() -> Result<gst::Pipeline, Error> {
    gst::init()?;

    let pipeline = gst::Pipeline::new(None);
    let src =
        gst::ElementFactory::make("rtspsrc", None).map_err(|_| MissingElement("audiotestsrc"))?;
    src.set_property("location", URI);
    let sink = gst::ElementFactory::make("appsink", None).map_err(|_| MissingElement("appsink"))?;

    let elements = &[&src, &sink];
    pipeline.add_many(elements)?;
    gst::Element::link_many(elements).map_err(|e| {
        println!("{:?}", e);
    });

    let appsink = sink
        .dynamic_cast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(|appsink| {
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                let buffer = sample.buffer().ok_or_else(|| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                // At this point, buffer is only a reference to an existing memory region somewhere.
                // When we want to access its content, we have to map it while requesting the required
                // mode of access (read, read/write).
                // This type of abstraction is necessary, because the buffer in question might not be
                // on the machine's main memory itself, but rather in the GPU's memory.
                // So mapping the buffer makes the underlying memory region accessible to us.
                // See: https://gstreamer.freedesktop.org/documentation/plugin-development/advanced/allocation.html
                let map = buffer.map_readable().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;

                // We know what format the data in the memory region has, since we requested
                // it by setting the appsink's caps. So what we do here is interpret the
                // memory region we mapped as an array of signed 16 bit integers.
                let samples = map.as_slice_of::<i16>().map_err(|_| {
                    element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;

                // For buffer (= chunk of samples), we calculate the root mean square:
                // (https://en.wikipedia.org/wiki/Root_mean_square)
                let sum: f64 = samples
                    .iter()
                    .map(|sample| {
                        let f = f64::from(*sample) / f64::from(i16::MAX);
                        f * f
                    })
                    .sum();
                let rms = (sum / (samples.len() as f64)).sqrt();
                println!("rms: {}", rms);

                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline) -> Result<(), Error> {
    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .src()
                        .map(|s| String::from(s.path_string()))
                        .unwrap_or_else(|| String::from("None")),
                    error: err.error().to_string(),
                    debug: err.debug(),
                    source: err.error(),
                }
                .into());
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

fn main() {
    match create_pipeline().and_then(main_loop) {
        Ok(r) => r,
        Err(e) => eprintln!("Error! {}", e),
    }
}
