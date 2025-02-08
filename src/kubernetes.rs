use base64::{prelude::BASE64_STANDARD, Engine};
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::{
    api::{Api, Patch, PatchParams, PostParams},
    Client,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct KubernetesStore {
    namespace: String,
    name: String,
    client: Client,
    cache: Arc<RwLock<BTreeMap<String, String>>>,
    resource_type: KubernetesResource, // Determines if we're using ConfigMap or Secret
}

#[derive(Clone, PartialEq, Debug, Copy)]
pub enum KubernetesResource {
    ConfigMap,
    Secret,
}

impl KubernetesStore {
    pub async fn new(
        namespace: &str,
        name: &str,
        resource_type: KubernetesResource,
    ) -> Result<Self, kube::Error> {
        let client = Client::try_default().await?;
        let cache = Arc::new(RwLock::new(BTreeMap::new()));

        let api_configmap: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
        let api_secret: Api<Secret> = Api::namespaced(client.clone(), namespace);

        // **Check if resource exists, if not, create an empty one**
        match resource_type {
            KubernetesResource::ConfigMap => {
                if api_configmap.get(name).await.is_err() {
                    let new_configmap = ConfigMap {
                        metadata: kube::api::ObjectMeta {
                            name: Some(name.to_string()),
                            namespace: Some(namespace.to_string()),
                            ..Default::default()
                        },
                        data: Some(BTreeMap::new()), // Empty ConfigMap
                        ..Default::default()
                    };
                    api_configmap
                        .create(&PostParams::default(), &new_configmap)
                        .await?;
                }
            }
            KubernetesResource::Secret => {
                if api_secret.get(name).await.is_err() {
                    let new_secret = Secret {
                        metadata: kube::api::ObjectMeta {
                            name: Some(name.to_string()),
                            namespace: Some(namespace.to_string()),
                            ..Default::default()
                        },
                        data: Some(BTreeMap::new()), // Empty Secret
                        ..Default::default()
                    };
                    api_secret
                        .create(&PostParams::default(), &new_secret)
                        .await?;
                }
            }
        }

        if resource_type == KubernetesResource::ConfigMap {
            if let Ok(config_map) = api_configmap.get(name).await {
                if let Some(data) = config_map.data {
                    let mut cache_write = cache.write().await;
                    *cache_write = data;
                }
            }
        } else if let Ok(secret) = api_secret.get(name).await {
            if let Some(data) = secret.data {
                let mut cache_write = cache.write().await;
                for (key, value) in data {
                    if let Ok(decoded) = String::from_utf8(value.0) {
                        cache_write.insert(key, decoded);
                    }
                }
            }
        }

        Ok(Self {
            namespace: namespace.to_string(),
            name: name.to_string(),
            client,
            cache,
            resource_type,
        })
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let cache_read = self.cache.read().await;
        cache_read.get(key).cloned()
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let patch = json!({"data": {key: value}});
        let patch_params = PatchParams::apply("simple-kv-store");

        if self.resource_type == KubernetesResource::ConfigMap {
            let api: Api<ConfigMap> = Api::namespaced(self.client.clone(), &self.namespace);
            api.patch(&self.name, &patch_params, &Patch::Merge(patch))
                .await?;
        } else {
            let api: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
            let encoded_value = BASE64_STANDARD.encode(value);
            let patch = json!({"data": {key: encoded_value}});
            api.patch(&self.name, &patch_params, &Patch::Merge(patch))
                .await?;
        }

        let mut cache_write = self.cache.write().await;
        cache_write.insert(key.to_string(), value.to_string());

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut cache_write = self.cache.write().await;

        if cache_write.remove(key).is_some() {
            let patch = json!({"data": cache_write.clone()});
            let patch_params = PatchParams::apply("simple-kv-store");

            if self.resource_type == KubernetesResource::ConfigMap {
                let api: Api<ConfigMap> = Api::namespaced(self.client.clone(), &self.namespace);
                api.patch(&self.name, &patch_params, &Patch::Merge(patch))
                    .await?;
            } else {
                let api: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
                api.patch(&self.name, &patch_params, &Patch::Merge(patch))
                    .await?;
            }
        }

        Ok(())
    }
}
