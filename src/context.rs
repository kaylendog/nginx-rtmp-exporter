use std::time::Duration;

use anyhow::{Context as AnyhowContext, Result};
use reqwest::{Client, Url};
use tracing::{debug, trace, warn};

use crate::{meta::MetaFile, metrics::MetricContext};

#[derive(Debug)]
pub struct Context {
    pub http: Client,
    pub metadata: MetaFile,
    pub metrics: MetricContext,
    pub rtmp_stats_endpoint: Url,
}

impl Context {
    pub fn new(endpoint: Url, metadata: MetaFile) -> Result<Self> {
        let metrics =
            MetricContext::from_metadata(&metadata).context("failed to create MetricContext")?;
        // create context
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .expect("failed to build reqwest client"),
            metadata,
            metrics,
            rtmp_stats_endpoint: endpoint,
        })
    }

    pub async fn collect_metrics(&mut self) {
        debug!("collecting metrics...");
        // reset all metrics to prevent stale data
        // TODO: use existing metrics to remove extraneous labels
        trace!("resetting metrics...");
        self.metrics.nginx_build_info.reset();
        self.metrics.nginx_rtmp_incoming_bytes_total.set(0);
        self.metrics.nginx_rtmp_outgoing_bytes_total.set(0);
        self.metrics.nginx_rtmp_incoming_bandwidth.set(0);
        self.metrics.nginx_rtmp_outgoing_bandwidth.set(0);
        self.metrics.nginx_rtmp_stream_bandwidth_audio.reset();
        self.metrics.nginx_rtmp_stream_bandwidth_video.reset();
        self.metrics.nginx_rtmp_stream_incoming_bandwidth.reset();
        self.metrics.nginx_rtmp_stream_outgoing_bandwidth.reset();
        self.metrics.nginx_rtmp_stream_incoming_bytes_total.reset();
        self.metrics.nginx_rtmp_stream_outgoing_bytes_total.reset();
        self.metrics.nginx_rtmp_stream_publisher_avsync.reset();
        self.metrics.nginx_rtmp_stream_total_clients.reset();
        // fetch stats and handle errors
        let stats = self.fetch_rtmp_stats().await;
        if let Err(err) = stats {
            warn!("failed to fetch RTMP stats: {}", err);
            return;
        }
        let stats = stats.unwrap();
        // hydrate build info metric
        self.metrics
            .nginx_build_info
            .get_metric_with_label_values(&[
                &stats.nginx_version,
                &stats.compiler,
                &stats.nginx_rtmp_version,
            ])
            .unwrap()
            .set(1);
        // set root-level metrics
        self.metrics.nginx_rtmp_incoming_bytes_total.set(stats.bytes_in as i64);
        self.metrics.nginx_rtmp_outgoing_bytes_total.set(stats.bytes_out as i64);
        self.metrics.nginx_rtmp_incoming_bandwidth.set(stats.bw_in as i64);
        self.metrics.nginx_rtmp_outgoing_bandwidth.set(stats.bw_out as i64);
        // iterate through streams and set stats
        stats.server.applications.iter().for_each(|application| {
            // set active streams
            self.metrics
                .nginx_rtmp_active_streams
                .with_label_values(&[application.name.as_str()])
                .set(
                    application
                        .live
                        .streams
                        .iter()
                        // ignore streams with no metadata defined
                        .filter(|stream| stream.meta.is_some())
                        // ignore streams that are only used as relays
                        .filter(|stream| {
                            stream.clients.iter().any(|client| !client.is_local_relay())
                        })
                        .count() as i64,
                );
            // iterate over application streams
            application.live.streams.iter().for_each(|stream| {
                debug!("resolving information for stream {}", stream.name);
                // label values
                let mut lbs = vec![application.name.as_str(), stream.name.as_str()];

                // if let Some(globals) = &self.metadata.global_fields {
                //     globals.keys().for_each(|key| {
                //         lbs.push(globals.get(key).unwrap().as_str());
                //     });
                // }

                // collect and append metadata values
                let meta = self.metadata.get_values_for(&stream.name);
                let mut meta: Vec<&str> = meta.iter().map(|s| &**s).collect();
                lbs.append(&mut meta);
                let lbs = &lbs;

                // incoming bytes
                let incoming_bytes = self
                    .metrics
                    .nginx_rtmp_stream_incoming_bytes_total
                    .get_metric_with_label_values(lbs)
                    .unwrap();
                incoming_bytes.set(stream.bytes_in as i64);

                // outgoing bytes
                let outgoing_bytes = self
                    .metrics
                    .nginx_rtmp_stream_outgoing_bytes_total
                    .get_metric_with_label_values(lbs)
                    .unwrap();
                outgoing_bytes.set(stream.bytes_out as i64);

                // incoming bandwidth
                self.metrics
                    .nginx_rtmp_stream_incoming_bandwidth
                    .with_label_values(lbs)
                    .set(stream.bw_in as i64);

                // outgoing bandwidth
                self.metrics
                    .nginx_rtmp_stream_outgoing_bandwidth
                    .with_label_values(lbs)
                    .set(stream.bw_out as i64);

                // video bandwidth
                self.metrics
                    .nginx_rtmp_stream_bandwidth_video
                    .with_label_values(lbs)
                    .set(stream.bw_video as i64);

                // audio bandwidth
                self.metrics
                    .nginx_rtmp_stream_bandwidth_audio
                    .with_label_values(lbs)
                    .set(stream.bw_audio as i64);

                // avsync
                // if this stream includes audio, set avsync
                if stream.bw_audio != 0 {
                    if let Some(client) =
                        stream.clients.iter().find(|client| client.publishing.is_some())
                    {
                        self.metrics
                            .nginx_rtmp_stream_publisher_avsync
                            .with_label_values(lbs)
                            .set(client.avsync);
                    }
                }
                // connected clients
                // total clients - 1, 1 publisher
                self.metrics
                    .nginx_rtmp_stream_total_clients
                    .with_label_values(lbs)
                    .set((stream.clients.len() - 1) as i64);
            })
        });
    }
}
