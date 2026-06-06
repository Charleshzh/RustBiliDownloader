//! 下载任务状态机.
//!
//! Pending → Running → (Done | Failed | Cancelled | Paused) 转换.
//! 支持暂停/恢复/取消操作.

/// 下载任务状态机.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    /// 等待开始.
    Pending,
    /// 正在运行.
    Running,
    /// 已暂停.
    Paused,
    /// 已取消.
    Cancelled,
    /// 已完成.
    Done,
    /// 失败.
    Failed(String),
}

impl JobState {
    /// 验证状态转换.
    ///
    /// 合法转换:
    /// - Pending → Running / Cancelled
    /// - Running → Paused / Cancelled / Done / Failed
    /// - Paused → Running / Cancelled
    ///
    /// 其他转换返回 `Err`.
    pub fn transition(&self, next: &JobState) -> Result<JobState, String> {
        match (self, next) {
            // Pending 可进入 Running / Paused 可恢复
            (JobState::Pending | JobState::Paused, JobState::Running) => Ok(JobState::Running),
            // 任何可取消状态
            (JobState::Pending | JobState::Running | JobState::Paused, JobState::Cancelled) => {
                Ok(JobState::Cancelled)
            }

            // Running 可暂停/完成/失败
            (JobState::Running, JobState::Paused) => Ok(JobState::Paused),
            (JobState::Running, JobState::Done) => Ok(JobState::Done),
            (JobState::Running, JobState::Failed(_)) => Ok(next.clone()),

            // 终态不可再转换
            (JobState::Done, _) => Err("Done is a terminal state".to_string()),
            (JobState::Cancelled, _) => Err("Cancelled is a terminal state".to_string()),
            (JobState::Failed(_), _) => Err("Failed is a terminal state".to_string()),

            // 其他非法转换
            _ => Err(format!("invalid transition: {self:?} -> {next:?}")),
        }
    }

    /// 是否为终态 (不可再转换).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobState::Done | JobState::Cancelled | JobState::Failed(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_to_running() {
        assert_eq!(
            JobState::Pending.transition(&JobState::Running).unwrap(),
            JobState::Running
        );
    }

    #[test]
    fn test_pending_to_cancelled() {
        assert_eq!(
            JobState::Pending.transition(&JobState::Cancelled).unwrap(),
            JobState::Cancelled
        );
    }

    #[test]
    fn test_running_to_paused() {
        assert_eq!(
            JobState::Running.transition(&JobState::Paused).unwrap(),
            JobState::Paused
        );
    }

    #[test]
    fn test_running_to_done() {
        assert_eq!(
            JobState::Running.transition(&JobState::Done).unwrap(),
            JobState::Done
        );
    }

    #[test]
    fn test_running_to_cancelled() {
        assert_eq!(
            JobState::Running.transition(&JobState::Cancelled).unwrap(),
            JobState::Cancelled
        );
    }

    #[test]
    fn test_running_to_failed() {
        let result = JobState::Running.transition(&JobState::Failed("oops".to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JobState::Failed("oops".to_string()));
    }

    #[test]
    fn test_paused_to_running() {
        assert_eq!(
            JobState::Paused.transition(&JobState::Running).unwrap(),
            JobState::Running
        );
    }

    #[test]
    fn test_paused_to_cancelled() {
        assert_eq!(
            JobState::Paused.transition(&JobState::Cancelled).unwrap(),
            JobState::Cancelled
        );
    }

    #[test]
    fn test_done_is_terminal() {
        assert!(JobState::Done.transition(&JobState::Running).is_err());
    }

    #[test]
    fn test_cancelled_is_terminal() {
        assert!(JobState::Cancelled.transition(&JobState::Running).is_err());
    }

    #[test]
    fn test_failed_is_terminal() {
        assert!(JobState::Failed("e".into())
            .transition(&JobState::Running)
            .is_err());
    }

    #[test]
    fn test_invalid_pending_to_done() {
        assert!(JobState::Pending.transition(&JobState::Done).is_err());
    }

    #[test]
    fn test_invalid_pending_to_failed() {
        assert!(JobState::Pending
            .transition(&JobState::Failed("e".into()))
            .is_err());
    }

    #[test]
    fn test_is_terminal() {
        assert!(!JobState::Pending.is_terminal());
        assert!(!JobState::Running.is_terminal());
        assert!(!JobState::Paused.is_terminal());
        assert!(JobState::Done.is_terminal());
        assert!(JobState::Cancelled.is_terminal());
        assert!(JobState::Failed("e".into()).is_terminal());
    }
}
