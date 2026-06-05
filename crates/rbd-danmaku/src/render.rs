//! 弹幕渲染 — 入口.
//!
//! 从 `DanmakuList` 生成 ASS 格式字符串, 支持模式过滤和并行批量渲染.

use crate::model::{DanmakuList, DanmakuMode};
use crate::options::RenderOptions;
use crate::writer;
use anyhow::Result;

/// 将弹幕列表渲染为 ASS 格式字符串.
///
/// # 处理流程
///
/// 1. 按启用的模式过滤
/// 2. 按屏蔽模式过滤
/// 3. 按 (page, time, id) 排序
/// 4. 生成 ASS 内容
pub fn render_to_ass(list: &DanmakuList, opts: &RenderOptions) -> Result<String> {
    // 1. 按启用的模式过滤
    let filtered: Vec<&crate::model::Danmaku> = list
        .comments
        .iter()
        .filter(|d| {
            let mode = DanmakuMode::from_u8(d.mode);
            match mode {
                DanmakuMode::Scroll => opts.show_scroll,
                DanmakuMode::Top => opts.show_top,
                DanmakuMode::Bottom => opts.show_bottom,
                _ => true, // Reverse/Precise/Special 始终显示
            }
        })
        .filter(|d| !opts.block_modes.contains(&DanmakuMode::from_u8(d.mode)))
        .collect();

    // 2. 按 (page, time, id) 排序
    let mut sorted = filtered;
    sorted.sort_by(|a, b| {
        a.page
            .cmp(&b.page)
            .then_with(|| {
                a.time
                    .partial_cmp(&b.time)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.id.cmp(&b.id))
    });

    // 3. 生成 ASS
    let mut out = writer::write_head(
        opts.video_width,
        opts.video_height,
        &opts.font_name,
        opts.font_size,
        opts.opacity,
    );
    let duration = opts.display_duration;

    for d in &sorted {
        out.push_str(&writer::write_danmaku(
            d.time,
            duration,
            d.mode,
            d.color,
            &d.content,
            0,
        ));
    }

    Ok(out)
}

/// 并行渲染多个弹幕列表.
///
/// 使用 `rayon::into_par_iter` 实现多列表并行处理.
pub fn render_batch_parallel(
    lists: &[&DanmakuList],
    opts: &RenderOptions,
) -> Vec<Result<String>> {
    use rayon::prelude::*;
    lists.par_iter().map(|l| render_to_ass(l, opts)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Danmaku;

    #[test]
    fn test_render_empty_list() {
        let list = DanmakuList::new();
        let opts = RenderOptions::default();
        let ass = render_to_ass(&list, &opts).unwrap();
        // 应只包含头部, 无 Dialogue 行
        assert!(ass.contains("[Script Info]"));
        assert!(ass.contains("[Events]"));
        assert!(!ass.contains("Dialogue:"));
    }

    #[test]
    fn test_render_filters_disabled_mode() {
        let mut list = DanmakuList::new();
        list.push(Danmaku {
            id: 1,
            time: 1.0,
            mode: 1, // scroll
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "scroll".into(),
            page: 1,
        });
        let mut opts = RenderOptions::default();
        opts.show_scroll = false;

        let ass = render_to_ass(&list, &opts).unwrap();
        assert!(!ass.contains("scroll"));
    }

    #[test]
    fn test_render_basic() {
        let mut list = DanmakuList::new();
        list.push(Danmaku {
            id: 1,
            time: 5.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "hello".into(),
            page: 1,
        });
        let opts = RenderOptions::default();
        let ass = render_to_ass(&list, &opts).unwrap();
        assert!(ass.contains("hello"));
        assert!(ass.contains("Dialogue:"));
    }

    #[test]
    fn test_render_batch_parallel() {
        let mut list1 = DanmakuList::new();
        list1.push(Danmaku {
            id: 1,
            time: 1.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "a".into(),
            page: 1,
        });
        let mut list2 = DanmakuList::new();
        list2.push(Danmaku {
            id: 2,
            time: 2.0,
            mode: 1,
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "b".into(),
            page: 1,
        });

        let opts = RenderOptions::default();
        let results = render_batch_parallel(&[&list1, &list2], &opts);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
        assert!(results[0].as_ref().unwrap().contains("a"));
        assert!(results[1].as_ref().unwrap().contains("b"));
    }

    #[test]
    fn test_render_respects_block_modes() {
        let mut list = DanmakuList::new();
        list.push(Danmaku {
            id: 1,
            time: 1.0,
            mode: 4, // bottom
            font_size: 25,
            color: 0xFFFFFF,
            sender_id: 0,
            content: "blocked".into(),
            page: 1,
        });
        let mut opts = RenderOptions::default();
        opts.block_modes = vec![DanmakuMode::Bottom];

        let ass = render_to_ass(&list, &opts).unwrap();
        assert!(!ass.contains("blocked"));
    }
}
