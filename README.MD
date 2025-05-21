# Stereo Unity Gain

A tool intended to help set the level for stereo pairs of inputs your audio interface or other audio devices connected to your computer.

***_****This is still under serious development but it does usually work at least on Apple Silicon MacOS_***

It will also work for setting mono channel input levels and some other use cases as well so use your imagination.

There are 2 components to the application:

- The Output Stage - Tone Generator
    - This generates a 1kHz sine wave at -12 dbfs and sends it to your selected
      output device channels.
- The Input Stage - Level Meter
    - This will read sample frames from your input device channels one audio buffer worth at a time (using the default buffer size of your device)
      and calculates a peak value for that buffer.
    - There are 2 modes for the input meter controlled by the slider switch above the start button.
        - Delta Mode - The default mode. The level meter will show the differnece between the input level it reads and the reference output level.
            - ie. If the output is -12 dbfs and it reads -14 dbfs it will show -2.0 indicating that your input is 2 dbfs quieter than the output
              reference tone and you should increase the input gain.
            - The meters will show blue if the signal is too "cold" or quieter than reference and red if it is too "hot" or louder than the reference.
              it shows green if you are within +/- 0.1 dbfs of the reference tone level
        - Peak Mode - ie. Delta is disabled. This just shows the rolling peak level of your input signal

Build Steps:

- You will need a Rust language tool chain setup. You can get it here https://rustup.rs/
- Then grab this code base from gitlab (assuming you have git installed).
- In the root of the git repository you downloaded run "cargo build -r" this will generate a release binary
- The binary will be in the target/release folder and named stereo-unity-gain. Grab that in put it somewhere you can run it.
- Then just execute that binary to start the app

Usage Steps

- Connect the audio device/devices to your computer and make sure your OS recognizes them
- Physically connect a pair of audio outputs to inputs
    - I just use a pair of balanced patch cables and plug the outputs to the inputs on my
      patch bay or right on the ports on the audio interface
- Start Stereo Unity Gain
- Select the input and out devices in the center drop down menu
- Select the left and right input and output channels from the drop downs to either side of the device dropdown
- Hit the start button
- Dial in the input gain on your audio device until the level meters show 0.0 or what ever value you want to target


