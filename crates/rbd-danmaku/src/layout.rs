//! 布局算法 — 行冲突解决.
//!
//! 使用 `rustc_hash::FxHashMap` 快速追踪每行的最后占用时间,
//! 防止弹幕重叠.

use rustc_hash::FxHashMap;

/// 行追踪器: 记录每行最后被占用的时间.
pub struct RowTracker {
    last_used: FxHashMap<u32, f32>,
}

impl RowTracker {
    /// 创建新的空追踪器.
    #[must_use]
    pub fn new() -> Self {
        Self {
            last_used: FxHashMap::default(),
        }
    }

    /// 创建带预分配容量的追踪器.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            last_used: FxHashMap::with_capacity_and_hasher(cap, Default::default()),
        }
    }

    /// 查找第一行在 `time` 时刻空闲 (last_used[row] + duration <= time).
    ///
    /// 返回行号 (0-based), 所有行均占用则返回 `None`.
    #[must_use]
    pub fn find_free_row(&self, time: f32, duration: f32, max_rows: u32) -> Option<u32> {
        for row in 0..max_rows {
            if let Some(&last) = self.last_used.get(&row) {
                if last + duration > time {
                    continue; // 仍被占用
                }
            }
            return Some(row);
        }
        None
    }

    /// 标记行被占用直到 `end_time`.
    pub fn mark_used(&mut self, row: u32, end_time: f32) {
        self.last_used.insert(row, end_time);
    }

    /// 清空所有记录.
    pub fn clear(&mut self) {
        self.last_used.clear();
    }
}

impl Default for RowTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_free_row_empty() {
        let tracker = RowTracker::new();
        assert_eq!(tracker.find_free_row(0.0, 1.0, 3), Some(0));
    }

    #[test]
    fn test_find_free_row_after_release() {
        let mut tracker = RowTracker::new();
        // 标记行 0 占用到 time 5
        tracker.mark_used(0, 5.0);
        // 在 time 6 查找: last(5) + duration(1) = 6 <= 6, 行 0 空闲
        assert_eq!(tracker.find_free_row(6.0, 1.0, 3), Some(0));
    }

    #[test]
    fn test_find_free_row_occupied() {
        let mut tracker = RowTracker::new();
        tracker.mark_used(0, 10.0);
        // time 5 时行 0 仍被占用, 应返回下一行
        let row = tracker.find_free_row(5.0, 1.0, 3);
        assert!(row == Some(1) || row == Some(2));
    }

    #[test]
    fn test_all_rows_occupied() {
        let mut tracker = RowTracker::new();
        tracker.mark_used(0, 10.0);
        tracker.mark_used(1, 10.0);
        tracker.mark_used(2, 10.0);
        assert_eq!(tracker.find_free_row(5.0, 1.0, 3), None);
    }

    #[test]
    fn test_mark_and_clear() {
        let mut tracker = RowTracker::new();
        tracker.mark_used(0, 5.0);
        assert_eq!(tracker.find_free_row(3.0, 1.0, 1), None);

        tracker.clear();
        assert_eq!(tracker.find_free_row(0.0, 1.0, 1), Some(0));
    }

    #[test]
    fn test_with_capacity() {
        let tracker = RowTracker::with_capacity(32);
        assert_eq!(tracker.find_free_row(0.0, 1.0, 1), Some(0));
    }
}
