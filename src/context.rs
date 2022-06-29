use prometheus::{labels, opts, IntCounter, IntCounterVec, IntGauge, IntGaugeVec};
use reqwest::Url;

use crate::meta::MetaProvider;

macro_rules! append {
    ( $lhs:expr, $rhs:expr ) => {
        &{
            let mut lhs: Vec<&str> = $lhs.to_vec();
            let mut rhs: Vec<&str> = $rhs.iter().map(|s| &**s).collect();
            let mut output: Vec<&str> = Vec::new();
            output.append(&mut lhs);
            output.append(&mut rhs);
            output
        }
    };
}

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
        // create stream labels
        let labels = append!(&["stream", "application"], meta_provider.get_fields());
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
