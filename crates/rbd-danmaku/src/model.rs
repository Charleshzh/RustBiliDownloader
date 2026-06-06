//! 弹幕数据模型.
//!
//! 定义单条弹幕 (`Danmaku`), 弹幕集合 (`DanmakuList`), 以及弹幕模式枚举 (`DanmakuMode`).

use serde::{Deserialize, Serialize};

/// 单条弹幕.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Danmaku {
    /// 弹幕 ID (从 B 站响应获取)
    pub id: u64,
    /// 出现时间 (秒)
    pub time: f32,
    /// 模式: 1=滚动 4=底部 5=顶部 6=逆向 7=精确控制 8=高级
    pub mode: u8,
    /// 字号 (12-36, 默认 25)
    pub font_size: u8,
    /// 颜色 (RGB 十进制 0xRRGGBB)
    pub color: u32,
    /// 发送者 mid (0 = 匿名)
    pub sender_id: u64,
    /// 弹幕内容 (UTF-8 文本)
    pub content: String,
    /// 视频内分 P 序号 (1-based, 1 = 第一个分 P)
    #[serde(default = "default_page")]
    pub page: u32,
}

fn default_page() -> u32 {
    1
}

/// 弹幕集合.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DanmakuList {
    /// 弹幕列表.
    pub comments: Vec<Danmaku>,
}

impl DanmakuList {
    /// 创建空列表.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带预分配容量的列表.
    #[must_use]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            comments: Vec::with_capacity(cap),
        }
    }

    /// 添加一条弹幕.
    pub fn push(&mut self, d: Danmaku) {
        self.comments.push(d);
    }

    /// 弹幕数量.
    #[must_use]
    pub fn len(&self) -> usize {
        self.comments.len()
    }

    /// 是否为空.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    /// 按时间排序 (按 page, time, id).
    pub fn sort_by_time(&mut self) {
        self.comments.sort_by(|a, b| {
            a.page
                .cmp(&b.page)
                .then_with(|| {
                    a.time
                        .partial_cmp(&b.time)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| a.id.cmp(&b.id))
        });
    }
}

/// 弹幕模式枚举.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DanmakuMode {
    /// 1 - 滚动
    Scroll = 1,
    /// 4 - 底部
    Bottom = 4,
    /// 5 - 顶部
    Top = 5,
    /// 6 - 逆向
    Reverse = 6,
    /// 7 - 精确控制
    Precise = 7,
    /// 8 - 高级
    Special = 8,
}

impl DanmakuMode {
    /// 从 u8 转换为模式枚举. 未知值默认为 Scroll.
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            4 => Self::Bottom,
            5 => Self::Top,
            6 => Self::Reverse,
            7 => Self::Precise,
            8 => Self::Special,
            _ => Self::Scroll,
        }
    }

    /// 转换为 u8.
    #[must_use]
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_danmaku_sort_by_time() {
        let mut list = DanmakuList::new();
        list.push(Danmaku {
            id: 3,
            time: 5.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "third".into(),
            page: 1,
        });
        list.push(Danmaku {
            id: 1,
            time: 2.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "first".into(),
            page: 1,
        });
        list.push(Danmaku {
            id: 2,
            time: 2.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "second".into(),
            page: 1,
        });

        list.sort_by_time();
        assert_eq!(list.comments[0].id, 1);
        assert_eq!(list.comments[1].id, 2);
        assert_eq!(list.comments[2].id, 3);
    }

    #[test]
    fn test_mode_roundtrip() {
        let modes = [
            (DanmakuMode::Scroll, 1u8),
            (DanmakuMode::Bottom, 4),
            (DanmakuMode::Top, 5),
            (DanmakuMode::Reverse, 6),
            (DanmakuMode::Precise, 7),
            (DanmakuMode::Special, 8),
        ];
        for (mode, expected) in modes {
            assert_eq!(mode.as_u8(), expected);
            assert_eq!(DanmakuMode::from_u8(expected), mode);
        }
    }

    #[test]
    fn test_danmaku_list_push() {
        let mut list = DanmakuList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);

        list.push(Danmaku {
            id: 1,
            time: 1.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "hello".into(),
            page: 1,
        });
        assert!(!list.is_empty());
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_unknown_mode_defaults_scroll() {
        assert_eq!(DanmakuMode::from_u8(0), DanmakuMode::Scroll);
        assert_eq!(DanmakuMode::from_u8(99), DanmakuMode::Scroll);
        assert_eq!(DanmakuMode::from_u8(2), DanmakuMode::Scroll);
        assert_eq!(DanmakuMode::from_u8(3), DanmakuMode::Scroll);
    }

    #[test]
    fn test_with_capacity() {
        let list = DanmakuList::with_capacity(100);
        assert_eq!(list.len(), 0);
        assert!(list.is_empty());
    }
}
