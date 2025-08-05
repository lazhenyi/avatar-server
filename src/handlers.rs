use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::{error, get, post, web, Error, HttpResponse, Responder};
use chrono::{DateTime, Local};
use futures::{StreamExt, TryStreamExt};
use handlebars::Handlebars;
use std::io::Write;
use std::{fs, path::Path};
use uuid::Uuid;

use crate::config::AppConfig;

/// Upload response structure (上传响应结构)
#[derive(serde::Serialize)]
pub struct UploadResponse {
    status: String,
    path: String,
}

/// Statistics data structure (统计数据结构)
#[derive(serde::Serialize)]
pub struct StatsData {
    total_images: usize,
    today_new_images: usize,
    current_date: String,
}

/// Upload avatar handler (上传头像处理器)
#[post("/upload/{user_id}")]
pub async fn upload_avatar(
    config: web::Data<AppConfig>,
    mut payload: Multipart,
    path: web::Path<String>,
) -> Result<web::Json<UploadResponse>, Error> {
    let user_id = path.into_inner();

    // Ensure upload directory exists (确保上传目录存在)
    let upload_path = Path::new(&config.upload_dir);
    if !upload_path.exists() {
        fs::create_dir_all(upload_path).map_err(|e| error::ErrorInternalServerError(e))?;
    }

    // Process uploaded file (处理上传文件)
    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_type = field
            .content_disposition()
            .ok_or_else(|| error::ErrorBadRequest("No content disposition"))?;

        let filename = content_type
            .get_filename()
            .ok_or_else(|| error::ErrorBadRequest("No filename"))?
            .to_string();

        // Get file extension (获取文件扩展名)
        let extension = Path::new(&filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");

        // Generate unique filename (生成唯一文件名)
        let file_id = Uuid::new_v4();
        let filepath = format!("{}/{}.{}", config.upload_dir, file_id, extension);

        // Save file (保存文件)
        let mut file = web::block(move || std::fs::File::create(&filepath))
            .await?
            .map_err(|e| error::ErrorInternalServerError(e))?;

        // Write file content (写入文件内容)
        while let Some(chunk) = field.next().await {
            let data = chunk.map_err(|e| error::ErrorInternalServerError(e))?;
            file = web::block(move || file.write_all(&data).map(|_| file))
                .await?
                .map_err(|e| error::ErrorInternalServerError(e))?;
        }

        // Save user avatar mapping (保存用户头像映射)
        let user_dir = format!("{}/{}", config.upload_dir, user_id);
        if !Path::new(&user_dir).exists() {
            fs::create_dir_all(&user_dir).map_err(|e| error::ErrorInternalServerError(e))?;
        }

        let user_avatar_path = format!("{}/current.avatar", user_dir);
        std::fs::write(&user_avatar_path, format!("{}.{}", file_id, extension))
            .map_err(|e| error::ErrorInternalServerError(e))?;

        return Ok(web::Json(UploadResponse {
            status: "success".to_string(),
            path: format!("/avatars/{}", user_id),
        }));
    }

    Err(error::ErrorBadRequest("Invalid upload"))
}

/// Get avatar handler (获取头像处理器)
#[get("/avatars/{user_id}")]
pub async fn get_avatar(
    config: web::Data<AppConfig>,
    path: web::Path<String>,
) -> Result<impl Responder, Error> {
    let user_id = path.into_inner();
    let user_dir = format!("{}/{}", config.upload_dir, user_id);
    let user_avatar_path = format!("{}/current.avatar", user_dir);

    if Path::new(&user_avatar_path).exists() {
        let avatar_filename = fs::read_to_string(user_avatar_path)
            .map_err(|_| error::ErrorNotFound("Avatar not found"))?;

        let filepath = format!("{}/{}", config.upload_dir, avatar_filename);

        if Path::new(&filepath).exists() {
            Ok(NamedFile::open(filepath)?)
        } else {
            Err(error::ErrorNotFound("Avatar file not found"))
        }
    } else {
        Err(error::ErrorNotFound("Avatar not found"))
    }
}

/// Get statistics handler (获取统计信息处理器)
#[get("/stats")]
pub async fn get_stats(
    config: web::Data<AppConfig>,
    hb: web::Data<Handlebars<'_>>,
) -> Result<impl Responder, Error> {
    let upload_dir = &config.upload_dir;
    let today = Local::now().date_naive();

    let mut total_images = 0;
    let mut today_new_images = 0;

    // Traverse upload directory (遍历上传目录)
    if let Ok(entries) = fs::read_dir(upload_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            // Only count image files (只统计图片文件)
            if path.is_file() && is_image_file(&path) {
                total_images += 1;

                // Check if uploaded today (检查是否是今天上传的)
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(created) = metadata.created() {
                        if let Ok(duration) = created.duration_since(std::time::UNIX_EPOCH) {
                            let created_datetime = DateTime::from_timestamp(
                                duration.as_secs() as i64,
                                duration.subsec_nanos(),
                            );

                            if let Some(created_datetime) = created_datetime {
                                let created_date = created_datetime.date_naive();
                                if created_date == today {
                                    today_new_images += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let data = StatsData {
        total_images,
        today_new_images,
        current_date: today.format("%Y-%m-%d").to_string(),
    };

    let body = hb.render("stats", &data)
        .map_err(|e| error::ErrorInternalServerError(format!("Template error: {}", e)))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(body))
}

/// Check if file is an image (检查文件是否为图片)
fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(ext.to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp")
    } else {
        false
    }
}