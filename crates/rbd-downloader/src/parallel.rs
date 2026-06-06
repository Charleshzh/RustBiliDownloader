//! 并行下载器.

use std::io::{Seek, SeekFrom, Write};
use std::sync::Mutex as StdMutex;
use std::{path::PathBuf, sync::Arc, time::Instant};

use anyhow::{anyhow, Result};
use reqwest::header::HeaderMap;
use tokio::fs;

use crate::{event::DownloadEvent, range::RangeClient};

/// 下载规格.
#[derive(Debug, Clone)]
pub struct DownloadSpec {
    /// 下载地址.
    pub url: String,
    /// 请求头.
    pub headers: HeaderMap,
    /// 目标文件.
    pub dest: PathBuf,
    /// 任务 ID.
    pub task_id: String,
    /// 线程数覆盖.
    pub num_threads: usize,
    /// 分块大小覆盖.
    pub block_size: u64,
}

/// 并行 Range 下载器.
pub struct ParallelDownloader {
    range_client: RangeClient,
    num_threads: usize,
    block_size: u64,
    ratelimit: Option<Arc<rbd_foundation::ratelimit::RateLimiter>>,
}

impl ParallelDownloader {
    /// 创建下载器.
    #[must_use]
    pub fn new(range_client: RangeClient) -> Self {
        Self {
            range_client,
            num_threads: 4,
            block_size: 1024 * 1024,
            ratelimit: None,
        }
    }

    /// 设置线程数.
    #[must_use]
    pub fn with_num_threads(mut self, n: usize) -> Self {
        self.num_threads = n.max(1);
        self
    }

    /// 设置分块大小.
    #[must_use]
    pub fn with_block_size(mut self, b: u64) -> Self {
        self.block_size = b.max(1);
        self
    }

    /// 设置限流器.
    #[must_use]
    pub fn with_ratelimit(mut self, l: rbd_foundation::ratelimit::RateLimiter) -> Self {
        self.ratelimit = Some(Arc::new(l));
        self
    }

    /// 执行下载.
    pub async fn download<F: FnMut(DownloadEvent) + Send>(
        &self,
        spec: DownloadSpec,
        mut on_event: F,
    ) -> Result<PathBuf> {
        let total = self.range_client.head(&spec.url, &spec.headers).await?;
        on_event(DownloadEvent::Start {
            task_id: spec.task_id.clone(),
            total,
        });

        if let Some(parent) = spec.dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        if total == 0 {
            fs::write(&spec.dest, []).await?;
            on_event(DownloadEvent::Done {
                task_id: spec.task_id,
                path: spec.dest.clone(),
            });
            return Ok(spec.dest);
        }

        let num_threads = if spec.num_threads == 0 {
            self.num_threads
        } else {
            spec.num_threads
        };
        let block_size = if spec.block_size == 0 {
            self.block_size
        } else {
            spec.block_size
        };
        let blocks = calculate_blocks(total, block_size, num_threads);

        // 预分配文件并打开用于流式写入
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&spec.dest)?;
        file.set_len(total)?;
        let file = Arc::new(StdMutex::new(file));

        let started_at = Instant::now();
        let downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let limiter = self.ratelimit.clone();

        // 使用信号量限制并发数
        let semaphore = Arc::new(tokio::sync::Semaphore::new(num_threads.max(1)));

        let mut handles = Vec::new();
        for (start, end) in blocks {
            let client = self.range_client.clone();
            let url = spec.url.clone();
            let headers = spec.headers.clone();
            let file = Arc::clone(&file);
            let downloaded_bytes = Arc::clone(&downloaded);
            let limiter = limiter.clone();
            let semaphore = Arc::clone(&semaphore);
            handles.push(tokio::spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .map_err(|e| anyhow!("信号量获取失败: {e}"))?;
                if let Some(limiter) = limiter.as_ref() {
                    rbd_foundation::ratelimit::tick(limiter).await;
                }
                let response = client.get_range(&url, start, end, &headers).await?;

                // 流式写入: 直接在目标偏移量写入
                {
                    let mut f = file.lock().unwrap();
                    f.seek(SeekFrom::Start(start))?;
                    f.write_all(&response.data)?;
                }

                downloaded_bytes.fetch_add(
                    response.data.len() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
                Ok::<(), anyhow::Error>(())
            }));
        }

        for handle in handles {
            handle
                .await
                .map_err(|e| anyhow!("下载任务 join 失败: {e}"))??;
            let current = downloaded.load(std::sync::atomic::Ordering::Relaxed);
            let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
            on_event(DownloadEvent::Progress {
                task_id: spec.task_id.clone(),
                downloaded: current,
                total,
                speed_bps: current as f64 / elapsed,
            });
        }

        on_event(DownloadEvent::Done {
            task_id: spec.task_id,
            path: spec.dest.clone(),
        });
        Ok(spec.dest)
    }
}

/// 根据文件大小、分块大小和线程数计算下载块.
///
/// 按 `num_threads` 数量平均分割文件.
/// 使用 ceiling division 确保块数不超过 `num_threads`.
#[must_use]
pub fn calculate_blocks(total: u64, block_size: u64, num_threads: usize) -> Vec<(u64, u64)> {
    if total == 0 {
        return Vec::new();
    }

    let n = num_threads.max(1) as u64;
    // ceiling division: 确保不产生超过 n 的块
    let chunk_size = total.div_ceil(n).max(block_size.max(1));
    let mut blocks = Vec::new();
    let mut start = 0u64;
    while start < total {
        let end = (start + chunk_size - 1).min(total - 1);
        blocks.push((start, end));
        start = end + 1;
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::{calculate_blocks, ParallelDownloader};
    use crate::range::RangeClient;

    #[test]
    fn test_parallel_downloader_config() {
        let _downloader = ParallelDownloader::new(RangeClient::new())
            .with_num_threads(8)
            .with_block_size(2048);
    }

    #[test]
    fn test_parallel_downloader_calculates_blocks() {
        let blocks = calculate_blocks(100, 10, 2);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0], (0, 49));
        assert_eq!(blocks[1], (50, 99));
    }

    #[test]
    fn test_calculate_blocks_respects_num_threads() {
        let blocks = calculate_blocks(100, 10, 8);
        assert!(blocks.len() <= 8);
        // 覆盖整个范围
        assert_eq!(blocks.first().unwrap().0, 0);
        assert_eq!(blocks.last().unwrap().1, 99);
    }

    #[test]
    fn test_calculate_blocks_single_thread() {
        let blocks = calculate_blocks(100, 10, 1);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], (0, 99));
    }

    #[test]
    fn test_calculate_blocks_empty() {
        let blocks = calculate_blocks(0, 10, 4);
        assert!(blocks.is_empty());
    }
}
