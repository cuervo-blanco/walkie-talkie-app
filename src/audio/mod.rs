// ============================================
//                  Imports
// ============================================
use cpal::platform::Host;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use opus::{Encoder, Decoder, Application};
use opus::Channels;
use crate::log;

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: Channels = Channels::Mono;

pub type FormattedAudio = Result<Vec<u8>, opus::Error>;

// ============================================
//       Initialize Audio Interface
// Open communication with the default audio
// interface.
// ============================================

pub fn initialize_audio_interface() -> (Option<cpal::Device>, Option<cpal::Device>) {

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
        None => log::log_message(&format!("Default input device found")),
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
// ============================================
//            Get Audio Config
// ============================================
pub fn get_audio_config(device: &cpal::Device) -> Result<cpal::StreamConfig, cpal::DefaultStreamConfigError> {
    let config = match device.default_output_config() {
        Ok(cnfg) => cnfg,
        Err(e) => {
            log::log_message(&format!("Unable to get default config: {}", e));
            return Err(e);
        }
    };

    let config =  cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Fixed(256),
    };

    Ok(config)
}
// ============================================
//        Start Input Stream
// ============================================
pub fn start_input_stream(input_device: &cpal::Device, config: &cpal::StreamConfig) -> Result<cpal::Stream, cpal::BuildStreamError> {
    // Start the audio input/output stream

    let audio_buffer = Arc::new(Mutex::new(Vec::new()));

    let buffer_clone = Arc::clone(&audio_buffer);
    let timeout: Duration = Duration::from_secs(5);

    let stream = input_device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut buffer = match buffer_clone.lock() {
                Ok(guard) => guard,
                Err(poisoned_err) => {
                    let data = poisoned_err.get_ref();
                    log::log_message(&format!("Unable to guard data: {:?}", data));
                    return;
                }
            };
                buffer.extend_from_slice(data);
                },
            |err| log::log_message(&format!("An error occured on the input audio stream: {}", err)),
                Some(timeout)
                );
    match stream {
        Ok(s) => {
            if let Err(err) = s.play() {
                log::log_message(&format!("Failed to start input stream: {}", err));
            }
            Ok(s)
        }
        Err(e) => {
            log::log_message(&format!("Failed to build input stream: {}", e));
            Err(e)
        }
    }

}
// ============================================
//        Start Output Stream
// ============================================
pub fn start_output_stream(output_device: &cpal::Device, config: &cpal::StreamConfig,
    received_data: Arc<Mutex<Vec<u8>>>) -> Result<cpal::Stream, cpal::BuildStreamError> {
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
                        Err(err) =>
                            log::log_message(&format!("Opus decoding error: {}", err)),
                    }
                }
            },
        |err| log::log_message(&format!("An error occured on the output audio stream: {}", err)),
        None
    );

    match stream {
        Ok(s) => {
            if let Err(err) = s.play() {
                log::log_message(&format!("Failed to start output stream: {}", err));
            }
            Ok(s)
        }
        Err(e) => {
            log::log_message(&format!("Failed to build output stream: {}", e));
            Err(e)
        }
    }
}
// ============================================
//        Stop Audio Stream
// Stop the audio stream.
// ============================================
pub fn stop_audio_stream(stream: cpal::Stream) {
    match stream.pause() {
        Ok(_) => {
            // Dropping the stream to release resources
            // Stream will be dropped automatically when it goes out of scope
        }
        Err(e) => {
            log::log_message(&format!("Unable to pause audio stream: {}", e));
        }
    }
    // Explicitly dropping the stream after attempting to pause it
    drop(stream);
}
// ============================================
//    Convert PCM to Opus Format
// ============================================
// Convert audio stream from PCM format to Opus format
pub fn convert_audio_stream_to_opus(input_stream: &[f32]) -> Result<Vec<u8>, opus::Error> {
    let mut opus_encoder = Encoder::new(SAMPLE_RATE, CHANNELS, Application::Audio)?;
    let mut encoded_data = vec![0; 4000];
    let len = opus_encoder.encode_float(input_stream, &mut encoded_data)?;
    Ok(encoded_data[..len].to_vec())
}
// ============================================
//    Decode Opus to PCM Format
// ============================================
// Decode an audio stream  from Oputs format to PCM format
pub fn decode_opus_to_pcm(opus_data: &[u8]) -> Result<Vec<f32>, opus::Error> {
    let mut decoder = Decoder::new(SAMPLE_RATE, Channels::Stereo)?;
    let mut pcm_data = vec![0.0; opus_data.len() * 1];
    // FEC (Forward Error Correction) set to false
    let decoded_samples = decoder.decode_float(opus_data, &mut pcm_data, false)?;
    pcm_data.truncate(decoded_samples * 1);
    Ok(pcm_data)
}
