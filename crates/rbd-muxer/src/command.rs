//! ffmpeg command builder.

use std::path::Path;

/// Fluent builder for ffmpeg commands (alternative to raw arg lists).
#[derive(Debug, Clone, Default)]
pub struct FfmpegCommand {
    args: Vec<String>,
}

impl FfmpegCommand {
    /// 
    #[must_use]
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    /// 
    #[must_use]
    pub fn input(mut self, path: &Path) -> Self {
        self.args.push("-i".to_string());
        self.args.push(path.to_string_lossy().to_string());
        self
    }

    /// 
    #[must_use]
    pub fn output(mut self, path: &Path) -> Self {
        self.args.push(path.to_string_lossy().to_string());
        self
    }

    /// -c copy
    #[must_use]
    pub fn codec_copy(mut self) -> Self {
        self.args.push("-c".to_string());
        self.args.push("copy".to_string());
        self
    }

    /// Ƶ
    #[must_use]
    pub fn vcodec(mut self, codec: &str) -> Self {
        self.args.push("-c:v".to_string());
        self.args.push(codec.to_string());
        self
    }

    /// Ƶ
    #[must_use]
    pub fn acodec(mut self, codec: &str) -> Self {
        self.args.push("-c:a".to_string());
        self.args.push(codec.to_string());
        self
    }

    /// title
    #[must_use]
    pub fn metadata_title(mut self, t: &str) -> Self {
        self.args.push("-metadata".to_string());
        self.args.push(format!("title={}", t));
        self
    }

    /// artist
    #[must_use]
    pub fn metadata_artist(mut self, a: &str) -> Self {
        self.args.push("-metadata".to_string());
        self.args.push(format!("artist={}", a));
        self
    }

    /// cover
    #[must_use]
    pub fn cover(mut self, path: &Path) -> Self {
        self.args.push("-i".to_string());
        self.args.push(path.to_string_lossy().to_string());
        self.args.push("-map".to_string());
        self.args.push("0".to_string());
        self.args.push("-map".to_string());
        self.args.push("1".to_string());
        self.args.push("-c".to_string());
        self.args.push("copy".to_string());
        self.args.push("-disposition:v:1".to_string());
        self.args.push("attached_pic".to_string());
        self
    }

    /// 
    #[must_use]
    pub fn extra(mut self, args: &[&str]) -> Self {
        for &arg in args {
            self.args.push(arg.to_string());
        }
        self
    }

    /// 
    #[must_use]
    pub fn build(self) -> Vec<String> {
        self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_ffmpeg_command_build() {
        let cmd = FfmpegCommand::new()
            .input(&PathBuf::from("in.mp4"))
            .output(&PathBuf::from("out.mp4"))
            .codec_copy()
            .build();
        assert_eq!(cmd.len(), 5);
        assert_eq!(cmd[0], "-i");
        assert_eq!(cmd[1], "in.mp4");
        assert_eq!(cmd[2], "out.mp4"); // wait, output is usually at the end, but in this builder order matters
    }

    #[test]
    fn test_ffmpeg_command_with_metadata() {
        let cmd = FfmpegCommand::new()
            .metadata_title("Test")
            .metadata_artist("Artist")
            .cover(&PathBuf::from("cover.png"))
            .build();
        assert!(cmd.contains(&"-metadata".to_string()));
        assert!(cmd.contains(&"title=Test".to_string()));
        assert!(cmd.contains(&"artist=Artist".to_string()));
    }
}
