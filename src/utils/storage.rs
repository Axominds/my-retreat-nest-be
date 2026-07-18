use axum::{
    body::Bytes,
    http::{
        HeaderMap,
        header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    },
};
use std::error::Error;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
};
use uuid::Uuid;

use crate::env::ENV;

fn replace_file_name_with_uuid(file_path: &str) -> String {
    // Get the file extension, if any
    let path = std::path::Path::new(file_path);
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    // Generate a new UUID
    let uuid: Uuid = Uuid::new_v4();

    // Build new file name
    if extension.is_empty() {
        uuid.to_string()
    } else {
        format!("{}.{}", uuid, extension)
    }
}

pub async fn store_retreat_gallery(
    file_content: Bytes,
    file_name: String,
    old_image_path: Option<String>,
) -> String {
    // Remove old image if provided
    if let Some(old_relative_path) = old_image_path {
        remove_retreat_gallery(old_relative_path).await;
    }

    let upload_dir = ENV.upload_dir.clone();
    let sub_dir: &str = "retreat/gallery";
    let upload_directory_full_path = upload_dir.join(sub_dir);
    fs::create_dir_all(&upload_directory_full_path).await.ok();

    let updated_file_name = replace_file_name_with_uuid(&file_name);
    let relative_path = format!("{sub_dir}/{updated_file_name}");
    let gallery_path = upload_dir.join(&relative_path);
    let mut file = File::create(gallery_path).await.unwrap();
    file.write_all(&file_content).await.unwrap();
    file.flush().await.unwrap();
    return relative_path;
}

pub async fn remove_retreat_gallery(image_path: String) {
    let upload_dir = ENV.upload_dir.clone();

    // Remove old image if provided
    let full_old_path = upload_dir.join(image_path);
    if fs::remove_file(&full_old_path).await.is_err() {
        eprintln!("Warning: failed to remove file at {:?}", full_old_path);
    }
}

pub async fn store_image(
    file_content: Bytes,
    file_name: String,
    sub_dir: &str,
    old_image_path: Option<String>,
) -> String {
    if let Some(old) = old_image_path {
        remove_image(old).await;
    }
    let upload_dir = ENV.upload_dir.clone();
    let full_dir = upload_dir.join(sub_dir);
    fs::create_dir_all(&full_dir).await.ok();
    let updated_file_name = replace_file_name_with_uuid(&file_name);
    let relative_path = format!("{sub_dir}/{updated_file_name}");
    let file_path = upload_dir.join(&relative_path);
    let mut file = File::create(file_path).await.unwrap();
    file.write_all(&file_content).await.unwrap();
    file.flush().await.unwrap();
    relative_path
}

pub async fn remove_image(image_path: String) {
    let upload_dir = ENV.upload_dir.clone();
    let full_path = upload_dir.join(image_path);
    if fs::remove_file(&full_path).await.is_err() {
        eprintln!("Warning: failed to remove file at {:?}", full_path);
    }
}

pub async fn read_image_with_headers(
    image_path: String,
) -> Result<(Vec<u8>, HeaderMap), Box<dyn Error>> {
    let upload_dir = ENV.upload_dir.clone();
    let full_path = upload_dir.join(image_path);
    let mut image_content: Vec<u8> = Vec::new();
    let mut file = File::open(&full_path).await?;
    file.read_to_end(&mut image_content).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        mime_guess::from_path(&full_path)
            .first_or_octet_stream()
            .as_ref()
            .parse()
            .unwrap(),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        format!(
            "attachment; filename=\"{}\"",
            &full_path.file_name().unwrap().to_string_lossy()
        )
        .parse()
        .unwrap(),
    );
    Ok((image_content, headers))
}

pub async fn read_retreat_gallery_with_headers(
    image_path: String,
) -> Result<(Vec<u8>, HeaderMap), Box<dyn Error>> {
    let upload_dir = ENV.upload_dir.clone();
    let gallery_path = upload_dir.join(image_path);
    let mut image_content: Vec<u8> = Vec::new();

    let mut file = File::open(&gallery_path).await?;
    file.read_to_end(&mut image_content).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        // Infer content type based on file extension, or set a default
        mime_guess::from_path(&gallery_path)
            .first_or_octet_stream()
            .as_ref()
            .parse()
            .unwrap(),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        format!(
            "attachment; filename=\"{}\"",
            &gallery_path.file_name().unwrap().to_string_lossy()
        )
        .parse()
        .unwrap(),
    );
    Ok((image_content, headers))
}
