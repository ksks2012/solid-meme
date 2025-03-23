use hound::{WavReader, WavWriter};

fn remove_silence(input_path: &str, output_path: &str, silence_threshold: f32, min_silence_len: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Read the WAV file
    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();
    let channels = spec.channels as usize; // Number of channels
    let sample_rate = spec.sample_rate as usize;

    // Read samples into a Vec, considering multiple channels
    let samples: Vec<i16> = reader.samples().map(|s| s.unwrap()).collect();
    let total_samples = samples.len();

    // Ensure the number of samples is a multiple of the number of channels
    if total_samples % channels != 0 {
        eprintln!("Warning: The number of samples {} is not a multiple of the number of channels {}, data might be corrupted", total_samples, channels);
        return Err("Number of samples does not match the number of channels".into());
    }

    let mut result_samples = Vec::new();
    let mut silence_count = 0;

    for i in (0..total_samples).step_by(channels) {
        // Calculate the average amplitude of this frame
        let mut frame_amplitude = 0.0;
        for ch in 0..channels {
            let sample = samples[i + ch] as f32;
            frame_amplitude += sample.abs() / i16::MAX as f32;
        }
        frame_amplitude /= channels as f32;

        if frame_amplitude < silence_threshold {
            silence_count += 1;
        } else {
            if silence_count < min_silence_len / (sample_rate / 1000) { // Convert milliseconds to frame count
                // If the silence is too short, keep the previous frames
                for _ in 0..silence_count {
                    for _ in 0..channels {
                        result_samples.push(0); // Fill silence with 0
                    }
                }
            }
            silence_count = 0;
            // Keep all channel samples of this frame
            for ch in 0..channels {
                result_samples.push(samples[i + ch]);
            }
        }
    }

    // Write to a new file
    let mut writer = WavWriter::create(output_path, spec)?;
    for sample in result_samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    println!("Processing completed, saved to {}", output_path);
    Ok(())
}

fn main() {
    let input = "./var/input.wav";
    let output = "./var/output.wav";
    if let Err(e) = remove_silence(input, output, 0.01, 1000) { // Threshold 0.01, minimum silence 1000ms
        eprintln!("Error: {}", e);
    }
}