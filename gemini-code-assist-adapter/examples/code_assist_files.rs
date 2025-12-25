use gemini_code_assist_adapter::CodeAssistClient;
use gemini_rust::{
    Content, GenerateContentRequest, GenerationConfig, Part, Role, Blob
};
use std::env;
use std::io::Write;

const TEST_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("GCLOUD_ACCESS_TOKEN").expect("GCLOUD_ACCESS_TOKEN must be set");
    let raw_project = env::var("GCLOUD_PROJECT_ID").expect("GCLOUD_PROJECT_ID must be set");

    println!("🔑 Initializing client...");
    let mut client = CodeAssistClient::new(token, raw_project);

    // Handshake
    match client.load_code_assist().await {
        Ok(real_proj) => {
            println!("🤝 Handshake success. Real Project: {}", real_proj);
            client.set_project_id(real_proj);
        },
        Err(e) => eprintln!("⚠️ Handshake warning: {}", e),
    }

    // Onboarding
    if let Err(e) = client.onboard_user().await {
        eprintln!("⚠️ Onboarding warning: {}", e);
    }

    // --- TEST 1: Image ---
    println!("\n🖼️  TEST 1: Multimodal (Image Analysis)...");

    client = client.with_model("models/gemini-2.0-flash");

    let request_image = GenerateContentRequest {
        contents: vec![Content {
            role: Some(Role::User),
            parts: Some(vec![
                Part::Text {
                    text: "What is this? Answer in one word.".to_string(),
                    thought: None,
                    thought_signature: None,
                },
                Part::InlineData {
                    inline_data: Blob {
                        mime_type: "image/png".to_string(),
                        data: TEST_PNG_BASE64.to_string(),
                    },
                    media_resolution: None,
                }
            ]),
        }],
        generation_config: Some(GenerationConfig {
            temperature: Some(0.4),
            max_output_tokens: Some(100),
            ..Default::default()
        }),
        safety_settings: None,
        tools: None,
        tool_config: None,
        system_instruction: None,
        cached_content: None,
    };

    run_stream(&client, &request_image).await?;

    // --- TEST 2: Code ---
    println!("\n📂 TEST 2: Project Context...");

    let fake_file_content = r#"
    pub fn calculate_pi() -> f64 {
        3.14
    }
    "#;

    let system_prompt = format!(
        "You are a helper. Here is the file:\n```rust\n{}\n```",
        fake_file_content
    );

    let request_code = GenerateContentRequest {
        system_instruction: Some(Content::text(system_prompt)),
        contents: vec![
            Content::text("Make the function more precise.").with_role(Role::User)
        ],
        generation_config: Some(GenerationConfig {
            max_output_tokens: Some(512),
            ..Default::default()
        }),
        safety_settings: None,
        tools: None,
        tool_config: None,
        cached_content: None,
    };

    run_stream(&client, &request_code).await?;

    Ok(())
}

async fn run_stream(client: &CodeAssistClient, req: &GenerateContentRequest) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client.generate_content_stream(req).await?;
    print!("🤖 Answer: ");
    std::io::stdout().flush()?;
    while let Some(chunk_res) = futures::StreamExt::next(&mut stream).await {
        match chunk_res {
            Ok(resp) => {
                print!("{}", resp.text());
                std::io::stdout().flush()?;
            },
            Err(e) => eprintln!("\n❌ Error chunk: {}", e),
        }
    }
    println!("\n✅ Done.");
    Ok(())
}
