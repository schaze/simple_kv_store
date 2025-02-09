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
cargo add simple_kv_store
```

## Usage

### 1. Create a Store Instance

Manually initialize the store based on your preferred backend.

#### **In-Memory Store**

```rust
use my_library::{KeyValueStore, InMemoryStore};


let store = KeyValueStore::InMemory(InMemoryStore::new());
let value = true; // define a owned string value
store.set("some_key", &value).await.unwrap(); // store the value
println!("Value: {}", store.get::<bool>("some_key").await.unwrap()); // retrieve the value -- type annotations are
                                                                     // needed in this case as no type can be deferred
                                                                     // from the println! usage
```

#### **Kubernetes ConfigMap Store**

```rust
use my_library::{KeyValueStore, KubernetesStore, KubernetesResource};


let store = KeyValueStore::Kubernetes(KubernetesStore::new("default", "config", KubernetesResource::ConfigMap).await.unwrap());
let value = 123; // define a owned string value
store.set("some_key", &value).await.unwrap(); // store the value
println!("Value: {}", store.get::<i64>("some_key").await.unwrap()); // retrieve the value -- type annotations are
                                                                    // needed in this case as no type can be deferred
                                                                    // from the println! usage
```

_Be aware:_ Kubernetes requires keys to consist of **alphanumeric characters (`A-Z`, `a-z`, `0-9`), dashes (`-`), underscores (`_`), and dots (`.`)**.

#### **Kubernetes Secret Store** (Handles base64 encoding/decoding)

```rust
use my_library::{KeyValueStore, KubernetesStore, KubernetesResource};


let store = KeyValueStore::Kubernetes(KubernetesStore::new("default", "my-secret", KubernetesResource::Secret).await.unwrap());
let value = 123.123; // define a owned string value
store.set("some_key", &value).await.unwrap(); // store the value
println!("Value: {}", store.get::<f64>("some_key").await.unwrap()); // retrieve the value -- type annotations are
                                                                    // needed in this case as no type can be deferred
                                                                    // from the println! usage
```

_Be aware:_ Kubernetes requires keys to consist of **alphanumeric characters (`A-Z`, `a-z`, `0-9`), dashes (`-`), underscores (`_`), and dots (`.`)**.

#### **SQLite Store**

```rust
use my_library::{KeyValueStore, SQLiteStore};


let store = KeyValueStore::SQLite(SQLiteStore::new("store.db").await); // create a sqlite backed store
let value = "some_value".to_string(); // define a owned string value
store.set("some_key", &value).await.unwrap(); // store the value
println!("Value: {}", store.get::<String>("some_key").await.unwrap()); // retrieve the value -- type annotations are
                                                                       // needed in this case as no type can be deferred
                                                                       // from the println! usage
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

`KeyValueStore` implements the `get`, `set` and `delete` functions and will accept and return any type as value that can be serialized and deserialized with serde.

```rust

impl KeyValueStore {
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        ...
    }

    pub async fn set<T: Serialize>( &self, key: &str, value: &T,) -> Result<(), Box<dyn std::error::Error>> {
        ...
    }

    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        ...
    }
}

```

##

## License

This project is licensed under the MIT License.
