# Key-Value Store Library for Automation Controller

This Rust library provides a **generic, async-friendly key-value store** abstraction supporting multiple backends:

- **In-Memory** (for fast ephemeral storage)
- **Kubernetes ConfigMap/Secret** (for distributed configuration)
- **SQLite** (for persistent local storage)

This is intended for lightweight simple storage. It is by no means high performance or able to provide a high throughput.  Especially when using a kubernetes backend, keep in mind that with every set command the configmap or secret will be updated which can cause a high load in the kubernetes api server.

Currently it is mainly designed to be used in a simple smarthome automation controller to provide some simple persistence functionality. It is however designed generically so it can be used wherever deemed useful.&#x20;

## Features

✔ **Asynchronous API** using `tokio`
✔ **Support for multiple backends** (in-memory, Kubernetes, SQLite)
✔ **Automatic resource creation** for Kubernetes stores
✔ **Efficient caching** to reduce unnecessary API calls
✔ **Clonable storage instances** using `Arc<T>`
✔ **Base64 encoding/decoding for Kubernetes Secrets**

## Installation

Run the following command to add the library to your project:

```sh
cargo add my_library
```

## Usage

### 1. Create a Store Instance

Manually initialize the store based on your preferred backend.

#### **In-Memory Store**

```rust
use my_library::{KeyValueStore, InMemoryStore};


let store = KeyValueStore::InMemory(InMemoryStore::new());
store.set("foo", "bar").await.unwrap();
println!("Value: {:?}", store.get("foo").await);
```

#### **Kubernetes ConfigMap Store**

```rust
use my_library::{KeyValueStore, KubernetesStore, KubernetesResource};


let store = KeyValueStore::Kubernetes(KubernetesStore::new("default", "config", KubernetesResource::ConfigMap).await.unwrap());
store.set("config_key", "config_value").await.unwrap();
println!("Value: {:?}", store.get("config_key").await);
```

#### **Kubernetes Secret Store** (Handles base64 encoding/decoding)

```rust
use my_library::{KeyValueStore, KubernetesStore, KubernetesResource};


let store = KeyValueStore::Kubernetes(KubernetesStore::new("default", "my-secret", KubernetesResource::Secret).await.unwrap());
store.set("password", "supersecure").await.unwrap();
println!("Secret: {:?}", store.get("password").await);
```

#### **SQLite Store**

```rust
use my_library::{KeyValueStore, SQLiteStore};


let store = KeyValueStore::SQLite(SQLiteStore::new("store.db").await);
store.set("db_key", "db_value").await.unwrap();
println!("Value: {:?}", store.get("db_key").await);
```

## API Overview

### **`KeyValueStore`**

Represents a key-value store with different backends.

```rust
pub enum KeyValueStore {
    InMemory(InMemoryStore),
    Kubernetes(KubernetesStore),
    SQLite(SQLiteStore),
}
```

### **CRUD Methods**

All stores implement the same methods for interacting with key-value data:

```rust
impl KeyValueStore {
    pub async fn get(&self, key: &str) -> Option<String>;
    pub async fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>>;
    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
}
```

##

## License

This project is licensed under the MIT License.
