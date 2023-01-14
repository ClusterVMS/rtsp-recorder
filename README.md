# RTSP Recorder

Records an RTSP stream from an H.264 IP camera. Records as TS segments and an m3u8 playlist, suitable to serve up with HLS.

This is a minimal application, which just launches an ffmpeg process for each stream, restarting the process if it fails.

This application is primarily intended for use with ClusterVMS, but can also be used stand-alone.


## Usage

* Either build the docker container with `./docker-build.sh`, or build locally with `cargo build --release`.
* Create TOML files describing the cameras and streams you want forwarded. See `sample-config.toml` for format example.
	* When sharing config files between several ClusterVMS components, it's recommended to keep the login details and other sensitive config in separate files, accessible only to the applications that need them.
* Run the executable, pointing it to your config files
	* E.g. `./rtsp-recorder -c my-config.toml -c my-secret-config.toml`
* Serve up the recorded files in `/var/recordings/CAMERA_ID/STREAM_ID/` with your favorite http server.


## License

Licensed under either of the Apache License, Version 2.0 or the MIT License at your option. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
