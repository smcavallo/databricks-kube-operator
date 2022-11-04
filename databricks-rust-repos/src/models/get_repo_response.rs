use schemars::JsonSchema;
/*
 * Repos API
 *
 * The repos API allows users to manage their [repos](https://docs.databricks.com/repos.html). Users can use the API to access all repos that they have manage permissions on.
 *
 * The version of the OpenAPI document: 2.0.0
 * 
 * Generated by: https://openapi-generator.tech
 */




#[derive(JsonSchema, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct GetRepoResponse {
    /// ID of the repo object in the workspace.
    #[serde(rename = "id", skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// URL of the Git repository to be linked.
    #[serde(rename = "url", skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Git provider. This field is case-insensitive. The available Git providers are gitHub, bitbucketCloud, gitLab, azureDevOpsServices, gitHubEnterprise, bitbucketServer, gitLabEnterpriseEdition and awsCodeCommit.
    #[serde(rename = "provider", skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Desired path for the repo in the workspace. Must be in the format /Repos/{folder}/{repo-name}.
    #[serde(rename = "path", skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Branch that the local version of the repo is checked out to.
    #[serde(rename = "branch", skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// SHA-1 hash representing the commit ID of the current HEAD of the repo.
    #[serde(rename = "head_commit_id", skip_serializing_if = "Option::is_none")]
    pub head_commit_id: Option<String>,
}

impl GetRepoResponse {
    pub fn new() -> GetRepoResponse {
        GetRepoResponse {
            id: None,
            url: None,
            provider: None,
            path: None,
            branch: None,
            head_commit_id: None,
        }
    }
}


