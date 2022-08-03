use std::{error::Error, collections::HashMap};

use prometheus::core::Collector;
use tracing::debug;

use crate::{context::Context, xml::fetch_nginx_stats};

pub async fn collect_metrics(ctx: &mut Context) -> Result<(), Box<dyn Error>> {
    debug!("collecting metrics...");
    let stats = fetch_nginx_stats(&ctx.rtmp_stats_endpoint).await?;
    // reset the vector
    ctx.nginx_build_info.reset();
    // hydrate with build info
    ctx.nginx_build_info
        .get_metric_with_label_values(&[
            &stats.nginx_version,
            &stats.compiler,
            &stats.nginx_rtmp_version,
        ])
        .unwrap()
        .set(1);
    // incoming bytes
    ctx.nginx_rtmp_incoming_bytes_total.set(stats.bytes_in as i64);
    // outgoing bytes
    ctx.nginx_rtmp_outgoing_bytes_total.set(stats.bytes_out as i64);
    // incoming bandwidth
    ctx.nginx_rtmp_incoming_bandwidth.set(stats.bw_in as i64);
    // outgoing bandwidth
    ctx.nginx_rtmp_outgoing_bandwidth.set(stats.bw_out as i64);

	// reset all metrics to prevent stale data
	// TODO: use existing metrics to remove extraneous labels
	ctx.nginx_rtmp_stream_bandwidth_audio.reset();
	ctx.nginx_rtmp_stream_bandwidth_video.reset();
	ctx.nginx_rtmp_stream_incoming_bandwidth.reset();
	ctx.nginx_rtmp_stream_outgoing_bandwidth.reset();
	ctx.nginx_rtmp_stream_incoming_bytes_total.reset();
	ctx.nginx_rtmp_stream_outgoing_bytes_total.reset();
	ctx.nginx_rtmp_stream_publisher_avsync.reset();
	ctx.nginx_rtmp_stream_total_clients.reset();

    // iterate through streams and set stats
    stats.server.applications.iter().for_each(|application| {
        // set active streams
        ctx.nginx_rtmp_active_streams.with_label_values(&[application.name.as_str()]).set(
            application
                .live
                .streams
                .iter()
                // ignore streams with no metadata defined
                .filter(|stream| stream.meta.is_some())
                // ignore streams that are only used as relays
                .filter(|stream| stream.clients.iter().any(|client| !client.is_local_relay()))
                .collect::<Vec<_>>()
                .len() as i64,
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
            // reference labels
            let lbs = &lbs;
            // incoming bytes
            let incoming_bytes = ctx
                .nginx_rtmp_stream_incoming_bytes_total
                .get_metric_with_label_values(lbs)
                .unwrap();
            incoming_bytes.set(stream.bytes_in as i64);
            // outgoing bytes
            let outgoing_bytes = ctx
                .nginx_rtmp_stream_outgoing_bytes_total
                .get_metric_with_label_values(lbs)
                .unwrap();
            outgoing_bytes.set(stream.bytes_out as i64);
            // incoming bandwidth
            ctx.nginx_rtmp_stream_incoming_bandwidth
                .with_label_values(lbs)
                .set(stream.bw_in as i64);
            // outgoing bandwidth
            ctx.nginx_rtmp_stream_outgoing_bandwidth
                .with_label_values(lbs)
                .set(stream.bw_out as i64);
            // video bandwidth
            ctx.nginx_rtmp_stream_bandwidth_video
                .with_label_values(lbs)
                .set(stream.bw_video as i64);
            // audio bandwidth
            ctx.nginx_rtmp_stream_bandwidth_audio
                .with_label_values(lbs)
                .set(stream.bw_audio as i64);
            // avsync
            // if this stream includes audio, set avsync
            if stream.bw_audio != 0 {
                match stream.clients.iter().find(|client| client.publishing.is_some()) {
                    Some(client) => {
                        ctx.nginx_rtmp_stream_publisher_avsync
                            .with_label_values(lbs)
                            .set(client.avsync);
                    }
                    None => (),
                };
            }
            // connected clients
            // total clients - 1, 1 publisher
            ctx.nginx_rtmp_stream_total_clients
                .with_label_values(lbs)
                .set((stream.clients.len() - 1) as i64);
        })
    });

    Ok(())
}
