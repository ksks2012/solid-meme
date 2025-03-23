# Sound Editing Tool

This tool allows you to load, process, and play WAV audio files. It provides functionalities to remove silence from audio files and visualize the waveform of both the original and processed audio.

## Features

- **Load Audio**: Load a WAV audio file for processing.
- **Remove Silence**: Remove silence from the loaded audio file based on a specified threshold and minimum silence length.
- **Export**: Save the processed audio file as a new WAV file.
- **Play Original**: Play the original loaded audio file.
- **Play Processed**: Play the processed audio file.
- **Stop**: Stop the playback of the audio file.
- **Waveform Visualization**: Visualize the waveform of both the original and processed audio files.
- **Zoom and Pan**: Zoom in and out of the waveform and pan to different parts of the audio.

## Usage

1. **Load Audio**: Click the "Load Audio" button to load a WAV file.
2. **Remove Silence**: Click the "Remove Silence" button to remove silence from the loaded audio.
3. **Export**: Click the "Export" button to save the processed audio as a new WAV file.
4. **Play Original**: Click the "Play Original" button to play the original audio.
5. **Play Processed**: Click the "Play Processed" button to play the processed audio.
6. **Stop**: Click the "Stop" button to stop the playback.
7. **Zoom and Pan**: Use the mouse scroll wheel to zoom in and out of the waveform. Click and drag to pan across the waveform.

# Installation

To run the application, you need to have Rust installed. Clone the repository and run the following commands:

## Environment Setup (Ubuntu/Debian)

To install the necessary packages for this project, run the following command:

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libglib2.0-dev libatk1.0-dev libgtk-3-dev libcairo2-dev libpango1.0-dev libasound2-dev
```
## Project Structure

```
src/
├── main.rs         # Entry point of the application
├── app.rs          # SoundApp structure and core logic
├── audio.rs        # Audio processing and playback functionalities
├── ui.rs           # UI rendering and interaction logic
└── lib.rs          # Optional, defines public modules (if needed as a library)
```
