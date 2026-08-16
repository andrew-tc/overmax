pub mod detection_pipeline;
pub mod detection_worker;
pub mod hysteresis;
pub mod play_state;
pub mod roi;
pub mod telemetry;
pub mod templates;

pub use telemetry::{PipelineStatsCollector, PipelineTelemetrySnapshot, TimingAggregator};
pub use templates::RateTelemetry;
