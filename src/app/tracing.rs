use std::fs::File;
use std::sync::Mutex;
use tracing_perfetto::PerfettoLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

/// Initializes the application's tracing infrastructure:
/// - Terminal console output via `tracing_subscriber::fmt` (filtered by `RUST_LOG`, default `info`).
/// - Perfetto trace output (`.pftrace`) via `tracing_perfetto::PerfettoLayer` (filtered by `PERFETTO_LOG`, default `earth=trace,info`).
///   The output file can be loaded directly into <https://ui.perfetto.dev>.
pub fn init_tracing() -> anyhow::Result<()> {
    let trace_path = std::env::var("PERFETTO_TRACE_FILE")
        .unwrap_or_else(|_| "earth.pftrace".to_string());

    let file = File::create(&trace_path)
        .map_err(|e| anyhow::anyhow!("Failed to create Perfetto trace file at {trace_path}: {e}"))?;

    let perfetto_filter = tracing_subscriber::EnvFilter::try_from_env("PERFETTO_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("earth=trace,info"));

    let perfetto_layer = PerfettoLayer::new(Mutex::new(file))
        .with_debug_annotations(true)
        .with_filter(perfetto_filter);

    let fmt_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_filter(fmt_filter);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(perfetto_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing subscriber: {e}"))?;

    tracing::info!(
        trace_file = %trace_path,
        "Tracing initialized. Perfetto trace will be saved to '{}'. View it at https://ui.perfetto.dev",
        trace_path
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfetto_trace_generation() {
        let trace_path = std::path::PathBuf::from("earth_test.pftrace");

        let file = File::create(&trace_path).expect("failed to create temp trace file");
        let perfetto_layer = PerfettoLayer::new(Mutex::new(file)).with_debug_annotations(true);

        let subscriber = tracing_subscriber::registry().with(perfetto_layer);

        // Run tracing inside local subscriber
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("test_root_span", frame = 1);
            let _enter = span.enter();

            tracing::info!(event_name = "test_event", "inside root span");

            {
                let sub_span = tracing::trace_span!("sub_operation", duration_ms = 42);
                let _sub_enter = sub_span.enter();
                tracing::debug!("nested work");
            }
        });

        // Verify the file was written and is non-empty
        let metadata = std::fs::metadata(&trace_path).expect("trace file should exist");
        assert!(metadata.len() > 0, "Perfetto trace file should not be empty");
        let _ = std::fs::remove_file(trace_path);
    }
}
