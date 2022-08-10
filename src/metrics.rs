use tracing::{debug, trace, warn};

use crate::context::Context;

pub async fn collect_metrics(ctx: &mut Context) {
    debug!("collecting metrics...");
    // reset all metrics to prevent stale data
    // TODO: use existing metrics to remove extraneous labels
    trace!("resetting metrics...");
    ctx.metrics.nginx_build_info.reset();
    ctx.metrics.nginx_rtmp_incoming_bytes_total.set(0);
    ctx.metrics.nginx_rtmp_outgoing_bytes_total.set(0);
    ctx.metrics.nginx_rtmp_incoming_bandwidth.set(0);
    ctx.metrics.nginx_rtmp_outgoing_bandwidth.set(0);
    ctx.metrics.nginx_rtmp_stream_bandwidth_audio.reset();
    ctx.metrics.nginx_rtmp_stream_bandwidth_video.reset();
    ctx.metrics.nginx_rtmp_stream_incoming_bandwidth.reset();
    ctx.metrics.nginx_rtmp_stream_outgoing_bandwidth.reset();
    ctx.metrics.nginx_rtmp_stream_incoming_bytes_total.reset();
    ctx.metrics.nginx_rtmp_stream_outgoing_bytes_total.reset();
    ctx.metrics.nginx_rtmp_stream_publisher_avsync.reset();
    ctx.metrics.nginx_rtmp_stream_total_clients.reset();
    // fetch stats and handle errors
    let stats = ctx.fetch_rtmp_stats().await;
    if let Err(err) = stats {
        warn!("failed to fetch RTMP stats: {}", err);
        return;
    }
    let stats = stats.unwrap();
    // hydrate build info metric
    ctx.metrics
        .nginx_build_info
        .get_metric_with_label_values(&[
            &stats.nginx_version,
            &stats.compiler,
            &stats.nginx_rtmp_version,
        ])
        .unwrap()
        .set(1);
    // set root-level metrics
    ctx.metrics.nginx_rtmp_incoming_bytes_total.set(stats.bytes_in as i64);
    ctx.metrics.nginx_rtmp_outgoing_bytes_total.set(stats.bytes_out as i64);
    ctx.metrics.nginx_rtmp_incoming_bandwidth.set(stats.bw_in as i64);
    ctx.metrics.nginx_rtmp_outgoing_bandwidth.set(stats.bw_out as i64);
    // iterate through streams and set stats
    stats.server.applications.iter().for_each(|application| {
        // set active streams
        ctx.metrics.nginx_rtmp_active_streams.with_label_values(&[application.name.as_str()]).set(
            application
                .live
                .streams
                .iter()
                // ignore streams with no metadata defined
                .filter(|stream| stream.meta.is_some())
                // ignore streams that are only used as relays
                .filter(|stream| stream.clients.iter().any(|client| !client.is_local_relay()))
                .count() as i64,
        );
        // iterate over application streams
        application.live.streams.iter().for_each(|stream| {
            debug!("resolving information for stream {}", stream.name);
            // label values
            let mut lbs = vec![application.name.as_str(), stream.name.as_str()];
            // collect and append metadata values
            let meta = ctx.meta_provider.get_values_for(&stream.name);
            let mut meta: Vec<&str> = meta.iter().map(|s| &**s).collect();
            lbs.append(&mut meta);
            let lbs = &lbs;
            // incoming bytes
            let incoming_bytes = ctx
                .metrics
                .nginx_rtmp_stream_incoming_bytes_total
                .get_metric_with_label_values(lbs)
                .unwrap();
            incoming_bytes.set(stream.bytes_in as i64);
            // outgoing bytes
            let outgoing_bytes = ctx
                .metrics
                .nginx_rtmp_stream_outgoing_bytes_total
                .get_metric_with_label_values(lbs)
                .unwrap();
            outgoing_bytes.set(stream.bytes_out as i64);
            // incoming bandwidth
            ctx.metrics
                .nginx_rtmp_stream_incoming_bandwidth
                .with_label_values(lbs)
                .set(stream.bw_in as i64);
            // outgoing bandwidth
            ctx.metrics
                .nginx_rtmp_stream_outgoing_bandwidth
                .with_label_values(lbs)
                .set(stream.bw_out as i64);
            // video bandwidth
            ctx.metrics
                .nginx_rtmp_stream_bandwidth_video
                .with_label_values(lbs)
                .set(stream.bw_video as i64);
            // audio bandwidth
            ctx.metrics
                .nginx_rtmp_stream_bandwidth_audio
                .with_label_values(lbs)
                .set(stream.bw_audio as i64);
            // avsync
            // if this stream includes audio, set avsync
            if stream.bw_audio != 0 {
                match stream.clients.iter().find(|client| client.publishing.is_some()) {
                    Some(client) => {
                        ctx.metrics
                            .nginx_rtmp_stream_publisher_avsync
                            .with_label_values(lbs)
                            .set(client.avsync);
                    }
                    None => (),
                };
            }
            // connected clients
            // total clients - 1, 1 publisher
            ctx.metrics
                .nginx_rtmp_stream_total_clients
                .with_label_values(lbs)
                .set((stream.clients.len() - 1) as i64);
        })
    });
}
