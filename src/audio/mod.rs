use cpal::platform::Host;
use cpal::traits::{DeviceTrait, HostTrait};
use crate::log;

pub struct AudioStream;
pub struct FormattedAudio;

pub fn initialize_audio_interface() {
    // Open communication with the default audio interface

    // Get the default host
    let host: Host = cpal::default_host();

    // Get the default input device
    let input_device: Option<cpal::Device> = host.default_input_device();
    match input_device {
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
    match output_device {
        Some(device) => {
            match device.name() {
                Ok(name) => log::log_message(&format!("Default output device: {}", name)),
                Err(err) => log::log_message(&format!("Failed to get output device name: {}", err)),
            }
        },
        None => println!("Default input device found"),
    }
}

pub fn start_audio_stream() -> AudioStream {
    // Start the audio input/output stream
    todo!()
}

#[allow(unused_variables)]
pub fn stop_audio_stream(stream: AudioStream) {
    // Stop the audio stream
}

#[allow(unused_variables)]
pub fn convert_audio_format(stream: AudioStream) -> FormattedAudio {
    // Convert audio stream to a desired format
    todo!()
}
#[allow(unused_variables)]
pub fn play_audio(formatted_audio: FormattedAudio) {
        // Play the received audio data
}

