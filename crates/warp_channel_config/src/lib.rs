//! Loads a per-channel [`ChannelConfig`] for Warp's channel binaries.
//!
//! For non-bundled builds the internal `warp-channel-config` generator is
//! invoked at runtime; for `release_bundle` builds the config is embedded at
//! compile time via the consuming crate's build script. Shared by the GUI app
//! binaries and the `warp_tui` binaries so the loading logic lives in one place.
//!
//! The `release_bundle` cfg inside [`load_config!`] is evaluated in the
//! *consuming* crate, so each binary crate opts into embedding by defining its
//! own `release_bundle` feature (and generating `<channel>_config.json` into its
//! `OUT_DIR` from a build script).
use warp_core::channel::ChannelConfig;

/// The name of the config generator binary, expected to be on PATH.
const CONFIG_BIN_NAME: &str = "warp-channel-config";

#[macro_export]
#[cfg(windows)]
macro_rules! path_concat {
    ($path:expr, $file:expr) => {
        concat!($path, "\\", $file)
    };
}
#[macro_export]
#[cfg(not(windows))]
macro_rules! path_concat {
    ($path:expr, $file:expr) => {
        concat!($path, "/", $file)
    };
}

/// Loads the [`ChannelConfig`] for the given channel name.
///
/// In `release_bundle` builds the config is embedded at compile time (the
/// consuming crate's build script must generate `<channel>_config.json` into
/// `OUT_DIR`); otherwise the `warp-channel-config` generator is invoked at
/// runtime.
#[macro_export]
macro_rules! load_config {
    ($channel:expr) => {{
        #[cfg(feature = "release_bundle")]
        {
            $crate::load_config_from_embedded(include_str!($crate::path_concat!(
                env!("OUT_DIR"),
                concat!($channel, "_config.json")
            )))
        }

        #[cfg(not(feature = "release_bundle"))]
        {
            $crate::load_config_from_generator($channel)
        }
    }};
}

/// Invokes the config generator binary at runtime and deserializes its JSON
/// output into a [`ChannelConfig`].
pub fn load_config_from_generator(channel: &str) -> ChannelConfig {
    let target_family = if cfg!(target_family = "wasm") {
        "wasm"
    } else {
        "native"
    };

    let target_os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    };

    let output = command::blocking::Command::new(CONFIG_BIN_NAME)
        .arg("--channel")
        .arg(channel)
        .arg("--target-family")
        .arg(target_family)
        .arg("--target-os")
        .arg(target_os)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            if let Ok(config) = serde_json::from_slice(&out.stdout) {
                return config;
            }
        }
        _ => {}
    }

    ChannelConfig {
        app_id: warp_core::AppId::new("dev", "warp", "Warp"),
        logfile_name: "warp.log".into(),
        server_config: warp_core::channel::WarpServerConfig::production(),
        oz_config: warp_core::channel::OzConfig::production(),
        telemetry_config: None,
        crash_reporting_config: None,
        autoupdate_config: None,
        mcp_static_config: None,
    }
}

/// Deserializes a [`ChannelConfig`] from a JSON string embedded at compile time.
///
/// Used to load channel configuration in release bundles, where configuration
/// is embedded at compile time instead of being generated at runtime.
pub fn load_config_from_embedded(json: &str) -> ChannelConfig {
    serde_json::from_str(json)
        .unwrap_or_else(|err| panic!("Failed to parse embedded channel config: {err}"))
}
