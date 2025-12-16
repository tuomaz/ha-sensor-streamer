use crate::state::AppState;
use anyhow::{Context, Result};
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_rtsp_server as gst_rtsp_server;
use gstreamer_rtsp_server::prelude::*;
use gstreamer_video as gst_video;
use std::sync::Arc;

pub fn run_rtsp_server(config: &crate::config::Config, app_state: AppState) -> Result<()> {
    gst::init()?;

    let server = gst_rtsp_server::RTSPServer::new();
    server.set_service(&config.port.to_string());

    let mounts = server
        .mount_points()
        .context("Could not get mount points")?;
    let factory = gst_rtsp_server::RTSPMediaFactory::new();

    // Define the pipeline
    // appsrc -> videoconvert -> x264enc -> rtph264pay
    // We use speed-preset=ultrafast and tune=zerolatency for real-time performance
    let pipeline_str = "appsrc name=src format=time is-live=true do-timestamp=true \
        ! videoconvert \
        ! x264enc speed-preset=ultrafast tune=zerolatency \
        ! rtph264pay name=pay0 pt=96"
        .to_string();

    factory.set_launch(&pipeline_str);
    factory.set_shared(true); // Share the pipeline among clients?
                              // Actually, for appsrc, sharing is tricky if we don't manage the push loop centrally.
                              // If shared=false (default), every client gets its own appsrc and its own generation loop.
                              // This is safer for simple implementation, though more CPU intensive if many clients connect.
                              // Let's stick to non-shared (default) for simplicity.

    // Clone state for the closure
    let state = Arc::new(app_state);

    factory.connect_media_configure(move |_factory, media| {
        let element = media.element();
        let appsrc_element = element
            .downcast_ref::<gst::Bin>()
            .unwrap()
            .by_name("src")
            .expect("Could not find appsrc 'src'");

        let appsrc = appsrc_element
            .downcast::<gst_app::AppSrc>()
            .expect("Source element is not an appsrc");

        // Setup the video info
        let width = state.config.video_width as i32;
        let height = state.config.video_height as i32;
        let fps = state.config.video_fps as i32;

        let video_info =
            gst_video::VideoInfo::builder(gst_video::VideoFormat::Rgb, width as u32, height as u32)
                .fps(gst::Fraction::new(fps, 1))
                .build()
                .expect("Failed to create video info");

        appsrc.set_caps(Some(&video_info.to_caps().unwrap()));
        appsrc.set_format(gst::Format::Time);

        // We need to keep a mutable state for the timestamp/frame count inside the callback
        // The callback is called from GStreamer threads.
        let state_clone = state.clone();
        let mut timestamp = 0u64;
        let frame_duration = 1_000_000_000 / (fps as u64); // duration in ns

        let callbacks = gst_app::AppSrcCallbacks::builder()
            .need_data(move |appsrc, _hint| {
                // Check current sensor value
                let val_map = {
                    let lock = state_clone.sensor_values.read().unwrap();
                    lock.clone()
                };

                // Generate frame
                // Note: ImageGenerator now returns raw RGB bytes for RTSP efficiency.
                let raw_bytes = state_clone.image_gen.generate_raw_frame(&val_map);

                // Create buffer
                let mut buffer = gst::Buffer::from_slice(raw_bytes);

                // Set timestamps
                let pts = timestamp;
                {
                    let buffer_ref = buffer.get_mut().unwrap();
                    buffer_ref.set_pts(gst::ClockTime::from_nseconds(pts));
                    buffer_ref.set_duration(gst::ClockTime::from_nseconds(frame_duration));
                }

                // Push buffer
                let _ = appsrc.push_buffer(buffer);
                timestamp += frame_duration;
            })
            .build();

        appsrc.set_callbacks(callbacks);
    });

    mounts.add_factory("/stream", factory);

    println!(
        "RTSP Server listening on rtsp://0.0.0.0:{}/stream",
        config.port
    );

    // Attach the server to the default main context
    server.attach(None)?;

    // Run the main loop
    let main_loop = gst::glib::MainLoop::new(None, false);
    main_loop.run();

    Ok(())
}
