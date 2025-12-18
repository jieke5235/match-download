use crate::downloader::{DownloadItem, DownloadManager, DownloadProgress};
use axum::{extract::Query, response::Html, Router};
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::sync::Mutex;

// OAuth callback state
static OAUTH_STATE: once_cell::sync::Lazy<Arc<Mutex<Option<OauthResult>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Clone, Debug, PartialEq)]
enum BatchState {
    Running,
    Paused,
}

// 批次任务管理器
struct BatchControlInfo {
    senders: Vec<mpsc::Sender<BatchControl>>,
    items: Vec<DownloadItem>, // 保存下载项以便恢复
    state: BatchState,        // 批次状态
}

type BatchTasksMap = Arc<Mutex<HashMap<String, BatchControlInfo>>>;

static BATCH_TASKS: once_cell::sync::Lazy<BatchTasksMap> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Clone, Debug)]
enum BatchControl {
    Stop,
    Pause, // 新增暂停信号
}

// 无效代码块已删除

#[derive(Debug, Clone)]
struct OauthResult {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    error: Option<String>,
    state: Option<String>,
}

// Mock data structures
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct School {
    id: i32,
    school_name: String,
    domain: Option<String>,
}

#[tauri::command]
pub async fn get_schools() -> Result<Vec<School>, String> {
    println!("Invoking get_schools...");
    let client = reqwest::Client::builder()
        .user_agent("MatchDownload/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let schools_url = "https://job3.posedu.cn/school/public_api/schools";
    println!("Fetching schools from: {}", schools_url);

    let response = client.get(schools_url).send().await.map_err(|e| {
        println!("Request failed: {}", e);
        e.to_string()
    })?;

    let status = response.status();
    println!("Response status: {}", status);

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        println!("Error response body: {}", text);
        return Err(format!(
            "获取学校列表失败: Status {}, Body: {}",
            status, text
        ));
    }

    let text = response.text().await.map_err(|e| e.to_string())?;
    println!("Response body: {}", text);

    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("JSON parsing failed: {}", e))?;

    println!("Parsed JSON: {:?}", data);

    if data["code"].as_i64() != Some(0) {
        let msg = data["msg"].as_str().unwrap_or("未知错误");
        println!("API returned error: {}", msg);
        return Err(msg.to_string());
    }

    let schools: Vec<School> = serde_json::from_value(data["data"].clone())
        .map_err(|e| format!("Data deserialization failed: {}", e))?;

    println!("Successfully parsed {} schools", schools.len());
    Ok(schools)
}

#[tauri::command]
pub async fn start_oauth(app: AppHandle, domain: String) -> Result<String, String> {
    println!("=== start_oauth called with domain: {} ===", domain);

    // OAuth configuration
    let oauth_base_url = format!("https://{}.job3.posedu.cn/school/oauth/authorize", domain);
    let redirect_uri = "http://localhost:3000/callback";
    let state = uuid::Uuid::new_v4().to_string();

    println!("OAuth URL: {}", oauth_base_url);
    println!("Redirect URI: {}", redirect_uri);
    println!("State: {}", state);

    // Reset oauth state
    println!("Resetting OAuth state...");
    *OAUTH_STATE.lock().await = None;

    // Start local HTTP server (non-blocking)
    println!("Starting local callback server on port 3000...");
    tokio::spawn(async move {
        println!("Callback server task started");
        if let Err(e) = run_callback_server().await {
            eprintln!("Callback server error: {}", e);
        }
    });

    // Wait a bit for server to start
    println!("Waiting for server to start...");
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("Server should be ready");

    // Build authorization URL
    let auth_url = format!(
        "{}?redirect_uri={}&state={}",
        oauth_base_url,
        urlencoding::encode(redirect_uri),
        state
    );

    println!("Opening browser with URL: {}", auth_url);

    // Use opener plugin instead of shell
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(&auth_url, None::<&str>)
        .map_err(|e| {
            let err_msg = format!("Failed to open browser: {}", e);
            eprintln!("{}", err_msg);
            err_msg
        })?;

    println!("Browser opened, waiting for callback...");

    // Wait for callback (max 5 minutes)
    for i in 0..300 {
        if i % 10 == 0 {
            println!("Waiting for OAuth callback... {}s", i);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let result = OAUTH_STATE.lock().await.clone();
        if let Some(oauth_result) = result {
            if let Some(code) = oauth_result.code {
                println!("=== OAuth code received: {} ===", code);
                return Ok(code);
            } else if let Some(error) = oauth_result.error {
                let err_msg = format!("OAuth error: {}", error);
                eprintln!("{}", err_msg);
                return Err(err_msg);
            }
        }
    }

    eprintln!("OAuth timeout after 300 seconds");
    Err("OAuth timeout".to_string())
}

async fn run_callback_server() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating HTTP router...");
    let app = Router::new().route("/callback", axum::routing::get(handle_callback));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Attempting to bind to {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => {
            println!("Successfully bound to port 3000");
            l
        }
        Err(e) => {
            eprintln!("Failed to bind to port 3000: {}", e);
            return Err(Box::new(e));
        }
    };

    println!("Starting axum server...");
    axum::serve(listener, app).await?;
    println!("Axum server stopped");
    Ok(())
}

async fn handle_callback(Query(params): Query<CallbackParams>) -> Html<String> {
    println!("=== Callback received! ===");
    println!("Code: {:?}", params.code);
    println!("Error: {:?}", params.error);
    println!("State: {:?}", params.state);

    // Store result
    *OAUTH_STATE.lock().await = Some(OauthResult {
        code: params.code.clone(),
        error: params.error.clone(),
    });

    println!("OAuth state updated");

    // Return success page
    let html = if params.code.is_some() {
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>授权成功</title>
            <style>
                body { font-family: -apple-system, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
                .card { background: white; padding: 40px; border-radius: 12px; text-align: center; box-shadow: 0 20px 60px rgba(0,0,0,0.3); }
                h1 { color: #48bb78; margin: 0 0 16px 0; }
                p { color: #4a5568; }
            </style>
            <script>
                // 自动关闭标签页
                setTimeout(function() {
                    window.close();
                }, 1500);
            </script>
        </head>
        <body>
            <div class="card">
                <h1>✓ 授权成功</h1>
                <p>页面将自动关闭...</p>
            </div>
        </body>
        </html>
        "#
    } else {
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>授权失败</title>
            <style>
                body { font-family: -apple-system, sans-serif; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
                .card { background: white; padding: 40px; border-radius: 12px; text-align: center; box-shadow: 0 20px 60px rgba(0,0,0,0.3); }
                h1 { color: #f56565; margin: 0 0 16px 0; }
                p { color: #4a5568; }
            </style>
        </head>
        <body>
            <div class="card">
                <h1>✗ 授权失败</h1>
                <p>您可以关闭此页面</p>
            </div>
        </body>
        </html>
        "#
    };

    Html(html.to_string())
}

#[tauri::command]
pub async fn exchange_token(code: String, domain: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let token_url = format!("https://{}.job3.posedu.cn/school/oauth/token", domain);

    let params = [
        ("code", code.as_str()),
        ("grant_type", "authorization_code"),
    ];

    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {}", error_text));
    }

    let token_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(token_data)
}

#[tauri::command]
pub async fn get_user_info(
    access_token: String,
    domain: String,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let userinfo_url = format!("https://{}.job3.posedu.cn/school/oauth/userinfo", domain);

    let response = client
        .get(userinfo_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Get user info failed: {}", error_text));
    }

    let user_data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    Ok(user_data)
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Match {
    id: i32,
    title: String,
    createtime: i64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Stage {
    id: i32,
    title: String,
    sort: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Work {
    id: i32,
    title: String,
    student_id: i32,
    student_name: Option<String>,
    college_name: Option<String>,
    major_name: Option<String>,
    class_name: Option<String>,
    match_title: Option<String>,
    stage_name: Option<String>,
    check_status: i32,
    createtime: i64,
    files: Vec<WorkFile>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct UserContent {
    url: String,
    name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct WorkFile {
    id: i32,
    element_label: String,
    user_content: UserContent,
    element_type: i32,
}

#[tauri::command]
pub async fn fetch_matches(access_token: String, domain: String) -> Result<Vec<Match>, String> {
    println!("Fetching matches...");
    let client = reqwest::Client::new();
    let url = format!("https://{}.job3.posedu.cn/school/match_api/matches", domain);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err("获取比赛列表失败".to_string());
    }

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    if data["code"].as_i64() != Some(0) {
        return Err(data["msg"].as_str().unwrap_or("未知错误").to_string());
    }

    let matches: Vec<Match> =
        serde_json::from_value(data["data"].clone()).map_err(|e| e.to_string())?;

    println!("Fetched {} matches", matches.len());
    Ok(matches)
}

#[tauri::command]
pub async fn fetch_stages(
    access_token: String,
    domain: String,
    match_id: i32,
) -> Result<Vec<Stage>, String> {
    println!("Fetching stages for match {}...", match_id);
    let client = reqwest::Client::new();
    let url = format!(
        "https://{}.job3.posedu.cn/school/match_api/stages?match_id={}",
        domain, match_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err("获取赛段列表失败".to_string());
    }

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    if data["code"].as_i64() != Some(0) {
        return Err(data["msg"].as_str().unwrap_or("未知错误").to_string());
    }

    let stages: Vec<Stage> =
        serde_json::from_value(data["data"].clone()).map_err(|e| e.to_string())?;

    println!("Fetched {} stages", stages.len());
    Ok(stages)
}

#[tauri::command]
pub async fn fetch_works(
    access_token: String,
    domain: String,
    match_id: i32,
    stage_id: i32,
) -> Result<Vec<Work>, String> {
    println!(
        "Fetching works for match {} stage {}...",
        match_id, stage_id
    );
    let client = reqwest::Client::new();
    let url = format!(
        "https://{}.job3.posedu.cn/school/match_api/works?match_id={}&stage_id={}",
        domain, match_id, stage_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err("获取作品列表失败".to_string());
    }

    let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    if data["code"].as_i64() != Some(0) {
        return Err(data["msg"].as_str().unwrap_or("未知错误").to_string());
    }

    let works: Vec<Work> =
        serde_json::from_value(data["data"].clone()).map_err(|e| e.to_string())?;

    println!("Fetched {} works", works.len());
    Ok(works)
}

#[tauri::command]
pub async fn download_works(
    app: AppHandle,
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
    works: Vec<Work>,
    batch_id: Option<String>,
    save_path: String,
) -> Result<(), String> {
    let mut download_items = Vec::new();

    for work in works {
        // 构建目录路径: 比赛名称/赛段名称/学院/专业/班级/学生姓名_学号
        let match_title = work.match_title.as_deref().unwrap_or("未命名比赛");
        let stage_name = work.stage_name.as_deref().unwrap_or("未命名赛段");
        let college_name = work.college_name.as_deref().unwrap_or("未知学院");
        let major_name = work.major_name.as_deref().unwrap_or("未知专业");
        let class_name = work.class_name.as_deref().unwrap_or("未知班级");
        let student_name = work.student_name.as_deref().unwrap_or("未知学生");

        // 学生文件夹名: 姓名_学号
        let student_folder = format!("{}_{}", student_name, work.student_id);

        // 构建完整的保存路径
        let work_save_path = format!(
            "{}/{}/{}/{}/{}/{}/{}",
            save_path,
            sanitize_filename(match_title),
            sanitize_filename(stage_name),
            sanitize_filename(college_name),
            sanitize_filename(major_name),
            sanitize_filename(class_name),
            sanitize_filename(&student_folder)
        );

        // 遍历每个文件
        for file in work.files {
            let id = uuid::Uuid::new_v4().to_string();
            let filename = sanitize_filename(&file.user_content.name);

            download_items.push(DownloadItem {
                id,
                batch_id: batch_id.clone(),
                url: file.user_content.url,
                filename,
                save_path: work_save_path.clone(),
            });
        }
    }

    // 关键修复：立即注册批次，确保 Stop/Pause 按钮立即可用
    if let Some(ref bid) = batch_id {
        println!(
            "📦 Registering batch: {} (items: {})",
            bid,
            download_items.len()
        );
        BATCH_TASKS.lock().await.insert(
            bid.clone(),
            BatchControlInfo {
                senders: Vec::new(),
                items: download_items.clone(),
                state: BatchState::Running,
            },
        );
    }

    let manager = state.lock().await;
    let semaphore = manager.get_semaphore();
    let app_clone = app.clone();
    let batch_id_clone = batch_id.clone();

    // 关键修复：将整个调度逻辑放入后台任务，避免阻塞主线程
    tokio::spawn(async move {
        let mut control_senders = Vec::new();

        for item in download_items {
            // 检查批次状态：如果已暂停或删除，停止生成新任务
            if let Some(ref bid) = batch_id_clone {
                let tasks = BATCH_TASKS.lock().await;
                if let Some(info) = tasks.get(bid) {
                    if info.state == BatchState::Paused {
                        println!("⏸️ Batch is paused, stopping dispatch loop: {}", bid);
                        break;
                    }
                } else {
                    // 批次已删除
                    break;
                }
            }

            // 在这里等待信号量，不会阻塞主线程 UI
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break, // 信号量关闭
            };

            // 再次检查状态（因为等待信号量可能花了很久）
            if let Some(ref bid) = batch_id_clone {
                let mut tasks = BATCH_TASKS.lock().await;
                if let Some(info) = tasks.get_mut(bid) {
                    if info.state == BatchState::Paused {
                        // 释放信号量并退出
                        drop(permit);
                        break;
                    }

                    let app_handle = app_clone.clone();
                    let client = create_http_client();
                    let (tx, rx) = mpsc::channel(1);
                    control_senders.push(tx);

                    // 添加到 senders
                    info.senders.push(control_senders.last().unwrap().clone());

                    tokio::spawn(async move {
                        let _permit = permit; // 任务结束自动释放
                        if let Err(e) =
                            download_file_with_control(&client, &app_handle, item, rx).await
                        {
                            eprintln!("Download failed: {}", e);
                        }
                    });
                } else {
                    drop(permit);
                    break;
                }
            } else {
                // 单个文件下载（无 batch_id），保持原有逻辑
                let app_handle = app_clone.clone();
                let client = create_http_client();
                let (tx, rx) = mpsc::channel(1); // dummy channel

                tokio::spawn(async move {
                    let _permit = permit;
                    if let Err(e) = download_file_with_control(&client, &app_handle, item, rx).await
                    {
                        eprintln!("Download failed: {}", e);
                    }
                });
            }
        }
    });

    Ok(())
}

// 带控制通道的下载函数
async fn download_file_with_control(
    client: &reqwest::Client,
    app: &AppHandle,
    item: DownloadItem,
    mut control_rx: mpsc::Receiver<BatchControl>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_RETRIES: u32 = 3;
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        // 检查是否收到停止信号
        if let Ok(BatchControl::Stop) = control_rx.try_recv() {
            println!("Download stopped for: {}", item.filename);
            app.emit(
                "download://progress",
                DownloadProgress {
                    id: item.id.clone(),
                    batch_id: item.batch_id.clone(),
                    total: 0,
                    current: 0,
                    status: "stopped".to_string(),
                },
            )?;
            return Ok(());
        }

        match download_file_simple_with_control(client, app, &item, attempt, &mut control_rx).await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                eprintln!(
                    "Download attempt {}/{} failed for {}: {}",
                    attempt, MAX_RETRIES, item.filename, e
                );
                last_error = Some(e);

                if attempt < MAX_RETRIES {
                    let wait_time = Duration::from_secs(2u64.pow(attempt - 1));
                    tokio::time::sleep(wait_time).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

async fn download_file_simple_with_control(
    client: &reqwest::Client,
    app: &AppHandle,
    item: &DownloadItem,
    attempt: u32,
    control_rx: &mut mpsc::Receiver<BatchControl>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::io::{Seek, SeekFrom, Write};

    let path = std::path::Path::new(&item.save_path).join(&item.filename);

    println!("Downloading: {} (attempt {})", item.filename, attempt);

    // 创建目录
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 检查已存在文件的大小（断点续传）
    let mut downloaded_size = 0u64;
    let mut file_exists = false;
    if path.exists() {
        if let Ok(metadata) = std::fs::metadata(&path) {
            downloaded_size = metadata.len();
            file_exists = true;
        }
    }

    // 发送请求，带上前 Range
    let mut req_builder = client.get(&item.url);
    if downloaded_size > 0 {
        println!(
            "Resuming download for {}: bytes={}-",
            item.filename, downloaded_size
        );
        req_builder = req_builder.header("Range", format!("bytes={}-", downloaded_size));
    }

    let res = req_builder
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    // 处理 416 Range Not Satisfiable (说明文件可能已下载完)
    if res.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        // 假设文件已完整，标记为完成
        // 更好的做法是检查 Content-Length，但 416 通常意味着 offset >= total
        app.emit(
            "download://progress",
            DownloadProgress {
                id: item.id.clone(),
                batch_id: item.batch_id.clone(),
                total: downloaded_size,
                current: downloaded_size,
                status: "completed".to_string(),
            },
        )?;
        return Ok(());
    }

    if !res.status().is_success() {
        return Err(format!("HTTP error: {}", res.status()).into());
    }

    let content_length = res.content_length().unwrap_or(0);
    let total_size = downloaded_size + content_length;

    // 通知开始
    app.emit(
        "download://progress",
        DownloadProgress {
            id: item.id.clone(),
            batch_id: item.batch_id.clone(),
            total: total_size,
            current: downloaded_size,
            status: "downloading".to_string(),
        },
    )?;

    // 打开文件：如果已存在则追加，否则创建
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&path)?;

    let mut writer = std::io::BufWriter::with_capacity(8 * 1024 * 1024, file);
    let mut current = downloaded_size;
    let mut last_progress_update = downloaded_size;
    const PROGRESS_UPDATE_THRESHOLD: u64 = 1024 * 1024;

    let mut stream = res.bytes_stream();
    use futures::StreamExt;

    while let Some(chunk_result) = stream.next().await {
        // 检查控制信号
        if let Ok(control) = control_rx.try_recv() {
            match control {
                BatchControl::Stop => {
                    println!("Download stopped for: {}", item.filename);
                    app.emit(
                        "download://progress",
                        DownloadProgress {
                            id: item.id.clone(),
                            batch_id: item.batch_id.clone(),
                            total: total_size,
                            current,
                            status: "stopped".to_string(),
                        },
                    )?;
                    return Ok(());
                }
                BatchControl::Pause => {
                    println!("Download paused for: {}", item.filename);
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
                }
            }
        }

        let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;

        writer
            .write_all(&chunk)
            .map_err(|e| format!("Failed to write to file: {}", e))?;
        current += chunk.len() as u64;

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

fn create_http_client() -> reqwest::Client {
    use std::time::Duration;
    reqwest::Client::builder()
        .pool_max_idle_per_host(20)
        .pool_idle_timeout(Duration::from_secs(90))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .tcp_keepalive(Duration::from_secs(60))
        .build()
        .unwrap()
}

// 辅助函数：清理文件名，移除不安全字符
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

// 获取系统信息
#[tauri::command]
pub async fn get_system_info() -> Result<serde_json::Value, String> {
    let cpu_count = num_cpus::get();

    Ok(serde_json::json!({
        "cpu_cores": cpu_count,
        "recommended_concurrency": cpu_count,
        "max_concurrency": cpu_count * 2,
    }))
}

// 暂停下载
#[tauri::command]
pub async fn pause_downloads(
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
) -> Result<(), String> {
    let manager = state.lock().await;
    manager.pause().await;
    Ok(())
}

// 继续下载
#[tauri::command]
pub async fn resume_downloads(
    app: AppHandle,
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
) -> Result<(), String> {
    let manager = state.lock().await;
    manager.resume(&app).await;
    Ok(())
}

// 停止下载
#[tauri::command]
pub async fn stop_downloads(
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
) -> Result<(), String> {
    let manager = state.lock().await;
    manager.stop().await;
    Ok(())
}

// 停止单个批次
#[tauri::command]
pub async fn stop_batch(batch_id: String) -> Result<(), String> {
    println!("🛑 Attempting to stop batch: {}", batch_id);

    let mut tasks = BATCH_TASKS.lock().await;

    if let Some(info) = tasks.remove(&batch_id) {
        println!(
            "🛑 Stopping batch: {} (tasks: {})",
            batch_id,
            info.senders.len()
        );

        // 向所有任务发送停止信号
        for sender in info.senders {
            let _ = sender.send(BatchControl::Stop).await;
        }

        println!("✅ Batch stopped successfully: {}", batch_id);
        Ok(())
    } else {
        let err_msg = format!("Batch {} not found in memory.", batch_id);
        eprintln!("❌ {}", err_msg);
        Err(err_msg)
    }
}

// 暂停单个批次
#[tauri::command]
pub async fn pause_batch(batch_id: String) -> Result<(), String> {
    println!("⏸️ Attempting to pause batch: {}", batch_id);

    let mut tasks = BATCH_TASKS.lock().await;

    // 注意：暂停时不从 map 中移除，因为我们需要保留 items 信息以便恢复
    if let Some(info) = tasks.get_mut(&batch_id) {
        println!(
            "⏸️ Pausing batch: {} (tasks: {})",
            batch_id,
            info.senders.len()
        );

        // 设置状态为 Paused，通知后台调度循环停止
        info.state = BatchState::Paused;

        // 向所有任务发送停止信号（暂停本质上是停止连接，状态标记为 paused）
        for sender in &info.senders {
            let _ = sender.send(BatchControl::Pause).await;
        }

        // 清空 senders，因为旧的任务线程会退出
        info.senders.clear();

        Ok(())
    } else {
        Err(format!("Batch {} not found", batch_id))
    }
}

// 恢复单个批次
#[tauri::command]
pub async fn resume_batch(
    app: AppHandle,
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
    batch_id: String,
) -> Result<(), String> {
    println!("▶️ Attempting to resume batch: {}", batch_id);

    // 获取之前的下载项
    let items_to_resume = {
        let tasks = BATCH_TASKS.lock().await;
        if let Some(info) = tasks.get(&batch_id) {
            info.items.clone()
        } else {
            return Err(format!("Batch {} not found in memory history", batch_id));
        }
    };

    println!(
        "▶️ Resuming batch: {} (total items: {})",
        batch_id,
        items_to_resume.len()
    );

    // 重置状态为 Running
    {
        let mut tasks = BATCH_TASKS.lock().await;
        if let Some(info) = tasks.get_mut(&batch_id) {
            println!("🔄 Resetting batch {} state to Running", batch_id);
            info.state = BatchState::Running;
        } else {
            println!("❌ Resume failed: Batch {} not found", batch_id);
            return Err(format!("Batch {} not found", batch_id));
        }
    }

    let manager = state.lock().await;
    let semaphore = manager.get_semaphore();
    println!(
        "🚦 Resume process starting. Semaphore permits available: {}",
        semaphore.available_permits()
    );

    let app_clone = app.clone();
    let batch_id_clone = batch_id.clone();

    // 启动新的调度任务
    tokio::spawn(async move {
        println!("🚀 Dispatch loop started for batch: {}", batch_id_clone);
        let mut control_senders = Vec::new();

        for (index, item) in items_to_resume.iter().enumerate() {
            // 检查批次状态：如果已暂停或删除，停止生成新任务
            {
                let tasks = BATCH_TASKS.lock().await;
                if let Some(info) = tasks.get(&batch_id_clone) {
                    if info.state == BatchState::Paused {
                        println!(
                            "⏸️ Resumed batch paused again, stopping dispatch: {}",
                            batch_id_clone
                        );
                        break;
                    }
                } else {
                    println!("❌ Batch deleted during resume dispatch");
                    break;
                }
            }

            println!(
                "⏳ Waiting for semaphore (item {}/{})",
                index + 1,
                items_to_resume.len()
            );
            // 获取信号量
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("❌ Semaphore closed: {}", e);
                    break;
                }
            };
            println!(
                "✅ Semaphore acquired (item {}/{})",
                index + 1,
                items_to_resume.len()
            );

            // 再次检查状态
            {
                let mut tasks = BATCH_TASKS.lock().await;
                if let Some(info) = tasks.get_mut(&batch_id_clone) {
                    if info.state == BatchState::Paused {
                        println!("⏸️ Batch paused after semaphore acquire, dropping permit");
                        drop(permit);
                        break;
                    }

                    let app_handle = app_clone.clone();
                    let client = create_http_client();
                    let (tx, rx) = mpsc::channel(1);
                    control_senders.push(tx);
                    info.senders.push(control_senders.last().unwrap().clone());

                    let item_clone = item.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        if let Err(e) =
                            download_file_with_control(&client, &app_handle, item_clone, rx).await
                        {
                            eprintln!("Resume download failed: {}", e);
                        }
                    });
                } else {
                    drop(permit);
                    break;
                }
            }
        }
        println!("🏁 Dispatch loop finished for batch: {}", batch_id_clone);
    });

    Ok(())
}

// 获取下载管理器状态
#[tauri::command]
pub async fn get_download_state(
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
) -> Result<String, String> {
    let manager = state.lock().await;
    let state = manager.get_state().await;
    Ok(format!("{:?}", state))
}

// 获取当前并发数
#[tauri::command]
pub async fn get_current_concurrency(
    state: tauri::State<'_, Arc<Mutex<DownloadManager>>>,
) -> Result<usize, String> {
    let manager = state.lock().await;
    Ok(manager.get_concurrency())
}

// 打开文件夹
#[tauri::command]
pub async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}
