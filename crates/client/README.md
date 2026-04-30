# ferro-client

Async WebDAV client SDK for Ferro servers, with optional C-FFI for mobile platforms (Swift/Kotlin).

## Rust Usage

```rust
use ferro_client::FerroClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = FerroClient::new("https://ferro.example.com", "my-token");

    // Test connection
    let info = client.test_connection().await?;
    println!("Server has {} root entries", info.root_entries);

    // List files
    let files = client.list("/").await?;
    for file in &files {
        println!("{} ({} bytes)", file.name, file.size);
    }

    // Upload
    client.put_text("/hello.txt", "Hello, Ferro!").await?;

    // Download
    let content = client.get_text("/hello.txt").await?;
    println!("{}", content);

    // Create directory
    client.create_directory("/documents").await?;

    // Move / Copy
    client.move_item("/hello.txt", "/documents/hello.txt").await?;
    client.copy("/documents/hello.txt", "/documents/hello-backup.txt").await?;

    // Delete
    client.delete("/documents/hello-backup.txt").await?;

    Ok(())
}
```

## C-FFI (Mobile)

Enable the `ffi` feature to expose C-compatible bindings:

```toml
[dependencies]
ferro-client = { version = "0.1", features = ["ffi"] }
```

### FFI API

```c
typedef struct FerroClientHandle FerroClientHandle;
typedef enum {
    FerroResult_Success = 0,
    FerroResult_ErrorNetwork = 1,
    FerroResult_ErrorAuth = 2,
    FerroResult_ErrorNotFound = 3,
    FerroResult_ErrorHttp = 4,
    FerroResult_ErrorXml = 5,
    FerroResult_ErrorInvalidArg = 6,
    FerroResult_ErrorUnknown = 99,
} FerroResult;

typedef struct {
    char *name;
    char *path;
    uint64_t size;
    bool is_dir;
    char *modified;
    char *etag;
} FerroFileEntry;

typedef struct {
    FerroFileEntry *entries;
    size_t count;
    FerroResult result;
} FerroFileList;

// Create/destroy client
FerroClientHandle *ferro_client_new(const char *server_url, const char *token);
void ferro_client_free(FerroClientHandle *handle);

// Operations
FerroResult ferro_test_connection(FerroClientHandle *handle);

// Cleanup
void ferro_file_list_free(FerroFileList *list);
void ferro_bytes_free(FerroBytes *bytes);
void ferro_string_free(char *s);
```

### Swift Example

```swift
let handle = ferro_client_new("https://ferro.example.com", "my-token")
defer { ferro_client_free(handle) }

let result = ferro_test_connection(handle)
if result == .Success {
    print("Connected!")
}
```

### Kotlin Example

```kotlin
val handle = ferro_client_new("https://ferro.example.com", "my-token")
try {
    val result = ferro_test_connection(handle)
    if (result == FerroResult.Success) {
        println("Connected!")
    }
} finally {
    ferro_client_free(handle)
}
```

## Safety

- All FFI pointer returns must be freed with their corresponding `_free` function
- String pointers are null-terminated UTF-8
- Client handles are not thread-safe; use one per thread or synchronize externally

## License

AGPL-3.0-or-later
