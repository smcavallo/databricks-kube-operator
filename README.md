# databricks-kube-operator

A [kube-rs](https://kube.rs/) operator for Databricks APIs:

| API                 | CRD           |
|---------------------|---------------|
| Jobs 2.1            | DatabricksJob |
| Git Credentials 2.0 | GitCredential |
| Repos 2.0           | Repo          |

WIP and experimental!

## Getting Started

### Installation

Add the Helm repository:

```bash
helm repo add mach https://mach-kernel.github.io/databricks-kube-operator
helm install databricks-kube-operator mach/databricks-kube-operator
```

Create a config map in the same namespace as the operator:
```bash
cat <<EOF | kubectl apply -f -
apiVersion: v1
kind: ConfigMap
metadata:
  name: databricks-kube-operator
data:
  access_token: shhh
  databricks_url: https://my-tenant.cloud.databricks.com/api
EOF
```

### Usage

See the examples directory for how to create Databricks resource in Kube. Resources that are created via Kubernetes are owned by the operator -- meaning your checked-in manifests are the source of truth. It will not sync anything other than status back from the API, and overwrite changes made via the UI.

You may provide the `databricks-operator/owner` annotation as shown below (to be explicit). However, all resources created in Kube first (i.e. no associated API object found) are assumed to be owned by the operator. 

```yaml
apiVersion: com.dstancu.databricks/v1
kind: GitCredential
metadata:
  annotations:
    databricks-operator/owner: operator
  name: example-credential
  namespace: default
spec:
  secret_name: my-secret-name
  credential:
    git_username: mach-kernel
    git_provider: gitHub
```

By default, databricks-kube-operator will also sync existing API resources from Databricks into Kubernetes (goal: surface status). Resources owned by the API are tagged as such with an annotation on ingest:

```yaml
apiVersion: v1
items:
- apiVersion: com.dstancu.databricks/v1
  kind: DatabricksJob
  metadata:
    annotations:
      databricks-operator/owner: api
    creationTimestamp: "2022-11-04T21:46:12Z"
    generation: 1
    name: hello-world
    ...
```

## Developers

Begin by creating the configmap as per the Helm instructions.

Generate and install the CRDs by running the `crd_gen` bin target:
```bash
cargo run --bin crd_gen | kubectl apply -f -
```

The quickest way to test the operator is with a working [minikube](https://minikube.sigs.k8s.io/docs/start/) cluster:

```bash
minikube start
minikube tunnel &
```

```bash
export RUST_LOG=databricks_kube
cargo run
[2022-11-02T18:56:25Z INFO  databricks_kube] boot! (build: df7e26b-modified)
[2022-11-02T18:56:25Z INFO  databricks_kube::context] Waiting for CRD: databricksjobs.com.dstancu.databricks
[2022-11-02T18:56:25Z INFO  databricks_kube::context] Waiting for CRD: gitcredentials.com.dstancu.databricks
[2022-11-02T18:56:25Z INFO  databricks_kube::context] Waiting for settings in config map: databricks-kube-operator
[2022-11-02T18:56:25Z INFO  databricks_kube::context] Found config map
[2022-11-02T18:56:25Z INFO  databricks_kube::traits::synced_api_resource] Looking for uningested GitCredential(s)
[2022-11-02T18:56:25Z INFO  databricks_kube::traits::synced_api_resource] Looking for uningested DatabricksJob(s)
```

### Generating API Clients

The client is generated by `openapi-generator` and then lightly postprocessed so we get models that derive [`JsonSchema`](https://github.com/GREsau/schemars#basic-usage) and fix some bugs.

TODO: Fork or fix generator/template issues instead of sed.

```bash
# Hey!! This uses GNU sed
# brew install gnu-sed

# Jobs API
openapi-generator generate -g rust -i openapi/jobs-2.1-aws.yaml -c openapi/config-jobs.yaml -o dbr_jobs

# Derive JsonSchema for all models and add schemars as dep
gsed -i -e 's/derive(Clone/derive(JsonSchema, Clone/' dbr_jobs/src/models/*
gsed -i -e 's/\/\*/use schemars::JsonSchema;\n\/\*/' dbr_jobs/src/models/*
gsed -r -i -e 's/(\[dependencies\])/\1\nschemars = "0.8.11"/' dbr_jobs/Cargo.toml

# Missing import?
gsed -r -i -e 's/(use reqwest;)/\1\nuse crate::models::ViewsToExport;/' dbr/src/apis/default_api.rs

# Git Credentials API
openapi-generator generate -g rust -i openapi/gitcredentials-2.0-aws.yaml -c openapi/config-git.yaml -o dbr_git_creds

# Derive JsonSchema for all models and add schemars as dep
gsed -i -e 's/derive(Clone/derive(JsonSchema, Clone/' dbr_git_creds/src/models/*
gsed -i -e 's/\/\*/use schemars::JsonSchema;\n\/\*/' dbr_git_creds/src/models/*
gsed -r -i -e 's/(\[dependencies\])/\1\nschemars = "0.8.11"/' dbr_git_creds/Cargo.toml

# Repos API
openapi-generator generate -g rust -i openapi/repos-2.0-aws.yaml -c openapi/config-repos.yaml -o dbr_repo

# Derive JsonSchema for all models and add schemars as dep
gsed -i -e 's/derive(Clone/derive(JsonSchema, Clone/' dbr_repo/src/models/*
gsed -i -e 's/\/\*/use schemars::JsonSchema;\n\/\*/' dbr_repo/src/models/*
gsed -r -i -e 's/(\[dependencies\])/\1\nschemars = "0.8.11"/' dbr_repo/Cargo.toml
```

### Expand CRD macros

Deriving `CustomResource` uses macros to generate another struct. For this example, the output struct name would be `DatabricksJob`:

```rust
#[derive(Clone, CustomResource, Debug, Default, Deserialize, PartialEq, Serialize, JsonSchema)]
#[kube(
    group = "com.dstancu.databricks",
    version = "v1",
    kind = "DatabricksJob",
    derive = "Default",
    namespaced
)]
pub struct DatabricksJobSpec {
    pub job: Job,
}
```

`rust-analyzer` shows squiggles when you `use crds::databricks_job::DatabricksJob`, but one may want to look inside. To see what is generated with [cargo-expand](https://github.com/dtolnay/cargo-expand):

```bash
rustup default nightly
cargo expand --bin databricks_kube
```
