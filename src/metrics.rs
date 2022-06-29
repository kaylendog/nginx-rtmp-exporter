use std::error::Error;

use tracing::debug;

use crate::{context::Context, xml::fetch_nginx_stats};

#[tracing::instrument]
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
    // set active streams
    ctx.nginx_rtmp_active_streams.set(
        stats
            .server
            .applications
            .iter()
            .map(|app| app.live.streams.len())
            .reduce(|total, count| total + count)
            .unwrap_or(0) as i64,
    );
    // incoming bytes
    ctx.nginx_rtmp_incoming_bytes_total.reset();
    ctx.nginx_rtmp_incoming_bytes_total.inc_by(stats.bytes_in);
    // outgoing bytes
    ctx.nginx_rtmp_outgoing_bytes_total.reset();
    ctx.nginx_rtmp_outgoing_bytes_total.inc_by(stats.bytes_out);
    // incoming bandwidth
    ctx.nginx_rtmp_incoming_bandwidth.set(stats.bw_in as i64);
    // outgoing bandwidth
    ctx.nginx_rtmp_outgoing_bandwidth.set(stats.bw_out as i64);
    // iterate through streams and set stats
    stats.server.applications.iter().for_each(|application| {
        application.live.streams.iter().for_each(|stream| {
            // label values
            let lbs: &[&str] = &[&application.name, &stream.name];
            // incoming bytes
            let incoming_bytes = ctx
                .nginx_rtmp_stream_incoming_bytes_total
                .get_metric_with_label_values(lbs)
                .unwrap();
            incoming_bytes.reset();
            incoming_bytes.inc_by(stream.bytes_in);
            // outgoing bytes
            let outgoing_bytes = ctx
                .nginx_rtmp_stream_outgoing_bytes_total
                .get_metric_with_label_values(lbs)
                .unwrap();
            outgoing_bytes.reset();
            outgoing_bytes.inc_by(stream.bytes_out);
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
            if stream.bw_audio != 0 {
                ctx.nginx_rtmp_stream_bandwidth_audio
                    .with_label_values(lbs)
                    .set(stream.bw_audio as i64);
            }
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
