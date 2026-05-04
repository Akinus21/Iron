use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

use futures::future::join_all;
use reqwest::Client;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch;

const CHUNK_COUNT: u64 = 8;
const MIN_PARALLEL_BYTES: u64 = 1024 * 512;

#[derive(Debug, Clone)]
pub enum DownloadError {
    Network(String),
    Io(String),
    Other(String),
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Network(s) | DownloadError::Io(s) | DownloadError::Other(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Progress {
    pub downloaded: u64,
    pub total: u64,
}

impl Progress {
    pub fn fraction(&self) -> f64 {
        if self.total == 0 { return 0.0; }
        (self.downloaded as f64 / self.total as f64).clamp(0.0, 1.0)
    }
}

pub fn start(
    url: String,
    dest: PathBuf,
    user_agent: String,
    rt: tokio::runtime::Handle,
) -> (watch::Receiver<Progress>, tokio::task::JoinHandle<Result<PathBuf, DownloadError>>) {
    let (tx, rx) = watch::channel(Progress { downloaded: 0, total: 0 });

    let handle = rt.spawn(async move {
        run(url, dest, user_agent, tx).await
    });

    (rx, handle)
}

async fn run(
    url: String,
    dest: PathBuf,
    user_agent: String,
    progress_tx: watch::Sender<Progress>,
) -> Result<PathBuf, DownloadError> {
    let client = Client::builder()
        .user_agent(&user_agent)
        .build()
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    let head = client.head(&url).send().await
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    let content_length = head.headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());

    let accept_ranges = head.headers()
        .get(reqwest::header::ACCEPT_RANGES)
        .and_then(|v| v.to_str().ok())
        .map(|v| v != "none")
        .unwrap_or(false);

    let total = content_length.unwrap_or(0);
    let _ = progress_tx.send(Progress { downloaded: 0, total });

    if accept_ranges && total >= MIN_PARALLEL_BYTES {
        download_parallel(&client, &url, &dest, total, &progress_tx).await
    } else {
        download_single(&client, &url, &dest, total, &progress_tx).await
    }
}

async fn download_parallel(
    client: &Client,
    url: &str,
    dest: &Path,
    total: u64,
    progress_tx: &watch::Sender<Progress>,
) -> Result<PathBuf, DownloadError> {
    let chunk_size = total / CHUNK_COUNT;
    let downloaded_bytes = Arc::new(AtomicU64::new(0));

    let mut ranges: Vec<(u64, u64)> = (0..CHUNK_COUNT)
        .map(|i| {
            let start = i * chunk_size;
            let end = if i == CHUNK_COUNT - 1 { total - 1 } else { start + chunk_size - 1 };
            (start, end)
        })
        .collect();

    let temp_paths: Vec<PathBuf> = (0..CHUNK_COUNT)
        .map(|i| dest.with_extension(format!("part{}", i)))
        .collect();

    let futures: Vec<_> = ranges.iter().zip(temp_paths.iter()).map(|(&(start, end), temp)| {
        let client = client.clone();
        let url = url.to_string();
        let temp = temp.clone();
        let downloaded_bytes = downloaded_bytes.clone();
        let progress_tx = progress_tx.clone();
        let total = total;

        async move {
            download_range(&client, &url, start, end, &temp, &downloaded_bytes, &progress_tx, total).await
        }
    }).collect();

    let results = join_all(futures).await;
    for r in results {
        r?;
    }

    assemble_chunks(&temp_paths, dest).await?;

    for temp in &temp_paths {
        let _ = tokio::fs::remove_file(temp).await;
    }

    let _ = progress_tx.send(Progress { downloaded: total, total });
    Ok(dest.to_path_buf())
}

async fn download_range(
    client: &Client,
    url: &str,
    start: u64,
    end: u64,
    dest: &Path,
    downloaded_bytes: &Arc<AtomicU64>,
    progress_tx: &watch::Sender<Progress>,
    total: u64,
) -> Result<(), DownloadError> {
    use futures::StreamExt;

    let range_header = format!("bytes={}-{}", start, end);
    let resp = client
        .get(url)
        .header(reqwest::header::RANGE, range_header)
        .send()
        .await
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    let mut file = File::create(dest).await
        .map_err(|e| DownloadError::Io(e.to_string()))?;

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
        file.write_all(&bytes).await
            .map_err(|e| DownloadError::Io(e.to_string()))?;
        let done = downloaded_bytes.fetch_add(bytes.len() as u64, Ordering::Relaxed) + bytes.len() as u64;
        let _ = progress_tx.send(Progress { downloaded: done, total });
    }

    file.flush().await.map_err(|e| DownloadError::Io(e.to_string()))?;
    Ok(())
}

async fn download_single(
    client: &Client,
    url: &str,
    dest: &Path,
    total: u64,
    progress_tx: &watch::Sender<Progress>,
) -> Result<PathBuf, DownloadError> {
    use futures::StreamExt;

    let resp = client.get(url).send().await
        .map_err(|e| DownloadError::Network(e.to_string()))?;

    let mut file = File::create(dest).await
        .map_err(|e| DownloadError::Io(e.to_string()))?;

    let mut downloaded = 0u64;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
        file.write_all(&bytes).await
            .map_err(|e| DownloadError::Io(e.to_string()))?;
        downloaded += bytes.len() as u64;
        let _ = progress_tx.send(Progress { downloaded, total });
    }

    file.flush().await.map_err(|e| DownloadError::Io(e.to_string()))?;
    let _ = progress_tx.send(Progress { downloaded, total: downloaded });
    Ok(dest.to_path_buf())
}

async fn assemble_chunks(parts: &[PathBuf], dest: &Path) -> Result<(), DownloadError> {
    let mut out = File::create(dest).await
        .map_err(|e| DownloadError::Io(e.to_string()))?;

    for part in parts {
        let mut f = File::open(part).await
            .map_err(|e| DownloadError::Io(e.to_string()))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).await
            .map_err(|e| DownloadError::Io(e.to_string()))?;
        out.write_all(&buf).await
            .map_err(|e| DownloadError::Io(e.to_string()))?;
    }

    out.flush().await.map_err(|e| DownloadError::Io(e.to_string()))?;
    Ok(())
}