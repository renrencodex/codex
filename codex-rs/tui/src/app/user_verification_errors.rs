//! Safe user-facing messages for typed local verification failures.

use codex_app_server_client::TypedRequestError;
use codex_app_server_protocol::UserVerificationCancellationReason;
use codex_app_server_protocol::UserVerificationErrorDetails;
use codex_app_server_protocol::UserVerificationFailureReason;
use codex_app_server_protocol::UserVerificationInvalidRequestReason;
use codex_app_server_protocol::UserVerificationUnavailableReason;

pub(super) fn verification_error_message(error: &TypedRequestError) -> &'static str {
    let TypedRequestError::Server { source, .. } = error else {
        return "无法使用本地 Codex 二进制文件完成用户验证。";
    };
    let details = source
        .data
        .clone()
        .and_then(|data| serde_json::from_value::<UserVerificationErrorDetails>(data).ok());
    match details {
        Some(UserVerificationErrorDetails::InvalidRequest {
            reason: UserVerificationInvalidRequestReason::InvalidParams,
        }) => "本地 Codex 二进制文件无法验证此请求。",
        Some(UserVerificationErrorDetails::Unavailable {
            reason: UserVerificationUnavailableReason::CredentialMissing,
        }) => "本地 Codex 二进制文件中没有可用的用户验证凭据。",
        Some(UserVerificationErrorDetails::Unavailable {
            reason: UserVerificationUnavailableReason::BiometricsUnavailable,
        }) => "此设备当前无法使用生物识别验证。",
        Some(UserVerificationErrorDetails::Unavailable {
            reason: UserVerificationUnavailableReason::ProviderUnavailable,
        }) => "此请求无法使用用户验证。",
        Some(UserVerificationErrorDetails::Cancelled {
            reason: UserVerificationCancellationReason::UserCancelled,
        }) => "用户验证已取消。",
        Some(UserVerificationErrorDetails::Cancelled {
            reason: UserVerificationCancellationReason::Interrupted,
        }) => "用户验证已中断。",
        Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::AuthenticationFailed,
        }) => "生物识别验证未成功，请求已取消。",
        Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::Timeout,
        }) => "用户验证超时，请求已取消。",
        Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::ProviderError,
        })
        | Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::ServiceError,
        })
        | None => "本地 Codex 二进制文件无法完成用户验证。",
    }
}

#[cfg(test)]
#[path = "user_verification_errors_tests.rs"]
mod tests;
