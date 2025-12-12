use crate::downloader::{DownloadItem, DownloadManager};
use axum::{extract::Query, response::Html, Router};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

// OAuth callback state
static OAUTH_STATE: once_cell::sync::Lazy<Arc<Mutex<Option<OauthResult>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

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
    let manager = state.lock().await;

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

            manager
                .add_task(DownloadItem {
                    id,
                    batch_id: batch_id.clone(),
                    url: file.user_content.url,
                    filename,
                    save_path: work_save_path.clone(),
                })
                .await;
        }
    }

    let manager_clone = state.inner().clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        let mgr = manager_clone.lock().await;
        mgr.start_downloads(&app_clone);
    });

    Ok(())
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
