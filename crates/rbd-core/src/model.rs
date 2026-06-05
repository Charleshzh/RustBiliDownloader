//! 数据模型: VInfo / Page / Track / VideoTrack / AudioTrack / SubtitleTrack.
//!
//! **算法来源**: 综合 BBDown VInfo 结构 + Yutto types.py 字段.
//! 字段命名用 snake_case 以匹配 serde_json 直反序列化 B 站 API 响应.

use serde::{Deserialize, Serialize};

/// 视频元信息 (从 extractor 返回).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VInfo {
    /// 视频主标题
    pub title: String,
    /// 视频描述
    #[serde(default)]
    pub desc: String,
    /// 封面 URL
    #[serde(default)]
    pub pic: String,
    /// 发布时间 (Unix timestamp, seconds)
    #[serde(default)]
    pub pubdate: i64,
    /// UP 主 mid
    #[serde(default)]
    pub owner_mid: u64,
    /// UP 主名称
    #[serde(default)]
    pub owner_name: String,
    /// aid
    #[serde(default)]
    pub aid: u64,
    /// bvid
    #[serde(default)]
    pub bvid: String,
    /// 视频 cid 列表
    #[serde(default)]
    pub cids: Vec<u64>,
    /// 分 P 标题
    #[serde(default)]
    pub part_names: Vec<String>,
    /// 分 P 详情 (cid → 页)
    #[serde(default)]
    pub pages: Vec<Page>,
    /// 互动视频节点
    #[serde(default)]
    pub view_points: Vec<ViewPoint>,
    /// 番剧标记
    #[serde(default)]
    pub is_bangumi: bool,
    /// 课程标记
    #[serde(default)]
    pub is_cheese: bool,
    /// 互动视频标记
    #[serde(default)]
    pub is_stein_gate: bool,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 分 P 信息.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    /// 分 P 序号 (从 1 开始)
    pub page_index: u32,
    /// 分 P cid
    pub cid: u64,
    /// 分 P 标题
    pub title: String,
    /// 时长 (秒)
    pub duration: u32,
    /// 分辨率描述 (e.g. "1920x1080")
    #[serde(default)]
    pub dimension: String,
}

/// 互动视频节点.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewPoint {
    /// 节点 cid
    pub cid: u64,
    /// 节点标题
    pub title: String,
    /// 节点开始时间 (秒)
    pub start: u32,
    /// 节点结束时间 (秒)
    pub end: u32,
}

/// Track 是 VideoTrack / AudioTrack / SubtitleTrack 的统一外壳.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Track {
    /// 视频轨
    Video(VideoTrack),
    /// 音频轨
    Audio(AudioTrack),
    /// 字幕轨
    Subtitle(SubtitleTrack),
}

/// 视频轨.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTrack {
    /// 唯一 ID
    pub id: String,
    /// 画质代码
    pub quality: u32,
    /// 画质描述 ("1080P", "4K", "8K")
    pub quality_desc: String,
    /// 编码 (avc / hevc / av1)
    pub codec: String,
    /// 帧率 (30 / 60)
    #[serde(default)]
    pub frame_rate: f32,
    /// 分辨率
    pub resolution: String,
    /// 比特率 (bps)
    #[serde(default)]
    pub bandwidth: u64,
    /// 是否 HDR
    #[serde(default)]
    pub is_hdr: bool,
    /// 是否杜比视界
    #[serde(default)]
    pub is_dolby_vision: bool,
    /// 是否高帧率
    #[serde(default)]
    pub is_high_frame_rate: bool,
    /// 是否编码合并流 (durl/flv 格式, 音视频已合并).
    /// 为 true 时无需额外下载音频轨, 也无需混流.
    #[serde(default)]
    pub is_combined: bool,
    /// 下载 URL 列表 (主 + 备)
    pub urls: Vec<String>,
    /// 单文件大小 (字节, 0 = 未知)
    #[serde(default)]
    pub size: u64,
}

/// 音频轨.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    /// 唯一 ID
    pub id: String,
    /// 音质代码
    pub quality: u32,
    /// 音质描述 ("320kbps", "Dolby Atmos", "Hi-Res")
    pub quality_desc: String,
    /// 编码 (aac / flac / eac3)
    pub codec: String,
    /// 比特率
    #[serde(default)]
    pub bandwidth: u64,
    /// 是否杜比全景声
    #[serde(default)]
    pub is_dolby_atmos: bool,
    /// 是否 Hi-Res
    #[serde(default)]
    pub is_hi_res: bool,
    /// 下载 URL 列表
    pub urls: Vec<String>,
    /// 单文件大小
    #[serde(default)]
    pub size: u64,
}

/// 字幕轨.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    /// 唯一 ID
    pub id: String,
    /// 语言 (zh-Hans, en-US, ja-JP)
    pub lang: String,
    /// 语言名称 ("简体中文", "English")
    pub lang_name: String,
    /// 字幕格式 (srt / ass / json)
    pub format: String,
    /// 下载 URL
    pub url: String,
    /// 字幕来源 ("ai" / "human")
    #[serde(default)]
    pub source: String,
    /// 是否 AI 翻译
    #[serde(default)]
    pub is_ai: bool,
}
