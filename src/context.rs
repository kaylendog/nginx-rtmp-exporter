use prometheus::{labels, opts, IntCounter, IntCounterVec, IntGauge, IntGaugeVec};
use reqwest::Url;

use crate::meta::MetaProvider;

#[derive(Debug)]
pub struct Context {
    pub rtmp_stats_endpoint: Url,
    pub nginx_build_info: IntGaugeVec,
    pub nginx_rtmp_application_count: IntGauge,
    pub nginx_rtmp_active_streams: IntGauge,
    pub nginx_rtmp_incoming_bytes_total: IntCounter,
    pub nginx_rtmp_outgoing_bytes_total: IntCounter,
    pub nginx_rtmp_incoming_bandwidth: IntGauge,
    pub nginx_rtmp_outgoing_bandwidth: IntGauge,
    pub nginx_rtmp_stream_incoming_bytes_total: IntCounterVec,
    pub nginx_rtmp_stream_outgoing_bytes_total: IntCounterVec,
    pub nginx_rtmp_stream_incoming_bandwidth: IntGaugeVec,
    pub nginx_rtmp_stream_outgoing_bandwidth: IntGaugeVec,
    pub nginx_rtmp_stream_bandwidth_video: IntGaugeVec,
    pub nginx_rtmp_stream_bandwidth_audio: IntGaugeVec,
    pub nginx_rtmp_stream_publisher_avsync: IntGaugeVec,
    pub nginx_rtmp_stream_total_clients: IntGaugeVec,
    pub meta_provider: MetaProvider,
}

impl Context {
    pub fn new(endpoint: Url, meta_provider: MetaProvider) -> Self {
        // register build info gauge
        prometheus::register_gauge!(opts!(
			"nginx_rtmp_exporter_build_info",
			"A metric with constant value '1', labelled with nginx-rtmp-exporter's build information.",
			labels! {
				"version" => env!("VERGEN_GIT_SEMVER"),
				"rustc_version" => env!("VERGEN_RUSTC_SEMVER")
			}
		))
        .unwrap()
        .set(1.0);
        // export metadata fields as metric
        let field_metric = prometheus::register_int_gauge_vec!(
            opts!(
                "nginx_rtmp_exporter_metadata_fields",
                "A metric with constant value '1', labelled with available metadata fields."
            ),
            &["field"]
        )
        .unwrap();
        meta_provider.get_fields().iter().for_each(|field| {
            field_metric.with_label_values(&[field.as_str()]).set(1);
        });
        // export metadata values as metric
        let value_metric = prometheus::register_int_gauge_vec!(
            opts!(
                "nginx_rtmp_exporter_metadata_values",
                "A metric with constant value '1', labelled with available metadata values."
            ),
            &["stream", "field", "value"]
        )
        .unwrap();
        meta_provider.entries().iter().for_each(|(stream, field, value)| {
            value_metric
                .with_label_values(&[stream.as_str(), field.as_str(), value.as_str()])
                .set(1);
        });
        // create stream labels
        let mut labels = vec!["application", "stream"];
        let mut fields = meta_provider.get_fields().iter().map(|s| &**s).collect();
        labels.append(&mut fields);
        let labels = &labels;
        // create context
        Context {
            rtmp_stats_endpoint: endpoint,
            nginx_build_info: prometheus::register_int_gauge_vec!(
                opts!(
                "nginx_build_info",
                "A metric with either '0' or '1', labelled with NGINX's build info when available.",
            ),
                &["version", "compiler", "rtmp_version"]
            )
            .unwrap(),
            nginx_rtmp_application_count: prometheus::register_int_gauge!(opts!(
                "nginx_rtmp_application_count",
                "A metric tracking the number of NGINX RTMP applications."
            ))
            .unwrap(),
            nginx_rtmp_active_streams: prometheus::register_int_gauge!(opts!(
                "nginx_rtmp_active_streams",
                "A metric tracking the number of active RTMP streams."
            ))
            .unwrap(),
            nginx_rtmp_incoming_bytes_total: prometheus::register_int_counter!(opts!(
                "nginx_rtmp_incoming_bytes_total",
                "A metric tracking the total number of incoming bytes processed."
            ))
            .unwrap(),
            nginx_rtmp_outgoing_bytes_total: prometheus::register_int_counter!(opts!(
                "nginx_rtmp_outgoing_bytes_total",
                "A metric tracking the total number of outgoing bytes processed."
            ))
            .unwrap(),
            nginx_rtmp_incoming_bandwidth: prometheus::register_int_gauge!(opts!(
                "nginx_rtmp_incoming_bandwidth",
                "A metric tracking the incoming bandwidth to the server."
            ))
            .unwrap(),
            nginx_rtmp_outgoing_bandwidth: prometheus::register_int_gauge!(opts!(
                "nginx_rtmp_outgoing_bandwidth",
                "A metric tracking the outgoing bandwidth from the server."
            ))
            .unwrap(),

            nginx_rtmp_stream_incoming_bytes_total: prometheus::register_int_counter_vec!(
                opts!(
                    "nginx_rtmp_stream_incoming_bytes_total",
                    "A metric tracking the total received bytes from a stream, labelled by stream and application."
                ),
                labels
            )
            .unwrap(),
            nginx_rtmp_stream_outgoing_bytes_total: prometheus::register_int_counter_vec!(
                opts!(
                    "nginx_rtmp_stream_outgoing_bytes_total",
                    "A metric tracking the total sent bytes by a given stream, labelled by stream and application."
                ),
                labels
            )
            .unwrap(),
            nginx_rtmp_stream_incoming_bandwidth: prometheus::register_int_gauge_vec!(
                opts!(
					"nginx_rtmp_stream_incoming_bandwidth",
					"A metric tracking the incoming bandwidth of a given stream, labelled by stream and application."
				),
                labels
            )
            .unwrap(),
            nginx_rtmp_stream_outgoing_bandwidth: prometheus::register_int_gauge_vec!(
                opts!(
					"nginx_rtmp_stream_outgoing_bandwidth",
					"A metric tracking the outgoing bandwidth of a given stream, labelled by stream and application."
				),
                labels
            )
            .unwrap(),
			nginx_rtmp_stream_bandwidth_video: prometheus::register_int_gauge_vec!(
				opts!(
					"nginx_rtmp_stream_bandwidth_video",
					"A metric tracking the video bandwidth of a given stream, labelled by stream and application."
				),
                labels
			).unwrap(),
			nginx_rtmp_stream_bandwidth_audio: prometheus::register_int_gauge_vec!(
				opts!(
					"nginx_rtmp_stream_bandwidth_audio",
					"A metric tracking the audio bandwidth of a given stream, labelled by stream and application."
				),
                labels
			).unwrap(),
			nginx_rtmp_stream_publisher_avsync: prometheus::register_int_gauge_vec!(
				opts!(
					"nginx_rtmp_stream_publisher_avsync",
					"A metric tracking the A-V sync value of a given stream, labelled by stream and application."),
				labels
			).unwrap(),
			nginx_rtmp_stream_total_clients: prometheus::register_int_gauge_vec!(
				opts!(
					"nginx_rtmp_stream_total_clients",
					"A metric tracking the number of clients connected to a given stream, labelled by stream and application."
				),
				labels
			).unwrap(),
			meta_provider
        }
    }
}
