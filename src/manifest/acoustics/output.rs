use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn start_output_stream(mut sample_callback: impl FnMut(&mut [f32]) + Send + 'static) {
    let host = cpal::default_host();
    let device = host.default_output_device().expect("no output device");
    let config = device.default_output_config().unwrap();

    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _| sample_callback(data),
        move |err| eprintln!("Stream error: {:?}", err),
        None, // Adding the missing argument, e.g., latency or options
    ).unwrap();

    stream.play().unwrap();
}
