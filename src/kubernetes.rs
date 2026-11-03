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
        let metadata = kube::api::ObjectMeta {
            name: Some(name.to_string()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        };

        // Only a NotFound answer creates an empty resource; any other error
        // fails construction instead of starting with an empty cache.
        let data = match resource_type {
            KubernetesResource::ConfigMap => {
                let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
                let empty = ConfigMap {
                    metadata,
                    data: Some(BTreeMap::new()),
                    ..Default::default()
                };
                get_or_create(&api, name, empty)
                    .await?
                    .data
                    .unwrap_or_default()
            }
            KubernetesResource::Secret => {
                let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
                let empty = Secret {
                    metadata,
                    data: Some(BTreeMap::new()),
                    ..Default::default()
                };
                let secret = get_or_create(&api, name, empty).await?;
                secret
                    .data
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|(key, value)| {
                        String::from_utf8(value.0)
                            .ok()
                            .map(|decoded| (key, decoded))
                    })
                    .collect()
            }
        };
        let cache = Arc::new(RwLock::new(data));

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
        // Hold the lock across the patch, as delete does, so concurrent writes
        // reach the resource and the cache in the same order, and a failed
        // patch leaves the cache unchanged.
        let mut cache_write = self.cache.write().await;

        let patch = set_patch(key, value, self.resource_type);
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

        cache_write.insert(key.to_string(), value.to_string());

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Hold the lock across the patch so the cache changes only after the
        // resource did, and a failed patch leaves both unchanged.
        let mut cache_write = self.cache.write().await;

        // Patch even when the key is not cached: a Secret value that is not
        // UTF-8 is skipped when loading, but must still be deletable.
        let patch = delete_patch(key);
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

        cache_write.remove(key);

        Ok(())
    }
}

/// Merge patch body that removes `key` from a ConfigMap or Secret.
///
/// A JSON merge patch (RFC 7386) removes a field only when it is set to null;
/// omitting the field leaves it in place.
fn delete_patch(key: &str) -> serde_json::Value {
    json!({"data": {key: null}})
}

/// Merge patch body that sets `key` to `value`. Secret data is base64 encoded.
fn set_patch(key: &str, value: &str, resource_type: KubernetesResource) -> serde_json::Value {
    match resource_type {
        KubernetesResource::ConfigMap => json!({"data": {key: value}}),
        KubernetesResource::Secret => json!({"data": {key: BASE64_STANDARD.encode(value)}}),
    }
}

/// Whether the API server answered that the resource does not exist.
fn is_not_found(err: &kube::Error) -> bool {
    matches!(err, kube::Error::Api(response) if response.code == 404)
}

/// Whether the API server answered that the resource already exists.
fn is_already_exists(err: &kube::Error) -> bool {
    matches!(err, kube::Error::Api(response) if response.code == 409)
}

/// Reads the resource, creating `empty` when the server says it does not exist.
///
/// Every other error is returned, so a permission or network problem is never
/// mistaken for a missing resource and the store never starts with an empty
/// cache while the resource holds data.
async fn get_or_create<K>(api: &Api<K>, name: &str, empty: K) -> Result<K, kube::Error>
where
    K: kube::Resource + Clone + serde::de::DeserializeOwned + serde::Serialize + std::fmt::Debug,
{
    match api.get(name).await {
        Ok(resource) => Ok(resource),
        Err(err) if is_not_found(&err) => match api.create(&PostParams::default(), &empty).await {
            Ok(created) => Ok(created),
            // Someone else created it between our get and create: read theirs.
            Err(err) if is_already_exists(&err) => api.get(name).await,
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::core::ErrorResponse;

    fn api_error(code: u16, reason: &str) -> kube::Error {
        kube::Error::Api(ErrorResponse {
            status: "Failure".to_string(),
            message: format!("{reason} for test"),
            reason: reason.to_string(),
            code,
        })
    }

    #[test]
    fn only_a_404_api_error_means_the_resource_is_missing() {
        assert!(is_not_found(&api_error(404, "NotFound")));
        assert!(!is_not_found(&api_error(403, "Forbidden")));
        assert!(!is_not_found(&api_error(401, "Unauthorized")));
        assert!(!is_not_found(&api_error(500, "InternalError")));
        assert!(!is_not_found(&kube::Error::LinesCodecMaxLineLengthExceeded));
    }

    #[test]
    fn only_a_409_api_error_means_the_resource_already_exists() {
        assert!(is_already_exists(&api_error(409, "AlreadyExists")));
        assert!(!is_already_exists(&api_error(404, "NotFound")));
        assert!(!is_already_exists(&api_error(403, "Forbidden")));
        assert!(!is_already_exists(
            &kube::Error::LinesCodecMaxLineLengthExceeded
        ));
    }

    #[test]
    fn set_patch_for_a_configmap_sends_the_value_as_is() {
        assert_eq!(
            set_patch("hcd.mutes.v1", "{\"a\":1}", KubernetesResource::ConfigMap),
            json!({"data": {"hcd.mutes.v1": "{\"a\":1}"}})
        );
    }

    // "hello" in base64 is "aGVsbG8=".
    #[test]
    fn set_patch_for_a_secret_sends_the_value_base64_encoded() {
        assert_eq!(
            set_patch("token", "hello", KubernetesResource::Secret),
            json!({"data": {"token": "aGVsbG8="}})
        );
    }

    // ConfigMap and Secret both store entries under `data`, and a JSON merge
    // patch removes an entry only when it is set to null, so delete sends the
    // same body for either resource type.
    #[test]
    fn delete_patch_sets_only_the_deleted_key_to_null() {
        assert_eq!(delete_patch("muted"), json!({"data": {"muted": null}}));
    }

    #[test]
    fn delete_patch_keeps_the_key_verbatim() {
        assert_eq!(
            delete_patch("hcd.mutes.v1"),
            json!({"data": {"hcd.mutes.v1": null}})
        );
    }
}
