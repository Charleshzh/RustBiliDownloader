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
/// 使用 prost 解析 B 站二进制弹幕格式 (DmSegMobileReply).
///
/// proto 定义来自 `bilibili.community.service.dm.v1`, 此处手动构造 prost 类型
/// 以避免 `protoc` 编译依赖. 消息结构与官方 proto 定义完全一致.
///
/// **模式映射** (B 站 protobuf 与 XML 弹幕模式一致):
/// - 1 = 滚动, 4 = 底部, 5 = 顶部, 6 = 逆向, 7 = 精确控制, 8 = 高级
pub mod protobuf {
    use super::*;
    use prost::Message;

    /// B 站 protobuf 弹幕响应.
    ///
    /// 对应 proto: `bilibili.community.service.dm.v1.DmSegMobileReply`
    #[derive(Clone, PartialEq, Message)]
    pub struct DmSegMobileReply {
        /// 弹幕列表.
        #[prost(message, repeated, tag = "1")]
        pub elems: Vec<DanmakuElem>,
    }

    /// B 站单条弹幕 protobuf 元素.
    ///
    /// 对应 proto: `bilibili.community.service.dm.v1.DanmakuElem`
    #[derive(Clone, PartialEq, Message)]
    pub struct DanmakuElem {
        /// 弹幕 ID.
        #[prost(int64, tag = "1")]
        pub id: i64,
        /// 出现时间 (毫秒).
        #[prost(int32, tag = "2")]
        pub progress: i32,
        /// 弹幕模式.
        #[prost(int32, tag = "3")]
        pub mode: i32,
        /// 字号.
        #[prost(int32, tag = "4")]
        pub fontsize: i32,
        /// 十进制 RGB 颜色.
        #[prost(uint64, tag = "5")]
        pub color: u64,
        /// 发送者 mid 哈希.
        #[prost(string, tag = "6")]
        pub mid_hash: String,
        /// 弹幕内容.
        #[prost(string, tag = "7")]
        pub content: String,
        /// 发送时间戳.
        #[prost(int64, tag = "8")]
        pub ctime: i64,
        /// 权重.
        #[prost(int32, tag = "9")]
        pub weight: i32,
        /// 高级弹幕 action 数据.
        #[prost(string, tag = "10")]
        pub action: String,
        /// 弹幕池.
        #[prost(int32, tag = "11")]
        pub pool: i32,
        /// 高级弹幕用 id (字符串形式).
        #[prost(uint32, tag = "12")]
        pub id_str: u32,
    }

    /// 解析 Protobuf 二进制弹幕.
    ///
    /// 输入为 B 站 `/x/v2/dm/web/seg.so` API 返回的原始字节流.
    /// 使用 prost 解码为 `DmSegMobileReply`, 然后转换为 `Danmaku`.
    ///
    /// # 错误
    ///
    /// 如果字节不是有效的 protobuf 格式, 返回解码错误.
    pub fn parse(bytes: &[u8]) -> Result<DanmakuList> {
        if bytes.is_empty() {
            return Ok(DanmakuList::new());
        }

        let reply = DmSegMobileReply::decode(bytes)?;
        let mut list = DanmakuList::with_capacity(reply.elems.len());

        for elem in reply.elems {
            // progress 单位是毫秒, 转换为秒
            let time = elem.progress as f32 / 1000.0;
            // mode: 与 XML 定义一致, 直接透传
            let mode = elem.mode.clamp(0, i32::from(u8::MAX)) as u8;
            let font_size = elem.fontsize.clamp(0, i32::from(u8::MAX)) as u8;
            // color: 十进制 RGB, u64 → u32 (B 站颜色始终在 0x000000..0xFFFFFF)
            let color = elem.color as u32;
            // midHash → sender_id: 尝试解析为 u64, 失败则用 0
            let sender_id = elem.mid_hash.parse::<u64>().unwrap_or(0);

            list.push(Danmaku {
                id: elem.id as u64,
                time,
                mode,
                font_size,
                color,
                sender_id,
                content: elem.content,
                page: 1,
            });
        }

        Ok(list)
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
    fn test_protobuf_parse_empty_input() {
        let list = protobuf::parse(&[]).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_protobuf_parse_encode_decode_roundtrip() {
        // 构造一条测试弹幕
        let elem = protobuf::DanmakuElem {
            id: 12345,
            progress: 1500, // 1.5 秒 = 1500 毫秒
            mode: 1,        // 滚动
            fontsize: 25,
            color: 0xFF0000, // 红色
            mid_hash: "100".to_string(),
            content: "测试弹幕".to_string(),
            ctime: 0,
            weight: 0,
            action: String::new(),
            pool: 0,
            id_str: 0,
        };

        let reply = protobuf::DmSegMobileReply {
            elems: vec![elem],
        };

        // 编码为 protobuf 二进制
        let bytes = prost::Message::encode_to_vec(&reply);

        // 解码回 DanmakuList
        let list = protobuf::parse(&bytes).unwrap();
        assert_eq!(list.len(), 1);

        let d = &list.comments[0];
        assert_eq!(d.id, 12345);
        assert!((d.time - 1.5).abs() < 0.01);
        assert_eq!(d.mode, 1);
        assert_eq!(d.font_size, 25);
        assert_eq!(d.color, 0xFF0000);
        assert_eq!(d.sender_id, 100);
        assert_eq!(d.content, "测试弹幕");
    }

    #[test]
    fn test_protobuf_parse_multiple_elems() {
        let reply = protobuf::DmSegMobileReply {
            elems: vec![
                protobuf::DanmakuElem {
                    id: 1,
                    progress: 1000,
                    mode: 5,
                    fontsize: 20,
                    color: 0xFFFFFF,
                    mid_hash: String::new(),
                    content: "top".into(),
                    ctime: 0,
                    weight: 0,
                    action: String::new(),
                    pool: 0,
                    id_str: 0,
                },
                protobuf::DanmakuElem {
                    id: 2,
                    progress: 2000,
                    mode: 1,
                    fontsize: 25,
                    color: 0x00FF00,
                    mid_hash: "42".into(),
                    content: "scroll".into(),
                    ctime: 0,
                    weight: 0,
                    action: String::new(),
                    pool: 0,
                    id_str: 0,
                },
            ],
        };

        let bytes = prost::Message::encode_to_vec(&reply);
        let list = protobuf::parse(&bytes).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.comments[0].mode, 5);
        assert_eq!(list.comments[0].content, "top");
        assert_eq!(list.comments[1].sender_id, 42);
        assert_eq!(list.comments[1].content, "scroll");
    }
}
