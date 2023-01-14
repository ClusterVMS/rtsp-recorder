use async_std::task;
use clap::{AppSettings, App, Arg};
use clustervms::{Camera, CameraId, StreamId};
use clustervms::config;
use core::time::Duration;
use duct::cmd;
use log::{error, warn};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;



#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let mut app = App::new("rtsp-recorder")
		.version("0.1.0")
		.author("Alicrow")
		.about("Records an RTSP stream.")
		.setting(AppSettings::DeriveDisplayOrder)
		.setting(AppSettings::SubcommandsNegateReqs)
		.arg(
			Arg::new("config")
				.takes_value(true)
				.multiple(true)
				.short('c')
				.long("config")
				.help("TOML file with ClusterVMS config")
		)
		.arg(
			Arg::new("FULLHELP")
				.help("Prints more detailed help information")
				.long("fullhelp"),
		)
		.arg(
			Arg::new("debug")
				.long("debug")
				.short('d')
				.help("Prints debug log information"),
		);

	let matches = app.clone().get_matches();

	if matches.is_present("FULLHELP") {
		app.print_long_help().unwrap();
		std::process::exit(0);
	}

	let debug = matches.is_present("debug");

	let log_level_filter = match debug {
		true => log::LevelFilter::Trace,
		false => log::LevelFilter::Info,
	};

	env_logger::Builder::new()
		.format(|buf, record| {
			writeln!(
				buf,
				"{}:{} [{}] {} - {}",
				record.file().unwrap_or("unknown"),
				record.line().unwrap_or(0),
				record.level(),
				chrono::Local::now().format("%H:%M:%S.%6f"),
				record.args()
			)
		})
		.filter(None, log_level_filter)
		.init();

	let config_filenames = matches.values_of("config").unwrap().collect();


	let mut config_manager = config::ConfigManager::new();
	config_manager.read_config(config_filenames)?;

	for (camera_id, camera_info) in &config_manager.get_config().cameras {
		for stream_id in camera_info.streams.keys() {
			spawn_recorder_process(camera_info.clone(), camera_id.clone().to_string(), stream_id.clone().to_string()).await;
		}
	}

	// Main task just sleeps; each process is monitored by its own task/thread.
	loop {
		task::sleep(Duration::from_secs(60)).await;
	}

	anyhow::Ok(())
}


async fn spawn_recorder_process(camera_info: Camera, camera_id: CameraId, stream_id: StreamId) {
	// Task to spawn and monitor an ffmpeg process for the stream
	tokio::spawn(async move {
		loop {
			let mut source_url = camera_info.streams[&stream_id].source_url.clone();
			if let Some(username) = &camera_info.username {
				source_url.set_username(username);
			}
			if let Some(password) = &camera_info.password {
				source_url.set_password(Some(password));
			}
			
			let filename = Path::new("/var/recordings").join(&camera_id).join(&stream_id).join("combined.m3u8");

			// create parent folders if they don't already exist
			match filename.parent() {
				Some(parent_folder) => {
					if let Err(err) = fs::create_dir_all(parent_folder) {
						error!("Failed to create directory {} for recordings; error: {err}", parent_folder.display());
						// We'll try again in a minute, in case the issue gets resolved.
						task::sleep(Duration::from_secs(60)).await;
						continue;
					}
				}
				None => {}
			}

			let cmd = cmd!("ffmpeg",
				"-loglevel", "warning",
				"-rtsp_transport", "tcp",	// Needed for mysterious reasons to avoid "frame size not set" in alpine docker container, but not on Manjaro or Ubuntu... see https://stackoverflow.com/questions/61028524/could-not-find-codec-parameters-for-stream-0-video-hevc-none-unspecified-si
				"-i", source_url.as_str(),
				"-strftime", "1",
				"-c", "copy",
				"-flags", "+cgop", "-g", "30",
				"-hls_time", "10",
				"-hls_list_size", "60480",  // How long a list to keep.
				"-hls_flags", "append_list+delete_segments",   // Append to the list instead of starting over again
				filename,
			);

			// Read stdout and stderr of child process, and log it with identifier for camera and stream
			if let Ok(reader) = cmd.stderr_to_stdout().reader() {
				let mut bufread = BufReader::new(reader);
				let mut buf = String::new();

				while let Ok(bytes_read) = bufread.read_line(&mut buf) {
					if bytes_read > 0 {
						println!("Camera {camera_id} stream {stream_id}: {}", buf.trim());
						buf.clear();
					} else {
						break;
					}
				}

				warn!("Recorder process for camera {camera_id}, stream {stream_id} exited");
			} else {
				error!("Failed to spawn recorder process for camera {camera_id}, stream {stream_id}");
			}

			// Sleep for a bit after getting disconnected or failing to connect.
			// If the issue persists, we don't want to waste all our time constantly trying to reconnect.
			task::sleep(Duration::from_secs(1)).await;
		}
	});
}
