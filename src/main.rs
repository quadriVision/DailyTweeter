use std::{fs, path::{Path, PathBuf}, time::Duration};
use rand::{seq::IteratorRandom, thread_rng};
use reqwest::Client;
use reqwest_oauth1::Secrets;
use serde_json::Value;
use std::env;
use reqwest_oauth1::OAuthClientProvider;
pub mod multi_upload;
fn get_random_file() -> PathBuf {
    let mut rng = thread_rng();
    let files = fs::read_dir("./images").unwrap();
    let file = files.choose(&mut rng).unwrap().unwrap();
    file.path()
}
fn get_amount_of_files() -> usize {
    let files = fs::read_dir("./images").unwrap();
    files.count()
}

async fn upload_image(secrets: Secrets<'_>, client: &mut Client, path: PathBuf) {
    let mut form = reqwest::multipart::Form::new();
    form = form.file("media", path.clone()).await.unwrap();
    form = form.text("media_category", "tweet_image");
    let upload_resp: Value = client.clone().oauth1(secrets.clone()).post("https://upload.twitter.com/1.1/media/upload.json")
    .multipart(form).send().await.unwrap().json().await.unwrap();
    let post_resp: Value = client.clone()
    .oauth1(secrets)
    .post("https://api.twitter.com/2/tweets")
    .body(
        format!(r##"{{"media":{{"media_ids":["{}"]}},"text":"{}"}}"##,upload_resp["media_id"], env::var("MESSAGE").unwrap())
    )
    .header("Content-Type", "application/json").send().await.unwrap().json().await.unwrap();
    println!("{:?}",post_resp);
}

async fn upload_media(secrets: Secrets<'_>, client: &mut Client, path: PathBuf) -> u32 {
    let mut uploader = multi_upload::MediaUpload::new(&secrets, client, path).await;
    let mut content_len = 4_800_000;
    while content_len == 4_800_000 {
        content_len = multi_upload::upload_part(
            &secrets,
            client,
            Path::new("/").to_path_buf(), // we don't need to specify the path anymore
            multi_upload::UploadStages::Append, 
            Some(&mut uploader))
        .await;
    }
    let finalize = multi_upload::upload_part(
        &secrets,
        client,
        Path::new("/").to_path_buf(), // we don't need to specify the path anymore
        multi_upload::UploadStages::Finalize, 
        Some(&mut uploader))
    .await;
    if finalize == 0 {
        return 0;
    }
    let post_resp: Value = client.clone()
    .oauth1(secrets)
    .post("https://api.twitter.com/2/tweets")
    .body(
        format!(r##"{{"media":{{"media_ids":["{}"]}},"text":"{}"}}"##,uploader.media_id, env::var("MESSAGE").unwrap())
    )
    .header("Content-Type", "application/json").send().await.unwrap().json().await.unwrap();
    println!("{:?}",post_resp);
    return 1;
}


#[tokio::main]
async fn main() {
    let mut used_files: Vec<PathBuf> = Vec::new();
    dotenv::dotenv().ok();
    let secrets: Secrets<'_> = reqwest_oauth1::Secrets::new(env::var("API_KEY").unwrap(), env::var("API_SECRET").unwrap())
    .token(env::var("ACCESS").unwrap(), env::var("ACCESS_SECRET").unwrap());
    let image_extensions = ["png","jpg","jpeg","webp"];
    let mut client = reqwest::Client::new();
    loop {
        let mut path: PathBuf = PathBuf::new();
        'get_file: loop {
            path = get_random_file();
            if used_files.len() < (get_amount_of_files()/3) {
                if !used_files.contains(&path) {
                    break 'get_file;
                }
            }else {
                if !used_files.iter().rev().take(get_amount_of_files()/3).collect::<Vec<&PathBuf>>().contains(&&path) {
                    break 'get_file;
                }
            }
        }
        if image_extensions.contains(&path.extension().unwrap().to_str().unwrap()) {
            upload_image(secrets.clone(), &mut client, path).await;
        }else {
            upload_media(secrets.clone(), &mut client, path).await;
        }
        std::thread::sleep(Duration::from_secs(86400));
    }
    
}
