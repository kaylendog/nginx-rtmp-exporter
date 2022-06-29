use std::error::Error;

use reqwest::Url;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RtmpStats {
    pub nginx_version: String,
    pub nginx_rtmp_version: String,
    pub compiler: String,
    pub pid: u32,
    pub uptime: u32,
    pub naccepted: u32,
    pub bw_in: u64,
    pub bytes_in: u64,
    pub bw_out: u64,
    pub bytes_out: u64,
    pub server: RtmpServerBlock,
}

#[derive(Debug, Deserialize)]
pub struct RtmpServerBlock {
    #[serde(rename = "application")]
    pub applications: Vec<RtmpApplication>,
}

#[derive(Debug, Deserialize)]
pub struct RtmpApplication {
    pub name: String,
    pub live: RtmpApplicationLiveBlock,
}

#[derive(Debug, Deserialize)]
pub struct RtmpApplicationLiveBlock {
    #[serde(rename = "stream", default = "Vec::new")]
    pub streams: Vec<RtmpStream>,
}

#[derive(Debug, Deserialize)]
pub struct RtmpStream {
    pub name: String,
    pub time: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub bw_in: u64,
    pub bw_out: u64,
    pub bw_audio: u64,
    pub bw_video: u64,
    #[serde(rename = "client")]
    pub clients: Vec<RtmpStreamClient>,
    pub meta: RtmpStreamMeta,
}

#[derive(Debug, Deserialize)]
pub struct RtmpStreamClient {
    pub id: u32,
    pub address: Option<String>,
    pub time: u64,
    pub flashver: String,
    pub pageurl: Option<String>,
    pub dropped: u64,
    pub avsync: i64,
    pub timestamp: u64,
    pub publishing: Option<()>,
    pub active: Option<()>,
}

#[derive(Debug, Deserialize)]
pub struct RtmpStreamMeta {
    pub video: RtmpStreamVideoMeta,
    pub audio: RtmpStreamAudioMetaWrapper,
}

#[derive(Debug, Deserialize)]
pub struct RtmpStreamVideoMeta {
    pub width: u16,
    pub height: u64,
    pub frame_rate: f32,
    pub codec: String,
    pub profile: String,
    pub compat: u16,
    pub level: f32,
}

#[derive(Debug, Deserialize)]
pub struct RtmpStreamAudioMetaWrapper {
    pub inner: Option<RtmpStreamAudioMeta>,
}

#[derive(Debug, Deserialize)]
pub struct RtmpStreamAudioMeta {
    pub codec: String,
    pub profile: String,
    pub channels: u8,
    pub sample_rate: u32,
}

/// Fetch NGINX RTMP stats from the given URL.
pub async fn fetch_nginx_stats(url: &Url) -> Result<RtmpStats, Box<dyn Error>> {
    let res = reqwest::get(url.clone()).await?;
    let text = &res.text().await?;
    let mut de = quick_xml::de::Deserializer::from_str(text);
    serde_path_to_error::deserialize(&mut de).map_err(|err| err.into())
}

#[cfg(test)]
mod tests {
    use super::{RtmpStats, RtmpStreamAudioMetaWrapper};

    #[test]
    fn test_deserialize_nginx_stats() {
        let xml = include_str!("../test/stat_xml.xml");
        let mut de = quick_xml::de::Deserializer::from_str(xml);
        let _stats: RtmpStats = serde_path_to_error::deserialize(&mut de).unwrap();
    }

    #[test]
    fn test_deserialize_audio() {
        let audio = r#"<audio>
	<codec>AAC</codec>
	<profile>LC</profile>
	<channels>2</channels>
	<sample_rate>48000</sample_rate>
	<data_rate>312</data_rate>
</audio>"#;

        let _: RtmpStreamAudioMetaWrapper = quick_xml::de::from_str(audio).unwrap();
    }
}
