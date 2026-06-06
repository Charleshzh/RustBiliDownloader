//! 下载进度模型.

/// 下载进度.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// 任务 ID.
    pub task_id: String,
    /// 已下载字节数.
    pub downloaded: u64,
    /// 总字节数.
    pub total: u64,
    /// 当前速度 (B/s).
    pub speed_bps: f64,
    /// 剩余秒数.
    pub eta_secs: f64,
}

impl DownloadProgress {
    /// 百分比.
    #[must_use]
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            // percentage display: precision loss is acceptable
            #[allow(clippy::cast_precision_loss)]
            {
                (self.downloaded as f64 / self.total as f64) * 100.0
            }
        }
    }

    /// 可读格式.
    #[must_use]
    pub fn format(&self) -> String {
        format!(
            "{:.2} MB / {:.2} MB ({:.0}%) {:.1} MB/s ETA {}",
            bytes_to_mb(self.downloaded),
            bytes_to_mb(self.total),
            self.percent(),
            self.speed_bps / 1024.0 / 1024.0,
            format_eta(self.eta_secs),
        )
    }
}

/// 字节转 MB, 精度损失可接受 (仅用于显示).
#[allow(clippy::cast_precision_loss)]
fn bytes_to_mb(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

/// 格式化 ETA, f64→u64: max(0) 移除负数, round 保证安全.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn format_eta(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let minutes = total / 60;
    let secs = total % 60;
    format!("{minutes}:{secs:02}")
}

#[cfg(test)]
mod tests {
    use super::DownloadProgress;

    #[test]
    fn test_download_progress_percent() {
        let progress = DownloadProgress {
            task_id: "task".to_string(),
            downloaded: 50,
            total: 200,
            speed_bps: 0.0,
            eta_secs: 0.0,
        };
        assert_eq!(progress.percent(), 25.0);
    }

    #[test]
    fn test_download_progress_format() {
        let progress = DownloadProgress {
            task_id: "task".to_string(),
            downloaded: 12 * 1024 * 1024,
            total: 100 * 1024 * 1024,
            speed_bps: 5.6 * 1024.0 * 1024.0,
            eta_secs: 16.0,
        };
        let text = progress.format();
        assert!(text.contains("MB"));
        assert!(text.contains("%"));
    }
}
