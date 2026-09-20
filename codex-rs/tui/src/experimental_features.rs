//! Bounded experimental-feature discovery and persistence through existing RPCs.
//! Dropping a popup stops discovery, but submitted writes finish independently.
//! Readback describes configured enablement; reserved IDs bound unanswered requests.

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::ConfigBatchWriteParams;
use codex_app_server_protocol::ConfigWriteResponse;
use codex_app_server_protocol::ExperimentalFeature;
use codex_app_server_protocol::ExperimentalFeatureListParams;
use codex_app_server_protocol::ExperimentalFeatureListResponse;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::WriteStatus;
use codex_protocol::ThreadId;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::oneshot;

pub(crate) fn fetch(
    request_handle: AppServerRequestHandle,
    thread_id: Option<ThreadId>,
    request_id: &'static str,
    mut response_tx: oneshot::Sender<Result<Vec<ExperimentalFeature>, String>>,
) {
    tokio::spawn(async move {
        let discovery = async {
            let mut features = Vec::new();
            let mut cursor = None;
            let mut cursors = HashSet::new();
            let mut names = HashSet::new();
            for _ in 0..10 {
                let response = request_handle
                    .request_typed::<ExperimentalFeatureListResponse>(
                        ClientRequest::ExperimentalFeatureList {
                            // The client rejects duplicate pending IDs, bounding unanswered
                            // requests across popup cancellation and timeout/retry cycles.
                            request_id: RequestId::String(request_id.to_string()),
                            params: ExperimentalFeatureListParams {
                                cursor,
                                limit: Some(100),
                                thread_id: thread_id.map(|id| id.to_string()),
                            },
                        },
                    )
                    .await
                    .map_err(|_| "实验功能请求失败".to_string())?;
                if response.data.len() > 100 {
                    return Err("实验功能页面超出请求限制".to_string());
                }
                features.extend(
                    response
                        .data
                        .into_iter()
                        .filter(|feature| names.insert(feature.name.clone())),
                );
                cursor = response.next_cursor;
                let Some(next) = cursor.as_ref() else {
                    return Ok(features);
                };
                if !cursors.insert(next.clone()) {
                    return Err("实验功能分页重复使用了游标".to_string());
                }
            }
            Err("实验功能发现超过 10 页".to_string())
        };
        tokio::select! {
            _ = response_tx.closed() => {},
            result = tokio::time::timeout(Duration::from_secs(/*secs*/ 5), discovery) => {
                let result = result.unwrap_or_else(|_| Err("实验功能发现超时".to_string()));
                let _ = response_tx.send(result);
            }
        }
    });
}

/// Configured readback, deliberately separate from running-task feature state.
#[derive(Debug)]
pub(crate) struct FeatureWriteResult {
    pub features: Vec<ExperimentalFeature>,
    pub warning: Option<String>,
}

pub(crate) async fn write(
    request_handle: AppServerRequestHandle,
    thread_id: ThreadId,
    updates: Vec<(String, bool)>,
) -> Result<FeatureWriteResult, String> {
    let (tx, rx) = oneshot::channel();
    fetch(
        request_handle.clone(),
        Some(thread_id),
        "tui-experimental-save-readback",
        tx,
    );
    let features = rx.await.map_err(|_| "功能发现已中断")??;
    let edits = updates
        .iter()
        .map(|(name, enabled)| {
            let feature = features
                .iter()
                .find(|feature| feature.name == *name)
                .ok_or_else(|| format!("服务器未公布实验功能 `{name}`"))?;
            // Quote the server's key as a single TOML path segment.
            let key = format!("features.{}", serde_json::json!(name));
            Ok(crate::config_update::replace_config_value(
                key,
                // Keep the daemon opt-out explicit even before rollout defaults enable it.
                if *enabled || feature.default_enabled || name == "daemon_auto_start" {
                    serde_json::json!(enabled)
                } else {
                    serde_json::Value::Null
                },
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let response = tokio::time::timeout(
        Duration::from_secs(/*secs*/ 15),
        request_handle.request_typed::<ConfigWriteResponse>(ClientRequest::ConfigBatchWrite {
            // A timed-out write may still finish. Bound unanswered retries too.
            request_id: RequestId::String("tui-experimental-feature-write".to_string()),
            params: ConfigBatchWriteParams {
                edits,
                file_path: None,
                expected_version: None,
                reload_user_config: true,
            },
        }),
    )
    .await
    .map_err(|_| "保存实验功能超时；写入可能仍会完成。请重新打开 /experimental 检查。")?
    .map_err(|_| "保存实验功能失败。请重新打开 /experimental 检查配置值后再重试。")?;
    let (tx, rx) = oneshot::channel();
    fetch(
        request_handle,
        Some(thread_id),
        "tui-experimental-save-readback",
        tx,
    );
    let features = rx
        .await
        .map_err(|_| "功能已保存，但回读被中断")?
        .map_err(|error| format!("功能已保存，但无法刷新配置值：{error}"))?;
    let overridden = response.status == WriteStatus::OkOverridden
        || updates.iter().any(|(name, enabled)| {
            !features
                .iter()
                .any(|feature| feature.name == *name && feature.enabled == *enabled)
        });
    Ok(FeatureWriteResult {
        features,
        warning: overridden.then(|| {
            "更改已保存，但配置值与你的选择不同。更高优先级的设置可能覆盖了这些值。".to_string()
        }),
    })
}

#[cfg(test)]
#[path = "experimental_features_tests.rs"]
mod tests;
