
# authku
A rust library for authorization and authentication in HKU eLearning system, including hkuportal, moodle, library, etc.

### Usage
```rust
#[tokio::main]
fn main() {
    let client = authku::Client::new();
    client.login_lib("uid", "password")
        .await.unwrap();

    client.login_portal("uid", "password")
        .await.unwrap();

    client.login_moodle("uid", "password")
        .await.unwrap();
}
```

### TODO
- [x] portal
- [x] library
- [ ] moodle