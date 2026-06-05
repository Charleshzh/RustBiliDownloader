//! 文件路径模板渲染.
//!
//! 支持变量:
//! - `{title}` 视频主标题
//! - `{bvid}` BV 号
//! - `{aid}` AV 号
//! - `{cid}` 物理分片 ID
//! - `{p_title}` 分 P 标题
//! - `{p_n}` 分 P 序号
//! - `{p_count}` 分 P 总数
//! - `{owner_name}` UP 主名
//! - `{owner_mid}` UP 主 mid
//! - `{pubdate}` 发布时间 (Unix timestamp)
//! - `{year}/{month}/{day}/{hour}/{minute}/{second}` 时间分量
//! - `{quality}` 画质代码
//! - `{codec}` 编码
//! - `{ext}` 输出扩展名
//!
//! **算法来源**: BBDown/BBDownConfigParser.cs + Yutto/path_templates.py 变量融合.

use tera::{Context, Tera};

/// 默认文件命名模板 (BBDown 默认值).
pub const DEFAULT_PATTERN: &str = "{title}/{p_title}";

/// 创建单次使用的 Tera 渲染器.
#[must_use]
pub fn new_tera(pattern: &str) -> Tera {
    let mut tera = Tera::default();
    // 模板用 {var} 格式, Tera 默认是 {{ var }} 或 {% var %}
    // 简单做法: 把 {var} 转换为 {{ var | replace(" ", " ") }}
    let tpl = pattern_to_tera(pattern);
    tera.add_raw_template("default", &tpl)
        .expect("模板解析失败");
    tera
}

/// 把 `{var}` 格式转为 tera `{{ var }}` 格式.
fn pattern_to_tera(pattern: &str) -> String {
    // 简单替换: {xxx} -> {{ xxx }}
    let mut out = String::with_capacity(pattern.len() + 32);
    let mut in_brace = false;
    for c in pattern.chars() {
        match c {
            '{' => {
                if !in_brace {
                    out.push_str("{{ ");
                    in_brace = true;
                } else {
                    out.push(c);
                }
            }
            '}' => {
                if in_brace {
                    out.push_str(" }}");
                    in_brace = false;
                } else {
                    out.push(c);
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// 渲染模板.
pub fn render(pattern: &str, ctx: &Context) -> anyhow::Result<String> {
    let tera = new_tera(pattern);
    let result = tera.render("default", ctx)?;
    Ok(result)
}

/// 渲染文件路径 (含扩展名).
pub fn render_path(pattern: &str, ctx: &Context, ext: &str) -> anyhow::Result<std::path::PathBuf> {
    let rendered = render(pattern, ctx)?;
    let mut path = std::path::PathBuf::from(rendered);
    if !ext.is_empty() && path.extension().is_none() {
        path.set_extension(ext);
    }
    Ok(path)
}

/// 用 JSON 值构建 Context.
pub fn context_from_json(value: &serde_json::Value) -> Context {
    Context::from_serialize(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn test_basic_pattern() {
        let ctx = Context::from_serialize(json!({
            "title": "测试视频",
            "bvid": "BV1xx411c7mD",
        }))
        .unwrap();
        let out = render("{title} - {bvid}.mp4", &ctx).unwrap();
        assert!(out.contains("测试视频"));
        assert!(out.contains("BV1xx411c7mD"));
        assert!(out.contains(".mp4"));
    }

    #[test]
    fn test_pattern_to_tera() {
        let tpl = pattern_to_tera("{a}/{b}/{c}");
        assert_eq!(tpl, "{{ a }}/{{ b }}/{{ c }}");
    }

    #[test]
    fn test_default_pattern() {
        let ctx = Context::from_serialize(json!({
            "title": "标题",
            "p_title": "第一集",
        }))
        .unwrap();
        let out = render(DEFAULT_PATTERN, &ctx).unwrap();
        assert!(out.contains("标题"));
        assert!(out.contains("第一集"));
    }

    #[test]
    fn test_render_path() {
        let ctx = Context::from_serialize(json!({"title": "abc"})).unwrap();
        let path = render_path("{title}/v", &ctx, "mp4").unwrap();
        assert_eq!(path, Path::new("abc/v.mp4"));
    }
}
