use cpal::platform::Host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use opus::{Encoder, Decoder, Application};
use opus::Channels;
use crate::log;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: Channels = Channels::Mono;

pub fn initialize_audio_interface() -> (Option<cpal::Device>, Option<cpal::Device>) {
    // Open communication with the default audio interface

    // Get the default host
    let host: Host = cpal::default_host();

    // Get the default input device
    let input_device: Option<cpal::Device> = host.default_input_device();
    match &input_device {
        Some(device) => {
            match device.name() {
                Ok(name) => log::log_message(&format!("Default input device: {}", name)),
                Err(err) => log::log_message(&format!("Failed to get input device name: {}", err)),
            }
        },
        None => println!("Default input device found"),
    }

    // Get the default output device
    let output_device: Option<cpal::Device> = host.default_output_device();
    match &output_device {
        Some(device) => {
            match device.name() {
                Ok(name) => log::log_message(&format!("Default output device: {}", name)),
                Err(err) => log::log_message(&format!("Failed to get output device name: {}", err)),
            }
        },
        None => log::log_message(&format!("Default input device found")),
    }

    (input_device, output_device)
}

pub fn get_audio_config(device: &cpal::Device) -> cpal::StreamConfig {
    let config = device.default_output_config().unwrap();

    let config =  cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(256),
    };

    config
}

pub fn start_input_stream(input_device: &cpal::Device, config: &cpal::StreamConfig) -> cpal::Stream {
    // Start the audio input/output stream

    let audio_buffer = Arc::new(Mutex::new(Vec::new()));

    let buffer_clone = Arc::clone(&audio_buffer);
    let timeout: Duration = Duration::from_secs(5);

    let stream = input_device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = buffer_clone.lock().unwrap();
                buffer.extend_from_slice(data);
                },
            |err| eprintln!("An error occured on the input audio stream: {}", err),
                Some(timeout)
                ).expect("Failed to build input stream");

    stream.play().expect("Failed to start input stream");
    stream
}

pub fn start_output_stream(output_device: &cpal::Device, config: &cpal::StreamConfig,
    received_data: Arc<Mutex<Vec<u8>>>) -> cpal::Stream {
    // Start the audio input/output stream

    let output_buffer_clone = Arc::clone(&received_data);
    let stream = output_device.build_output_stream(
        &config,
        move |output_data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            let buffer = output_buffer_clone.lock().unwrap();
            if !buffer.is_empty() {
                    match decode_opus_to_pcm(&buffer) {
                        Ok(pcm_data) => {
                            for (i, sample) in output_data.iter_mut().enumerate() {
                                if i < pcm_data.len() {
                                    *sample = pcm_data[i];
                                } else {
                                    *sample = 0.0;
                                }
                            }
                        }
                        Err(err) => eprintln!("Opus decoding error: {}", err),
                    }
                }
            },
        |err| eprintln!("An error occured on the output audio stream: {}", err),
        None
    ).unwrap();

    stream.play().expect("Failed to start output stream");
    stream
}

#[allow(unused_variables)]
pub fn stop_audio_stream(stream: cpal::Stream) {
    // Stop the audio stream
}

#[allow(unused_variables)]
pub fn convert_audio_stream_to_opus(input_stream: &[f32]) -> Result<Vec<u8>, opus::Error> {
    // Convert audio stream to a desire formar (opus as default)

    let mut opus_encoder = Encoder::new(SAMPLE_RATE, CHANNELS, Application::Audio)?;
    let mut encoded_data = vec![0; 4000]; // Output buffer for Opus encoded data

    let len = opus_encoder.encode_float(input_stream, &mut encoded_data)?;
    Ok(encoded_data[..len].to_vec()) // Return the encoded data as a vector

}

pub fn decode_opus_to_pcm(opus_data: &[u8]) -> Result<Vec<f32>, opus::Error> {
    let mut decoder = Decoder::new(SAMPLE_RATE, Channels::Stereo)?;
    let mut pcm_data = vec![0.0; opus_data.len() * 1];
    let decoded_samples = decoder.decode_float(opus_data, &mut pcm_data, false)?;
    pcm_data.truncate(decoded_samples * 1);
    Ok(pcm_data)
}
