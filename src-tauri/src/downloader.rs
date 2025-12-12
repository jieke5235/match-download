use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, Semaphore};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DownloadItem {
    pub id: String,
    pub batch_id: Option<String>,
    pub url: String,
    pub filename: String,
    pub save_path: String,
}

#[derive(Clone, Serialize, Debug)]
pub struct DownloadProgress {
    pub id: String,
    pub batch_id: Option<String>,
    pub total: u64,
    pub current: u64,
    pub status: String, // "pending", "downloading", "paused", "completed", "error"
}

#[derive(Clone, Debug, PartialEq)]
pub enum DownloadManagerState {
    Idle,
    Running,
    Paused,
    Stopped,
}

pub struct DownloadManager {
    queue: Arc<Mutex<Vec<DownloadItem>>>,
    semaphore: Arc<Semaphore>,
    concurrency: usize,
    state: Arc<Mutex<DownloadManagerState>>,
    active_tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    progress_map: Arc<Mutex<HashMap<String, u64>>>, // 保存每个文件的已下载字节数
}

impl DownloadManager {
    pub fn new(concurrency: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(concurrency)),
            concurrency,
            state: Arc::new(Mutex::new(DownloadManagerState::Idle)),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            progress_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_concurrency(&self) -> usize {
        self.concurrency
    }

    pub async fn get_state(&self) -> DownloadManagerState {
        self.state.lock().await.clone()
    }

    pub async fn add_task(&self, item: DownloadItem) {
        self.queue.lock().await.push(item);
    }

    pub async fn pause(&self) {
        *self.state.lock().await = DownloadManagerState::Paused;
        println!("Download manager paused");
    }

    pub async fn resume(&self, app: &AppHandle) {
        let current_state = self.state.lock().await.clone();
        if current_state == DownloadManagerState::Paused {
            *self.state.lock().await = DownloadManagerState::Running;
            println!("Download manager resumed");

            // 重新启动下载 - 使用克隆的引用
            let queue = self.queue.clone();
            let semaphore = self.semaphore.clone();
            let state = self.state.clone();
            let active_tasks = self.active_tasks.clone();
            let progress_map = self.progress_map.clone();
            let app = app.clone();

            tokio::spawn(async move {
                Self::run_download_loop(queue, semaphore, state, active_tasks, progress_map, app)
                    .await;
            });
        }
    }

    pub async fn stop(&self) {
        *self.state.lock().await = DownloadManagerState::Stopped;

        // 取消所有活动任务
        let mut tasks = self.active_tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        // 清空队列
        self.queue.lock().await.clear();

        println!("Download manager stopped");
    }

    // 静态方法，不持有 self，内部按需获取锁
    async fn run_download_loop(
        queue: Arc<Mutex<Vec<DownloadItem>>>,
        semaphore: Arc<Semaphore>,
        state: Arc<Mutex<DownloadManagerState>>,
        active_tasks: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
        progress_map: Arc<Mutex<HashMap<String, u64>>>,
        app: AppHandle,
    ) {
        let client = create_http_client();

        loop {
            // 每次循环都重新检查状态（不持有锁）
            {
                let current_state = state.lock().await.clone();
                if current_state != DownloadManagerState::Running {
                    break;
                }
            } // 锁在这里释放

            // 从队列取任务（短暂持有锁）
            let item = {
                let mut q = queue.lock().await;
                q.pop()
            }; // 锁在这里释放

            if let Some(item) = item {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let app_handle = app.clone();
                let client = client.clone();
                let state_clone = state.clone();
                let active_tasks_clone = active_tasks.clone();
                let progress_map_clone = progress_map.clone();
                let item_id = item.id.clone();

                let handle = tokio::spawn(async move {
                    let _permit = permit;

                    // 检查是否应该继续
                    if *state_clone.lock().await != DownloadManagerState::Running {
                        return;
                    }

                    if let Err(e) = download_file_with_resume(
                        &client,
                        &app_handle,
                        item.clone(),
                        state_clone.clone(),
                        progress_map_clone.clone(),
                    )
                    .await
                    {
                        eprintln!("Download failed: {}", e);
                        let _ = app_handle.emit(
                            "download://progress",
                            DownloadProgress {
                                id: item.id,
                                batch_id: item.batch_id,
                                total: 0,
                                current: 0,
                                status: "error".to_string(),
                            },
                        );
                    }
                });

                active_tasks_clone
                    .lock()
                    .await
                    .insert(item_id.clone(), handle);
            } else {
                break;
            }
        }
    }

    pub fn start_downloads(&self, app: &AppHandle) {
        // 设置状态
        let state = self.state.clone();
        tokio::spawn(async move {
            *state.lock().await = DownloadManagerState::Running;
        });

        // 启动下载循环（不持有任何锁）
        let queue = self.queue.clone();
        let semaphore = self.semaphore.clone();
        let state = self.state.clone();
        let active_tasks = self.active_tasks.clone();
        let progress_map = self.progress_map.clone();
        let app = app.clone();

        tokio::spawn(async move {
            Self::run_download_loop(queue, semaphore, state, active_tasks, progress_map, app).await;
        });
    }
}

fn create_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap()
}

async fn download_file_with_resume(
    client: &reqwest::Client,
    app: &AppHandle,
    item: DownloadItem,
    state: Arc<Mutex<DownloadManagerState>>,
    progress_map: Arc<Mutex<HashMap<String, u64>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        // 检查状态
        if *state.lock().await != DownloadManagerState::Running {
            return Ok(());
        }

        match download_file_attempt_resumable(
            client,
            app,
            &item,
            attempt,
            state.clone(),
            progress_map.clone(),
        )
        .await
        {
            Ok(_) => {
                // 清除进度记录
                progress_map.lock().await.remove(&item.id);
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "Download attempt {}/{} failed for {}: {}",
                    attempt, MAX_RETRIES, item.filename, e
                );
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    let wait_time = Duration::from_secs(2u64.pow(attempt - 1));
                    eprintln!("Retrying in {:?}...", wait_time);
                    tokio::time::sleep(wait_time).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

async fn download_file_attempt_resumable(
    client: &reqwest::Client,
    app: &AppHandle,
    item: &DownloadItem,
    attempt: u32,
    state: Arc<Mutex<DownloadManagerState>>,
    progress_map: Arc<Mutex<HashMap<String, u64>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = std::path::Path::new(&item.save_path).join(&item.filename);

    // 获取已下载的字节数
    let downloaded = progress_map
        .lock()
        .await
        .get(&item.id)
        .copied()
        .unwrap_or(0);

    println!(
        "Downloading: {} (attempt {}, resume from {})",
        item.filename, attempt, downloaded
    );

    // 创建目录
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 构建请求，支持断点续传
    let mut request = client.get(&item.url);
    if downloaded > 0 {
        request = request.header("Range", format!("bytes={}-", downloaded));
    }

    let res = request
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !res.status().is_success() && res.status().as_u16() != 206 {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let total_size = if res.status().as_u16() == 206 {
        // 部分内容响应
        downloaded + res.content_length().unwrap_or(0)
    } else {
        res.content_length().unwrap_or(0)
    };

    // 通知开始
    app.emit(
        "download://progress",
        DownloadProgress {
            id: item.id.clone(),
            batch_id: item.batch_id.clone(),
            total: total_size,
            current: downloaded,
            status: "downloading".to_string(),
        },
    )?;

    let mut stream = res.bytes_stream();

    // 打开文件（追加模式如果是续传）
    let file = if downloaded > 0 {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .open(&path)?;
        f.seek(SeekFrom::End(0))?;
        f
    } else {
        std::fs::File::create(&path)?
    };

    let mut writer = BufWriter::with_capacity(8 * 1024 * 1024, file);
    let mut current = downloaded;

    let mut last_progress_update = 0u64;
    const PROGRESS_UPDATE_THRESHOLD: u64 = 1024 * 1024;

    while let Some(chunk_result) = stream.next().await {
        // 检查暂停状态
        let current_state = state.lock().await.clone();
        if current_state == DownloadManagerState::Paused {
            // 保存当前进度
            progress_map.lock().await.insert(item.id.clone(), current);

            app.emit(
                "download://progress",
                DownloadProgress {
                    id: item.id.clone(),
                    batch_id: item.batch_id.clone(),
                    total: total_size,
                    current,
                    status: "paused".to_string(),
                },
            )?;

            return Ok(());
        } else if current_state == DownloadManagerState::Stopped {
            return Ok(());
        }

        let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;

        writer
            .write_all(&chunk)
            .map_err(|e| format!("Failed to write to file: {}", e))?;
        current += chunk.len() as u64;

        // 更新进度映射
        if current - last_progress_update >= PROGRESS_UPDATE_THRESHOLD {
            progress_map.lock().await.insert(item.id.clone(), current);
        }

        if current - last_progress_update >= PROGRESS_UPDATE_THRESHOLD || current == total_size {
            app.emit(
                "download://progress",
                DownloadProgress {
                    id: item.id.clone(),
                    batch_id: item.batch_id.clone(),
                    total: total_size,
                    current,
                    status: "downloading".to_string(),
                },
            )?;
            last_progress_update = current;
        }
    }

    writer
        .flush()
        .map_err(|e| format!("Failed to flush file: {}", e))?;

    println!("Successfully downloaded: {}", item.filename);

    app.emit(
        "download://progress",
        DownloadProgress {
            id: item.id.clone(),
            batch_id: item.batch_id.clone(),
            total: total_size,
            current: total_size,
            status: "completed".to_string(),
        },
    )?;

    Ok(())
}
