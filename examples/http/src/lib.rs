pub async fn request(port: u16) -> reqwest::Result<String> {
    reqwest::get(format!("http://localhost:{port}/test"))
        .await?
        .text()
        .await
}
