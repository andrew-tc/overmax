//! Recommendation Provider Fetcher — network requests for external recommendation sources.

use std::fs;
use std::path::Path;
use std::time::Duration;

use overmax_data::{RecommendContext, VaryDim};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderManifest {
    pub protocol: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub vary: Vec<VaryDim>,
    #[serde(default = "default_ttl")]
    pub ttl_sec: u64,
    #[serde(default = "default_endpoint")]
    pub endpoint: String,
}

fn default_ttl() -> u64 {
    3600
}

fn default_endpoint() -> String {
    "/recommend".to_string()
}

impl Default for ProviderManifest {
    fn default() -> Self {
        Self {
            protocol: "overmax-recommend/1".to_string(),
            name: None,
            vary: vec![VaryDim::SongId, VaryDim::Mode, VaryDim::Diff],
            ttl_sec: default_ttl(),
            endpoint: default_endpoint(),
        }
    }
}

pub fn test_provider_connection_blocking(provider_url: &str) -> Result<ProviderManifest, String> {
    let clean_url = provider_url.trim_end_matches('/');
    let manifest_url = format!("{}/manifest", clean_url);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP Client 생성 실패: {}", e))?;

    let response = client
        .get(&manifest_url)
        .send()
        .map_err(|e| format!("연결 실패: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("서버 응답 오류 (HTTP {})", response.status()));
    }

    let manifest: ProviderManifest = response
        .json()
        .map_err(|e| format!("Manifest JSON 파싱 실패: {}", e))?;

    if manifest.protocol != "overmax-recommend/1" {
        return Err(format!(
            "지원하지 않는 프로토콜 버전: {}",
            manifest.protocol
        ));
    }

    Ok(manifest)
}

pub fn fetch_manifest_blocking(provider_url: &str) -> ProviderManifest {
    test_provider_connection_blocking(provider_url).unwrap_or_default()
}

pub fn fetch_recommend_blocking(
    provider_url: &str,
    manifest: &ProviderManifest,
    ctx: &RecommendContext,
    save_path: &Path,
) -> Result<(), String> {
    let clean_url = provider_url.trim_end_matches('/');
    let endpoint = if manifest.endpoint.starts_with('/') {
        format!("{}{}", clean_url, manifest.endpoint)
    } else if manifest.endpoint.starts_with("http://") || manifest.endpoint.starts_with("https://")
    {
        manifest.endpoint.clone()
    } else {
        format!("{}/{}", clean_url, manifest.endpoint)
    };

    let mode_str = match ctx.button_mode {
        overmax_core::Mode::B4 => "4B",
        overmax_core::Mode::B5 => "5B",
        overmax_core::Mode::B6 => "6B",
        overmax_core::Mode::B8 => "8B",
    };
    let diff_str = match ctx.difficulty {
        overmax_core::Difficulty::NM => "NM",
        overmax_core::Difficulty::HD => "HD",
        overmax_core::Difficulty::MX => "MX",
        overmax_core::Difficulty::SC => "SC",
    };
    let v_id_str = ctx.v_id.as_deref().unwrap_or("");

    let full_url = format!(
        "{}?song_id={}&mode={}&diff={}&v_id={}",
        endpoint, ctx.song_id, mode_str, diff_str, v_id_str
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP Client 생성 실패: {}", e))?;

    let response = client
        .get(&full_url)
        .send()
        .map_err(|e| format!("추천 요청 실패: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("서버 응답 오류 (HTTP {})", response.status()));
    }

    let body = response
        .text()
        .map_err(|e| format!("응답 읽기 실패: {}", e))?;

    if let Some(parent) = save_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(save_path, body).map_err(|e| format!("캐시 파일 저장 실패: {}", e))?;

    Ok(())
}
