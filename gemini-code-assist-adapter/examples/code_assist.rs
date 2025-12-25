use gemini_code_assist_adapter::CodeAssistClient;
use gemini_rust::{
    Content, GenerateContentRequest, GenerationConfig, Role,
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get token, e.g., from `gcloud auth print-access-token`
    let oauth_token = env::var("GCLOUD_ACCESS_TOKEN").expect("GCLOUD_ACCESS_TOKEN not set");
    let project_id = env::var("GCLOUD_PROJECT_ID").expect("GCLOUD_PROJECT_ID not set");

    let mut client = CodeAssistClient::new(oauth_token, project_id)
        .with_model("gemini-3-flash-preview");

    if let Ok(proj) = client.load_code_assist().await {
        client.set_project_id(proj);
    }

    if let Err(e) = client.onboard_user().await {
        eprintln!("Onboarding warning: {}", e);
        // Do not fail, try to continue, maybe already active
    }

    let request = GenerateContentRequest {
        contents: vec![
            Content::text("Hi! Write Hello World in Rust.")
                .with_role(Role::User)
        ],
        generation_config: Some(GenerationConfig {
            max_output_tokens: Some(1024),
            temperature: Some(0.5),
            ..Default::default()
        }),
        safety_settings: None,
        tools: None,
        tool_config: None,
        system_instruction: Some(Content::text("You are an experienced Rust developer.")),
        cached_content: None,
    };

    println!("Sending request via Code Assist API...");

    let mut stream = client.generate_content_stream(&request).await?;

    while let Some(chunk_result) = futures::StreamExt::next(&mut stream).await {
        match chunk_result {
            Ok(response) => {
                print!("{}", response.text());
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    println!("\nDone.");

    Ok(())
}
