use rand::Rng;

pub fn generate_otp() -> String {
    let mut rng = rand::thread_rng();
    let code: u32 = rng.gen_range(100_000..1_000_000);
    format!("{code:06}")
}

pub async fn send_otp_to_discord(webhook_url: &str, code: &str, username: &str) -> Result<(), anyhow::Error> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "content": format!("TreFM Login OTP for **{username}**: **{code}**\nThis code expires in 5 minutes.")
    });

    let response = client.post(webhook_url).json(&body).send().await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Discord webhook failed: {}",
            response.status()
        ));
    }

    Ok(())
}
