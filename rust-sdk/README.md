# PocketBase Rust SDK

A Rust implementation of the PocketBase SDK, providing similar functionality to the official JavaScript SDK.

## Features

- Full CRUD operations for collections and records
- Authentication (password, OAuth2, OTP)
- File handling
- Health checks
- Settings management
- Log access
- Backup management
- Cron job management
- Batch operations

## Usage

```rust
use pocketbase_sdk::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new client
    let client = Client::new("http://127.0.0.1:8090");
    
    // Health check
    let health = client.health().check().await?;
    println!("Health: {:?}", health);
    
    // Authenticate
    let auth_data = client
        .collection("users")
        .auth_with_password("user@example.com", "password123")
        .await?;
    println!("Authenticated as: {}", auth_data.record.id);
    
    // CRUD operations
    let records = client
        .collection("posts")
        .get_list(1, 20, None)
        .await?;
    
    for record in records.items {
        println!("Record: {:?}", record);
    }
    
    Ok(())
}
```

## License

MIT
