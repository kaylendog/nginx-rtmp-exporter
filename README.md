# nginx-rtmp-exporter [![Deploy](https://github.com/kaylendog/nginx-rtmp-exporter/workflows/Deploy/badge.svg)](https://github.com/kaylendog/nginx-rtmp-exporter/actions?query=workflow%3ADeploy)

Prometheus instrumentation service for the NGINX RTMP module.

## Usage

```
    nginx-rtmp-exporter [OPTIONS] --scrape-url <SCRAPE_URL>

OPTIONS:
        --format <FORMAT>            An optional format for the metadata file [default: json]
    -h, --help                       Print help information
        --host <HOST>                The host to listen on [default: 127.0.0.1]
        --metadata <METADATA>        An optional path to a metadata file
    -p, --port <PORT>                The port to listen on [default: 9114]
        --scrape-url <SCRAPE_URL>    The RTMP statistics endpoint of NGINX
```

## Metrics

The exporter provides the following metrics:

-   `nginx_build_info` - The build information of NGINX, including version, RTMP module version, and compiler.
-   `nginx_rtmp_application_count` - The total number of active applications, as defined in the NGINX `rtmp {}` block.
-   `nginx_rtmp_active_streams` - The total number of active live streams currently being processed by the RTMP server.
-   `nginx_rtmp_incoming_bytes_total` - The total number of incoming bytes processed by the RTMP server since it was started.
-   `nginx_rtmp_outgoing_bytes_total` - The total number of outgoing bytes processed by the RTMP server since it was started.
-   `nginx_rtmp_incoming_bandwidth` - The incoming bandwidth of the RTMP server, in bytes per second.
-   `nginx_rtmp_outgoing_bandwidth` - The outgoing bandwidth of the RTMP server, in bytes per second.
-   `nginx_rtmp_stream_incoming_bytes_total` - The total number of incoming bytes processed by the RTMP server, labelled by stream.
-   `nginx_rtmp_stream_outgoing_bytes_total` - The total number of outgoing bytes processed by the RTMP server, labelled by stream.
-   `nginx_rtmp_stream_incoming_bandwidth` - The incoming bandwidth of the RTMP server, in bytes per second, labelled by stream.
-   `nginx_rtmp_stream_outgoing_bandwidth` - The outgoing bandwidth of the RTMP server, in bytes per second, labelled by stream.
-   `nginx_rtmp_stream_bandwidth_video` - The incoming video bandwidth of the RTMP server, in bytes per second, labelled by stream.
-   `nginx_rtmp_stream_bandwidth_audio` - The incoming audio bandwidth of the RTMP server, in bytes per second, labelled by stream.
-   `nginx_rtmp_stream_publisher_avsync` - The AV-sync value if audio data is present, labelled by stream.
-   `nginx_rtmp_stream_total_clients` - The total connected clients to the RTMP server, labelled by stream.

By default, all bandwidth measurements are taken over a period of 10 seconds. This is done internally by NGINX and cannot be configured by the exporter.

## Metadata

The exporter also supports supplying metadata to streams. Using the `--metadata` flag, a metadata file can be parsed to the exporter, in the following format:

### As JSON

```json
{
	"fields": ["<field>"],
	"metadata": {
		"<stream>": {
			"<field>": "<value>"
		}
	}
}
```

### As TOML

```toml
fields = ["<field>"]

[metadata.<stream>]
field = "<value>"
```

Any metadata provided for each stream is passed through to Prometheus as labels.

## License

This project is licensed under the GNU General Public License v3.0. See the [LICENSE](./LICENSE) file for more information.
