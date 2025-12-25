use gemini_code_assist_adapter::auth::GoogleAuthManager;
use gemini_code_assist_adapter::CodeAssistClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let auth_manager = GoogleAuthManager::new();

    println!("Opening browser for authentication...");
    let token = auth_manager.login().await?;
    println!("Successfully logged in!");

    let projects = auth_manager.list_projects(&token).await?;

    if projects.is_empty() {
        return Err("No active Google Cloud projects found. Please create one.".into());
    }

    let project_id = projects[0].clone();
    println!("Using project: {}", project_id);

    println!("token={token}, project_id={project_id}");
    let client = CodeAssistClient::new(token, project_id);
    Ok(())
}
