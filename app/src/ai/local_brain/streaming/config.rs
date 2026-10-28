use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct LocalStreamingConfig {
    pub emission: StreamEmissionConfig,
    pub timeouts: StreamTimeoutConfig,
    pub limits: StreamLimitConfig,
}

#[derive(Debug, Clone)]
pub struct StreamEmissionConfig {
    pub flush_interval: Duration,
    pub max_buffer_chars: usize,
    pub canonical_channel_capacity: usize,
}

#[derive(Debug, Clone)]
pub struct StreamTimeoutConfig {
    pub connect_timeout: Duration,
    pub first_event_timeout: Duration,
    pub idle_timeout: Duration,
    pub total_timeout: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct StreamLimitConfig {
    pub max_reasoning_bytes: usize,
    pub max_output_bytes: usize,
    pub max_tool_calls: usize,
    pub max_tool_argument_bytes: usize,
    pub max_total_streamed_bytes: usize,
}

impl Default for StreamEmissionConfig {
    fn default() -> Self {
        Self {
            flush_interval: Duration::from_millis(30),
            max_buffer_chars: 384,
            canonical_channel_capacity: 64,
        }
    }
}

impl Default for StreamTimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(15),
            first_event_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(90),
            total_timeout: None,
        }
    }
}

impl Default for StreamLimitConfig {
    fn default() -> Self {
        Self {
            max_reasoning_bytes: 1024 * 1024, // 1MB
            max_output_bytes: 1024 * 1024,    // 1MB
            max_tool_calls: 32,
            max_tool_argument_bytes: 512 * 1024,       // 512KB
            max_total_streamed_bytes: 2 * 1024 * 1024, // 2MB
        }
    }
}

#[derive(Debug)]
pub struct StreamingConfigError(pub String);

impl std::fmt::Display for StreamingConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "streaming config error: {}", self.0)
    }
}

impl std::error::Error for StreamingConfigError {}

impl StreamEmissionConfig {
    pub fn validate(&self) -> Result<(), StreamingConfigError> {
        if self.flush_interval.is_zero() {
            return Err(StreamingConfigError("flush_interval cannot be zero".into()));
        }
        if self.max_buffer_chars == 0 {
            return Err(StreamingConfigError(
                "max_buffer_chars cannot be zero".into(),
            ));
        }
        if self.canonical_channel_capacity == 0 {
            return Err(StreamingConfigError(
                "canonical_channel_capacity cannot be zero".into(),
            ));
        }
        Ok(())
    }
}

impl StreamTimeoutConfig {
    pub fn validate(&self) -> Result<(), StreamingConfigError> {
        if self.connect_timeout.is_zero() {
            return Err(StreamingConfigError(
                "connect_timeout cannot be zero".into(),
            ));
        }
        if self.first_event_timeout.is_zero() {
            return Err(StreamingConfigError(
                "first_event_timeout cannot be zero".into(),
            ));
        }
        if self.idle_timeout.is_zero() {
            return Err(StreamingConfigError("idle_timeout cannot be zero".into()));
        }
        if let Some(total) = self.total_timeout {
            if total <= self.connect_timeout {
                return Err(StreamingConfigError(
                    "total_timeout must be greater than connect_timeout".into(),
                ));
            }
        }
        Ok(())
    }
}

impl StreamLimitConfig {
    pub fn validate(&self) -> Result<(), StreamingConfigError> {
        if self.max_tool_calls == 0 {
            return Err(StreamingConfigError("max_tool_calls cannot be zero".into()));
        }
        if self.max_tool_argument_bytes == 0 {
            return Err(StreamingConfigError(
                "max_tool_argument_bytes cannot be zero".into(),
            ));
        }
        if self.max_total_streamed_bytes < self.max_output_bytes {
            return Err(StreamingConfigError(
                "max_total_streamed_bytes must be >= max_output_bytes".into(),
            ));
        }
        Ok(())
    }
}

impl LocalStreamingConfig {
    pub fn production() -> Self {
        let config = Self::default();

        #[cfg(debug_assertions)]
        let config = config.with_dev_env_overrides();
        config
    }

    pub fn validate(&self) -> Result<(), StreamingConfigError> {
        self.emission.validate()?;
        self.timeouts.validate()?;
        self.limits.validate()?;
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn with_dev_env_overrides(mut self) -> Self {
        if let Some(ms) = parse_duration_override("WARP_DEV_LOCAL_STREAM_FLUSH_MS", 5, 250) {
            self.emission.flush_interval = ms;
        }
        if let Some(chars) = parse_usize_override("WARP_DEV_LOCAL_STREAM_BUFFER_CHARS", 1, 10000) {
            self.emission.max_buffer_chars = chars;
        }
        if let Some(cap) = parse_usize_override("WARP_DEV_LOCAL_STREAM_CHANNEL_CAPACITY", 1, 1024) {
            self.emission.canonical_channel_capacity = cap;
        }
        if let Some(ms) =
            parse_duration_override("WARP_DEV_LOCAL_STREAM_CONNECT_TIMEOUT_MS", 100, 60000)
        {
            self.timeouts.connect_timeout = ms;
        }
        if let Some(ms) =
            parse_duration_override("WARP_DEV_LOCAL_STREAM_FIRST_EVENT_TIMEOUT_MS", 100, 120000)
        {
            self.timeouts.first_event_timeout = ms;
        }
        if let Some(ms) =
            parse_duration_override("WARP_DEV_LOCAL_STREAM_IDLE_TIMEOUT_MS", 100, 120000)
        {
            self.timeouts.idle_timeout = ms;
        }
        self
    }
}

#[cfg(debug_assertions)]
fn parse_duration_override(name: &str, minimum_ms: u64, maximum_ms: u64) -> Option<Duration> {
    let value = std::env::var(name).ok()?;
    let milliseconds = value.parse::<u64>().ok()?;

    if !(minimum_ms..=maximum_ms).contains(&milliseconds) {
        log::warn!("{name} ignored because it is outside the supported range");
        return None;
    }
    Some(Duration::from_millis(milliseconds))
}

#[cfg(debug_assertions)]
fn parse_usize_override(name: &str, minimum: usize, maximum: usize) -> Option<usize> {
    let value = std::env::var(name).ok()?;
    let val = value.parse::<usize>().ok()?;

    if !(minimum..=maximum).contains(&val) {
        log::warn!("{name} ignored because it is outside the supported range");
        return None;
    }
    Some(val)
}
