use std::{fmt::Debug, hash::Hash, pin::Pin, sync::Arc, time::Duration};

use crate::{context::Context, error::DatabricksKubeError};

use assert_json_diff::assert_json_matches_no_panic;
use futures::{Future, FutureExt, Stream, StreamExt, TryFutureExt, TryStreamExt};

use k8s_openapi::NamespaceResourceScope;

use kube::{
    api::PostParams,
    runtime::{
        controller::Action, finalizer, finalizer::Event, reflector::ObjectRef, watcher, Controller,
    },
    Api, CustomResourceExt, Resource, ResourceExt,
};

use serde::{de::DeserializeOwned, Serialize};

#[allow(dead_code)]
async fn reconcile_apply<TAPIType, TCRDType>(
    resource: Arc<TCRDType>,
    context: Arc<Context>,
) -> Result<Action, DatabricksKubeError>
where
    TCRDType: From<TAPIType>,
    TCRDType: Resource<Scope = NamespaceResourceScope> + ResourceExt + CustomResourceExt,
    TCRDType::DynamicType: Default + Eq + Hash,
    TCRDType: RemoteAPIResource<TAPIType>,
    TCRDType: Send,
    TCRDType: Serialize,
    TCRDType: Sync,
    TCRDType: Default,
    TCRDType: Clone,
    TCRDType: CustomResourceExt,
    TCRDType: Debug,
    TCRDType: DeserializeOwned,
    TCRDType: 'static,
    TAPIType: From<TCRDType>,
    TAPIType: PartialEq,
    TAPIType: Send,
    TAPIType: Serialize,
    TAPIType: 'static,
{
    let mut resource = resource;
    let kube_api = Api::<TCRDType>::default_namespaced(context.client.clone());
    let kube_as_api: TAPIType = resource.as_ref().clone().into();

    let latest_remote = resource.remote_get(context.clone()).next().await.unwrap();
    let requeue_secs = context
        .as_ref()
        .get_operator_config()
        .and_then(|c| c.default_requeue_interval)
        .unwrap_or(300);

    match latest_remote {
        Err(DatabricksKubeError::IDUnsetError) => {
            log::info!(
                "Resource {} {} is missing in Databricks, creating",
                TCRDType::api_resource().kind,
                resource.name_unchecked()
            );

            let created = resource
                .remote_create(context.clone())
                .next()
                .await
                .unwrap()?;

            log::info!(
                "Created {} {} in Databricks",
                TCRDType::api_resource().kind,
                resource.name_unchecked()
            );

            kube_api
                .replace(&resource.name_unchecked(), &PostParams::default(), &created)
                .await
                .map_err(|e| DatabricksKubeError::ResourceUpdateError(e.to_string()))?;

            log::info!(
                "Updated {} {} in K8S",
                TCRDType::api_resource().kind,
                resource.name_unchecked()
            );
        }
        Err(other) => return Err(other),
        Ok(remote) => {
            if remote != kube_as_api {
                log::info!(
                    "Resource {} {} drifted!\nDiff (remote, kube):\n{}",
                    TCRDType::api_resource().kind,
                    resource.name_unchecked(),
                    assert_json_matches_no_panic(
                        &remote,
                        &kube_as_api,
                        assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict)
                    )
                    .unwrap_err()
                );

                log::info!(
                    "Resource {} {} reconciling drift...",
                    TCRDType::api_resource().kind,
                    resource.name_unchecked()
                );

                let updated = resource
                    .remote_update(context.clone())
                    .next()
                    .await
                    .unwrap()?;

                let replaced = kube_api
                    .replace(&resource.name_unchecked(), &PostParams::default(), &updated)
                    .await
                    .map_err(|e| DatabricksKubeError::ResourceUpdateError(e.to_string()))?;

                resource = replaced.into();

                log::info!(
                    "Updated {} {} in K8S",
                    TCRDType::api_resource().kind,
                    resource.name_unchecked()
                );
            }
        }
    }

    resource.every_reconcile(context.clone()).await?;
    Ok(Action::requeue(Duration::from_secs(requeue_secs)))
}

#[allow(dead_code)]
async fn reconcile_delete<TAPIType, TCRDType>(
    resource: Arc<TCRDType>,
    context: Arc<Context>,
) -> Result<Action, DatabricksKubeError>
where
    TCRDType: From<TAPIType>,
    TCRDType: Resource<Scope = NamespaceResourceScope> + ResourceExt + CustomResourceExt,
    TCRDType::DynamicType: Default + Eq + Hash,
    TCRDType: RemoteAPIResource<TAPIType>,
    TCRDType: Send,
    TCRDType: Serialize,
    TCRDType: Sync,
    TCRDType: Default,
    TCRDType: Clone,
    TCRDType: CustomResourceExt,
    TCRDType: Debug,
    TCRDType: DeserializeOwned,
    TCRDType: 'static,
    TAPIType: From<TCRDType>,
    TAPIType: PartialEq,
    TAPIType: Send,
    TAPIType: Serialize,
    TAPIType: 'static,
{
    log::info!(
        "Removing {} {} from Databricks",
        TCRDType::api_resource().kind,
        resource.name_unchecked()
    );

    resource.remote_delete(context.clone()).next().await;

    log::info!(
        "Removed {} {} from Databricks",
        TCRDType::api_resource().kind,
        resource.name_unchecked()
    );

    Ok(Action::await_change())
}

#[allow(dead_code)]
async fn reconcile<TAPIType, TCRDType>(
    resource: Arc<TCRDType>,
    context: Arc<Context>,
) -> Result<Action, DatabricksKubeError>
where
    TCRDType: From<TAPIType>,
    TCRDType: Resource<Scope = NamespaceResourceScope> + ResourceExt + CustomResourceExt,
    TCRDType::DynamicType: Default + Eq + Hash,
    TCRDType: RemoteAPIResource<TAPIType>,
    TCRDType: Send,
    TCRDType: Serialize,
    TCRDType: Sync,
    TCRDType: Default,
    TCRDType: Clone,
    TCRDType: CustomResourceExt,
    TCRDType: Debug,
    TCRDType: DeserializeOwned,
    TCRDType: 'static,
    TAPIType: From<TCRDType>,
    TAPIType: PartialEq,
    TAPIType: Send,
    TAPIType: Serialize,
    TAPIType: 'static,
{
    let kube_api = Api::<TCRDType>::default_namespaced(context.client.clone());

    finalizer(
        &kube_api,
        "databricks-operator/remote_api_resource",
        resource.clone(),
        |e| async {
            match e {
                Event::Apply(res) => reconcile_apply(res, context.clone()).await,
                Event::Cleanup(res) => reconcile_delete(res, context.clone()).await,
            }
        },
    )
    .map_err(|e| e.into())
    .await
}

/// Implement this on the macroexpanded CRD type, against the SDK type
pub trait RemoteAPIResource<TAPIType: 'static> {
    fn controller(
        context: Arc<Context>,
    ) -> Pin<Box<dyn Stream<Item = Result<(ObjectRef<Self>, Action), DatabricksKubeError>> + Send>>
    where
        Self: From<TAPIType>,
        Self: Resource<Scope = NamespaceResourceScope> + ResourceExt + CustomResourceExt,
        Self::DynamicType: Clone + Debug + Default + Eq + Hash + Unpin,
        Self: RemoteAPIResource<TAPIType>,
        Self: Send,
        Self: Serialize,
        Self: Sync,
        Self: Default,
        Self: Clone,
        Self: CustomResourceExt,
        Self: Debug,
        Self: DeserializeOwned,
        Self: 'static,
        TAPIType: From<Self>,
        TAPIType: PartialEq,
        TAPIType: Send,
        TAPIType: Serialize,
        TAPIType: 'static,
    {
        let root_kind_api = Api::<Self>::default_namespaced(context.client.clone());

        Controller::new(root_kind_api.clone(), watcher::Config::default())
            .shutdown_on_signal()
            .run(
                reconcile,
                |res, err, _ctx| {
                    log::error!(
                        "API Sync failed for {} {} (retrying in 30s):\n{}",
                        Self::api_resource().kind,
                        res.name_unchecked(),
                        err,
                    );
                    Action::requeue(Duration::from_secs(30))
                },
                context.clone(),
            )
            .map_err(|e| DatabricksKubeError::ControllerError(e.to_string()))
            .boxed()
    }

    fn self_url_unchecked(&self) -> String
    where
        Self: Resource + ResourceExt,
        Self::DynamicType: Default + Eq + Hash,
    {
        let ns = self.namespace().unwrap();
        format!(
            "{}/{}",
            Self::url_path(&Default::default(), Some(&ns)),
            self.name_unchecked()
        )
    }

    fn every_reconcile(
        &self,
        _context: Arc<Context>,
    ) -> Pin<Box<dyn Future<Output = Result<(), DatabricksKubeError>> + Send>> {
        async { Ok(()) }.boxed()
    }

    fn remote_list_all(
        context: Arc<Context>,
    ) -> Pin<Box<dyn Stream<Item = Result<TAPIType, DatabricksKubeError>> + Send>>;

    fn remote_get(
        &self,
        context: Arc<Context>,
    ) -> Pin<Box<dyn Stream<Item = Result<TAPIType, DatabricksKubeError>> + Send>>;

    fn remote_create(
        &self,
        context: Arc<Context>,
    ) -> Pin<Box<dyn Stream<Item = Result<Self, DatabricksKubeError>> + Send + '_>>
    where
        Self: Sized;

    fn remote_update(
        &self,
        context: Arc<Context>,
    ) -> Pin<Box<dyn Stream<Item = Result<Self, DatabricksKubeError>> + Send + '_>>
    where
        Self: Sized;

    fn remote_delete(
        &self,
        context: Arc<Context>,
    ) -> Pin<Box<dyn Stream<Item = Result<(), DatabricksKubeError>> + Send + '_>>;
}
