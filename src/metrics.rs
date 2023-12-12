use std::collections::HashMap;

use anyhow::{Context as AnyhowContext, Result};
use prometheus::{labels, opts, IntGauge, IntGaugeVec, Opts};

use crate::meta::MetaFile;

#[derive(Debug)]
pub struct MetricContext {
    pub nginx_build_info: IntGaugeVec,
    pub nginx_rtmp_application_count: IntGauge,
    pub nginx_rtmp_active_streams: IntGaugeVec,
    pub nginx_rtmp_incoming_bytes_total: IntGauge,
    pub nginx_rtmp_outgoing_bytes_total: IntGauge,
    pub nginx_rtmp_incoming_bandwidth: IntGauge,
    pub nginx_rtmp_outgoing_bandwidth: IntGauge,
    pub nginx_rtmp_stream_incoming_bytes_total: IntGaugeVec,
    pub nginx_rtmp_stream_outgoing_bytes_total: IntGaugeVec,
    pub nginx_rtmp_stream_incoming_bandwidth: IntGaugeVec,
    pub nginx_rtmp_stream_outgoing_bandwidth: IntGaugeVec,
    pub nginx_rtmp_stream_bandwidth_video: IntGaugeVec,
    pub nginx_rtmp_stream_bandwidth_audio: IntGaugeVec,
    pub nginx_rtmp_stream_publisher_avsync: IntGaugeVec,
    pub nginx_rtmp_stream_total_clients: IntGaugeVec,
}

impl MetricContext {
    /// Register a vector of integer gauges.
    ///
    /// TODO: These methods do a horrific amount of cloning for no good reason -
    /// pull request to upstream crate?
    fn register_int_gauge_vec(
        name: &'static str,
        description: &'static str,
        global_labels: &HashMap<String, String>,
        labels: &[&str],
    ) -> Result<IntGaugeVec> {
        let opts = Opts::new(name, description).const_labels(global_labels.clone());
        let labels: Vec<&str> = labels.iter().map(|x| &**x).collect();
        prometheus::register_int_gauge_vec!(opts, &labels).context("failed to create int gauge vec")
    }

    /// Register an integer gauge.
    fn register_int_gauge(
        name: &'static str,
        description: &'static str,
        global_labels: &HashMap<String, String>,
        labels: &[&str],
    ) -> Result<IntGauge> {
        let labels: Vec<String> = labels.iter().map(|x| x.to_string()).collect();
        let opts = Opts::new(name, description)
            .const_labels(global_labels.clone())
            .variable_labels(labels);
        prometheus::register_int_gauge!(opts).context("failed to create int gauge")
    }

    pub fn from_metadata(metadata: &MetaFile) -> Result<Self> {
        // register build info gauge
        prometheus::register_gauge!(opts!(
			"nginx_rtmp_exporter_build_info",
			"A metric with constant value '1', labelled with nginx-rtmp-exporter's build information.",
			labels! {
				"version" => env!("VERGEN_GIT_SEMVER"),
				"rustc_version" => env!("VERGEN_RUSTC_SEMVER"),
			}
		))
        .unwrap()
        .set(1.0);

        let global_labels = metadata.global_fields.clone().unwrap_or_default();

        // export metadata fields as metric
        let field_metric = Self::register_int_gauge_vec(
            "nginx_rtmp_exporter_metadata_fields",
            "A metric with constant value '1', labelled with available metadata fields.",
            &global_labels,
            &["field"],
        )?;

        // TODO - this can panic
        metadata.get_fields().iter().for_each(|field| {
            field_metric.with_label_values(&[field.as_str()]).set(1);
        });

        // export metadata values as metric
        let value_metric = Self::register_int_gauge_vec(
            "nginx_rtmp_exporter_metadata_values",
            "A metric with constant value '1', labelled with available metadata values.",
            &global_labels,
            &["stream", "field", "value"],
        )?;

        metadata.entries().iter().for_each(|(stream, field, value)| {
            value_metric
                .with_label_values(&[stream.as_str(), field.as_str(), value.as_str()])
                .set(1);
        });

        // create stream labels
        let mut labels = vec!["application", "stream"];
        metadata.get_fields().iter().for_each(|str| {
            labels.push(str.as_str());
        });
        let labels = &labels;

        Ok(Self {
            nginx_build_info: Self::register_int_gauge_vec(
                "nginx_build_info",
                "A metric with either '0' or '1', labelled with NGINX's build info when available.",
				&global_labels,
                &["version", "compiler", "rtmp_version"]
            )?,
			nginx_rtmp_application_count: Self::register_int_gauge(
				"nginx_rtmp_application_count",
				"A metric tracking the number of NGINX RTMP applications.",
				&global_labels,
				&[]
			)?,
			nginx_rtmp_active_streams: Self::register_int_gauge_vec(
				"nginx_rtmp_active_streams",
				"A metric tracking the number of active RTMP streams, labelled by application.",
				&global_labels,
				&["application"]
			)?,
            nginx_rtmp_incoming_bytes_total: Self::register_int_gauge(
                "nginx_rtmp_incoming_bytes_total",
                "A metric tracking the total number of incoming bytes processed.",
				&global_labels,
				&[]
			)?,
            nginx_rtmp_outgoing_bytes_total: Self::register_int_gauge(
                "nginx_rtmp_outgoing_bytes_total",
                "A metric tracking the total number of outgoing bytes processed.",
				&global_labels,
				&[]
			)?,
            nginx_rtmp_incoming_bandwidth: Self::register_int_gauge(
                "nginx_rtmp_incoming_bandwidth",
                "A metric tracking the incoming bandwidth to the server.",
				&global_labels,
				&[]
			)?,
            nginx_rtmp_outgoing_bandwidth: Self::register_int_gauge(
                "nginx_rtmp_outgoing_bandwidth",
                "A metric tracking the outgoing bandwidth from the server.",
				&global_labels,
				&[]
			)?,

            nginx_rtmp_stream_incoming_bytes_total: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_incoming_bytes_total",
				"A metric tracking the total received bytes from a stream, labelled by stream and application.",
				&global_labels,
                labels
            )?,
            nginx_rtmp_stream_outgoing_bytes_total: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_outgoing_bytes_total",
				"A metric tracking the total sent bytes by a given stream, labelled by stream and application.",
                &global_labels,
				labels
            )?,
            nginx_rtmp_stream_incoming_bandwidth: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_incoming_bandwidth",
				"A metric tracking the incoming bandwidth of a given stream, labelled by stream and application.",
                &global_labels,
				labels
            )?,
            nginx_rtmp_stream_outgoing_bandwidth: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_outgoing_bandwidth",
				"A metric tracking the outgoing bandwidth of a given stream, labelled by stream and application.",
                &global_labels,
				labels
            )?,
			nginx_rtmp_stream_bandwidth_video: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_bandwidth_video",
				"A metric tracking the video bandwidth of a given stream, labelled by stream and application.",
                &global_labels,
				labels
			)?,
			nginx_rtmp_stream_bandwidth_audio: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_bandwidth_audio",
				"A metric tracking the audio bandwidth of a given stream, labelled by stream and application.",
                &global_labels,
				labels
			)?,
			nginx_rtmp_stream_publisher_avsync: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_publisher_avsync",
				"A metric tracking the A-V sync value of a given stream, labelled by stream and application.",
				&global_labels,
				labels
			)?,
			nginx_rtmp_stream_total_clients: Self::register_int_gauge_vec(
				"nginx_rtmp_stream_total_clients",
				"A metric tracking the number of clients connected to a given stream, labelled by stream and application.",
				&global_labels,
				labels
			)?,
        })
    }
}
