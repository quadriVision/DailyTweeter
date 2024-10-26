use std::{io::{Cursor, Read}, path::PathBuf, time::Duration};

use base64::Engine;
use reqwest::Client;
use reqwest_oauth1::{OAuthClientProvider, Secrets};
use serde_json::Value;

pub enum UploadStages {
    Init,
    Append,
    Finalize
}

pub enum DataType {
    VideoMP4,
    VideoWEBM,
    VideoMPEG,
    LongVideoMP4,
    LongVideoWEBM,
    LongVideoMPEG,
    GIF
}

pub struct MediaUpload {
    pub(crate) segment_index: u32,
    pub(crate) media_id: u64,
    pub(crate) media: Cursor<Vec<u8>>
}

impl MediaUpload {
    pub async fn new(secrets: &Secrets<'_>, client: &mut Client, path: PathBuf) -> Self {
        MediaUpload {
            segment_index: 0,
            media_id: upload_part(secrets, client, path.clone(), UploadStages::Init, None).await,
            media: Cursor::new(std::fs::read(path).unwrap())
        }
    }
}

impl DataType {
    pub fn convert_to_filetype(&self) -> String {
        match self {
            DataType::VideoMP4 => String::from("video/mp4"),
            DataType::VideoWEBM => String::from("video/webm"),
            DataType::VideoMPEG => String::from("video/mpeg"),
            DataType::LongVideoMP4 => String::from("video/mp4"),
            DataType::LongVideoWEBM => String::from("video/webm"),
            DataType::LongVideoMPEG => String::from("video/mpeg"),
            DataType::GIF => String::from("image/gif"),
        }
    }
}
fn get_mime(string: String) -> String {
    match string.as_str() {
        "mp4" | "webm" | "mpeg" => format!("video/{}",string),
        "gif" => String::from("image/gif"),
        _ => String::from("image/gif")
    }
}

pub async fn upload_part(secrets: &Secrets<'_>, client: &mut Client, path: PathBuf, stage: UploadStages, upload: Option<&mut MediaUpload>) -> u64 {
    let mut form = reqwest::multipart::Form::new();
    match stage {
        UploadStages::Init => {
            let total_bytes = std::fs::read(path.clone()).unwrap().len();
            form = form.text("command", "INIT");
            form = form.text("total_bytes", format!("{}",total_bytes));
            form = form.text("media_type", get_mime(path.extension().unwrap().to_str().unwrap().to_string()));
            let upload_resp: Value = client.clone().oauth1(secrets.clone()).post("https://upload.twitter.com/1.1/media/upload.json")
            .multipart(form).send().await.unwrap().json().await.unwrap();
            println!("{:?}",upload_resp);
            return upload_resp["media_id"].as_u64().unwrap()
        },
        UploadStages::Append => {
            if let Some(upload) = upload {
                let mut content = vec![0;4_800_000];
                let read_bytes = upload.media.read(&mut content).unwrap();
                content.truncate(read_bytes);
                form = form.text("command", "APPEND");
                form = form.text("media_id", format!("{}",upload.media_id));
                form = form.part("media", reqwest::multipart::Part::bytes(content.clone()));
                form = form.text("segment_index", format!("{}",upload.segment_index));
                let upload_resp = client.clone().oauth1(secrets.clone()).post("https://upload.twitter.com/1.1/media/upload.json")
                .multipart(form).send().await.unwrap().status();
                println!("{:?}",upload_resp);
                upload.segment_index += 1;
                return content.len() as u64
            }
        },
        UploadStages::Finalize => {
            if let Some(upload) = upload {
                form = form.text("command", "FINALIZE");
                form = form.text("media_id", format!("{}",upload.media_id));
                let upload_resp: Value = client.clone().oauth1(secrets.clone()).post("https://upload.twitter.com/1.1/media/upload.json")
                .multipart(form).send().await.unwrap().json().await.unwrap();
                if upload_resp["processing_info"]["check_after_secs"].is_u64() {
                    std::thread::sleep(Duration::from_secs(upload_resp["processing_info"]["check_after_secs"].as_u64().unwrap()));
                    loop {
                        let upload_resp= client.clone().oauth1(secrets.clone())
                        .get(format!("https://upload.twitter.com/1.1/media/upload.json?command=STATUS&media_id={}",upload.media_id))
                        .send().await.unwrap();
                        println!("{:?}",upload_resp);
                        if let Ok(json) = upload_resp.json::<Value>().await {
                            if json["processing_info"]["state"].as_str().unwrap() == "in_progress" {
                                std::thread::sleep(Duration::from_secs(json["processing_info"]["check_after_secs"].as_u64().unwrap()));
                            }else if json["processing_info"]["state"].as_str().unwrap() == "succeeded" {
                                return 1;
                            }else {
                                return 0;
                            }
                        }
                        
                    }
                    
                }
                println!("{:?}",upload_resp);
                return 5
            }
            
        },
    }
    
    5
    /*let post_resp: Value = client.clone()
    .oauth1(secrets)
    .post("https://api.twitter.com/2/tweets")
    .body(
        format!(r##"{{"media":{{"media_ids":["{}"]}},"text":"testing rn"}}"##,upload_resp["media_id"])
    )
    .header("Content-Type", "application/json").send().await.unwrap().json().await.unwrap();
    println!("{:?}",post_resp);*/
}