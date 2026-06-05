//! 弹幕格式读取.
//!
//! 支持 XML (V1/V2), JSON (Web API), Protobuf (M5 待实现).

use crate::model::{Danmaku, DanmakuList};
use anyhow::Result;

/// 解析 B 站 V1 弹幕 XML 格式.
///
/// 示例输入:
/// ```xml
/// <i>
///   <d p="time,mode,fontsize,color,date,pool,user,id">content</d>
///   ...
/// </i>
/// ```
///
/// # 错误
///
/// 返回 `Err` 仅当 XML 格式严重错误 (非 UTF-8 或标签不匹配).
/// 单条 `<d>` 解析失败会被跳过, 不影响其他弹幕.
pub fn parse_xml(xml: &str) -> Result<DanmakuList> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut list = DanmakuList::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"d" {
                    let mut p_attr = String::new();
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"p" {
                            p_attr =
                                String::from_utf8_lossy(&attr.value).into_owned();
                        }
                    }
                    // 读取文本内容
                    let content = match reader.read_text(e.name()) {
                        Ok(cow) => cow.into_owned(),
                        Err(_) => String::new(),
                    };
                    if let Some(d) = parse_d_attr(&p_attr, &content) {
                        list.push(d);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(anyhow::anyhow!("XML 解析错误: {e}"));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(list)
}

fn parse_d_attr(p: &str, content: &str) -> Option<Danmaku> {
    // p 格式: "time,mode,fontsize,color,date,pool,user,id"
    let parts: Vec<&str> = p.split(',').collect();
    if parts.len() < 8 {
        return None;
    }

    let time: f32 = parts[0].parse().ok()?;
    let mode: u8 = parts[1].parse().ok()?;
    let font_size: u8 = parts[2].parse().ok()?;
    let color: u32 = u32::from_str_radix(parts[3], 10).ok()?;
    let _date: u64 = parts[4].parse().ok()?;
    let _pool: u8 = parts[5].parse().ok()?;
    let sender_id: u64 = parts[6].parse().ok()?;
    let id: u64 = parts[7].parse().ok()?;

    Some(Danmaku {
        id,
        time,
        mode,
        font_size,
        color,
        sender_id,
        content: content.to_string(),
        page: 1,
    })
}

/// 解析 B 站 Web JSON 弹幕格式.
///
/// 格式:
/// ```json
/// {"code":0,"data":{"danmaku":[{"id":...,"time":...,"mode":...,...},...]}}
/// ```
pub fn parse_json(json_str: &str) -> Result<DanmakuList> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Response {
        code: i32,
        #[serde(default)]
        data: Option<Data>,
    }
    #[derive(Deserialize)]
    struct Data {
        #[serde(default)]
        danmaku: Vec<JsonDanmaku>,
    }
    #[derive(Deserialize)]
    struct JsonDanmaku {
        id: u64,
        #[serde(rename = "time")]
        time: f32,
        mode: u8,
        #[serde(default = "default_fs")]
        fontsize: u8,
        #[serde(default = "default_color")]
        color: u32,
        #[serde(default)]
        mid: u64,
        content: String,
    }
    fn default_fs() -> u8 {
        25
    }
    fn default_color() -> u32 {
        0xFFFFFF
    }

    let resp: Response = serde_json::from_str(json_str)?;
    if resp.code != 0 {
        return Ok(DanmakuList::new());
    }
    let mut list = DanmakuList::new();
    if let Some(d) = resp.data {
        for jd in d.danmaku {
            list.push(Danmaku {
                id: jd.id,
                time: jd.time,
                mode: jd.mode,
                font_size: jd.fontsize,
                color: jd.color,
                sender_id: jd.mid,
                content: jd.content,
                page: 1,
            });
        }
    }
    Ok(list)
}

/// Protobuf 解析模块.
///
/// TODO M5: 使用 prost 实现 `bilibili.community.service.dm.v1.DmSegMobileReply` 解析.
pub mod protobuf {
    use super::*;

    /// 解析 Protobuf 二进制弹幕 (M5 待实现).
    ///
    /// 当前返回空列表以确保编译通过.
    pub fn parse(_bytes: &[u8]) -> Result<DanmakuList> {
        // TODO M5: 实现 prost 解析
        // use prost::Message;
        // let reply = DmSegMobileReply::decode(bytes)?;
        Ok(DanmakuList::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_empty() {
        let list = parse_xml("").unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_parse_xml_simple() {
        let xml = r#"<i><d p="1.0,1,25,16777215,1234567890,0,999,1001">hello</d><d p="2.0,1,25,16777215,1234567890,0,999,1002">world</d></i>"#;
        let list = parse_xml(xml).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.comments[0].content, "hello");
        assert_eq!(list.comments[1].content, "world");
    }

    #[test]
    fn test_parse_xml_d_attr_parsing() {
        let p = "1.5,1,25,16777215,1234567890,0,987654321,12345";
        let d = parse_d_attr(p, "test").unwrap();
        assert!((d.time - 1.5).abs() < 0.01);
        assert_eq!(d.mode, 1);
        assert_eq!(d.font_size, 25);
        assert_eq!(d.color, 16777215);
        assert_eq!(d.sender_id, 987654321);
        assert_eq!(d.id, 12345);
    }

    #[test]
    fn test_parse_xml_invalid_p_attr() {
        // p 只有 3 parts, 不够 8
        assert!(parse_d_attr("1.5,1,25", "test").is_none());
    }

    #[test]
    fn test_parse_json_basic() {
        let json = r#"{"code":0,"data":{"danmaku":[{"id":1,"time":1.5,"mode":5,"fontsize":25,"color":16777215,"mid":0,"content":"top danmaku"},{"id":2,"time":3.0,"mode":1,"fontsize":25,"color":16711680,"mid":123,"content":"scroll"}]}}"#;
        let list = parse_json(json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.comments[0].content, "top danmaku");
        assert_eq!(list.comments[0].mode, 5);
        assert_eq!(list.comments[1].color, 16711680); // 0xFF0000
        assert_eq!(list.comments[1].sender_id, 123);
    }

    #[test]
    fn test_parse_json_empty_response() {
        let json = r#"{"code":1,"data":null}"#;
        let list = parse_json(json).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_protobuf_parse_returns_empty() {
        let list = protobuf::parse(&[]).unwrap();
        assert!(list.is_empty());
    }
}
