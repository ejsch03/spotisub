#[tokio::main]
async fn main() {
    match spotisub::create_auth().await {
        Ok(..) => println!("\nSuccessfully authenticated."),
        Err(e) => eprintln!("\n{}", e),
    }
}
