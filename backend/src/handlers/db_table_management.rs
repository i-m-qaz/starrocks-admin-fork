use axum::{Json, extract::{Path, State}, Extension};use std::sync::Arc;

use crate::AppState;
use crate::utils::ApiResult;
use serde::{Deserialize, Serialize};

// ==============================================
// Database Management
// ==============================================

// Database response structure
#[derive(Debug, Serialize)]
pub struct DatabaseResponse {
    pub name: String,
    pub comment: Option<String>,
    pub create_time: String,
    pub tables_count: i64,
}

// Create database request
#[derive(Debug, Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub comment: Option<String>,
    pub properties: Option<serde_json::Value>,
}

// Update database request
#[derive(Debug, Deserialize)]
pub struct UpdateDatabaseRequest {
    pub comment: Option<String>,
    pub properties: Option<serde_json::Value>,
}

// List databases
#[utoipa::path(
    get,
    path = "/api/clusters/databases",
    responses(
        (status = 200, description = "List of databases", body = Vec<DatabaseResponse>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database Management"
)]
pub async fn list_databases(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<Vec<DatabaseResponse>>> {
    tracing::debug!("Listing databases for user {}", org_ctx.user_id);
    
    let databases = state.cluster_service.list_databases().await?;
    Ok(Json(databases))
}

// Create database
#[utoipa::path(
    post,
    path = "/api/clusters/databases",
    request_body = CreateDatabaseRequest,
    responses(
        (status = 200, description = "Database created successfully", body = DatabaseResponse),
        (status = 400, description = "Bad request")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database Management"
)]
pub async fn create_database(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
    Json(req): Json<CreateDatabaseRequest>,
) -> ApiResult<Json<DatabaseResponse>> {
    tracing::info!("Creating database: {}", req.name);
    
    let database = state.cluster_service.create_database(req).await?;
    Ok(Json(database))
}

// Get database
#[utoipa::path(
    get,
    path = "/api/clusters/databases/{name}",
    params(
        ("name" = String, Path, description = "Database name")
    ),
    responses(
        (status = 200, description = "Database details", body = DatabaseResponse),
        (status = 404, description = "Database not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database Management"
)]
pub async fn get_database(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<DatabaseResponse>> {
    tracing::debug!("Getting database: {}", name);
    
    let database = state.cluster_service.get_database(&name).await?;
    Ok(Json(database))
}

// Update database
#[utoipa::path(
    put,
    path = "/api/clusters/databases/{name}",
    params(
        ("name" = String, Path, description = "Database name")
    ),
    request_body = UpdateDatabaseRequest,
    responses(
        (status = 200, description = "Database updated successfully", body = DatabaseResponse),
        (status = 404, description = "Database not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database Management"
)]
pub async fn update_database(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
    Json(req): Json<UpdateDatabaseRequest>,
) -> ApiResult<Json<DatabaseResponse>> {
    tracing::info!("Updating database: {}", name);
    
    let database = state.cluster_service.update_database(&name, req).await?;
    Ok(Json(database))
}

// Delete database
#[utoipa::path(
    delete,
    path = "/api/clusters/databases/{name}",
    params(
        ("name" = String, Path, description = "Database name")
    ),
    responses(
        (status = 200, description = "Database deleted successfully"),
        (status = 404, description = "Database not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Database Management"
)]
pub async fn delete_database(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::warn!("Deleting database: {}", name);
    
    state.cluster_service.delete_database(&name).await?;
    Ok(Json(serde_json::json!({"message": "Database deleted successfully"})))
}

// ==============================================
// Table Management
// ==============================================

// Table response structure
#[derive(Debug, Serialize)]
pub struct TableResponse {
    pub database: String,
    pub name: String,
    pub table_type: String,
    pub engine: String,
    pub create_time: String,
    pub rows: Option<i64>,
    pub data_size: Option<i64>,
    pub comment: Option<String>,
}

// Table detail structure
#[derive(Debug, Serialize)]
pub struct TableDetailResponse {
    pub database: String,
    pub name: String,
    pub table_type: String,
    pub engine: String,
    pub create_time: String,
    pub rows: Option<i64>,
    pub data_size: Option<i64>,
    pub comment: Option<String>,
    pub columns: Vec<ColumnInfo>,
    pub partitions: Option<Vec<PartitionInfo>>,
    pub buckets: Option<BucketInfo>,
}

// Column information
#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub comment: Option<String>,
    pub default_value: Option<String>,
    pub is_nullable: bool,
}

// Partition information
#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionInfo {
    pub name: String,
    pub values: String,
    pub status: String,
    pub data_size: Option<i64>,
    pub rows: Option<i64>,
}

// Bucket information
#[derive(Debug, Serialize, Deserialize)]
pub struct BucketInfo {
    pub bucket_type: String,
    pub bucket_keys: Vec<String>,
    pub bucket_count: i32,
}

// Create table request
#[derive(Debug, Deserialize)]
pub struct CreateTableRequest {
    pub database: String,
    pub name: String,
    pub table_type: String, // duplicate, aggregate, unique, primary, update
    pub columns: Vec<ColumnDefinition>,
    pub partition_info: Option<PartitionDefinition>,
    pub bucket_info: Option<BucketDefinition>,
    pub comment: Option<String>,
    pub properties: Option<serde_json::Value>,
}

// Column definition
#[derive(Debug, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    pub comment: Option<String>,
    pub default_value: Option<String>,
    pub is_nullable: bool,
    pub is_key: bool,
    pub aggregate_type: Option<String>, // for aggregate table
}

// Partition definition
#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionDefinition {
    pub partition_type: String, // range, list, expression
    pub partition_key: String,
    pub partitions: Vec<PartitionSpec>,
}

// Partition spec
#[derive(Debug, Serialize, Deserialize)]
pub struct PartitionSpec {
    pub name: String,
    pub values: String,
}

// Bucket definition
#[derive(Debug, Serialize, Deserialize)]
pub struct BucketDefinition {
    pub bucket_type: String, // hash, range
    pub bucket_keys: Vec<String>,
    pub bucket_count: i32,
}

// Update table request
#[derive(Debug, Deserialize)]
pub struct UpdateTableRequest {
    pub columns: Option<Vec<ColumnDefinition>>,
    pub partition_info: Option<PartitionDefinition>,
    pub bucket_info: Option<BucketDefinition>,
    pub comment: Option<String>,
    pub properties: Option<serde_json::Value>,
}

// Table action request
#[derive(Debug, Deserialize)]
pub struct TableActionRequest {
    pub action: String, // truncate, optimize, repair
    pub parameters: Option<serde_json::Value>,
}

// List tables
#[utoipa::path(
    get,
    path = "/api/clusters/tables",
    responses(
        (status = 200, description = "List of tables", body = Vec<TableResponse>)
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Table Management"
)]
pub async fn list_tables(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<Vec<TableResponse>>> {
    tracing::debug!("Listing tables for user {}", org_ctx.user_id);
    
    let tables = state.cluster_service.list_tables().await?;
    Ok(Json(tables))
}

// Create table
#[utoipa::path(
    post,
    path = "/api/clusters/tables",
    request_body = CreateTableRequest,
    responses(
        (status = 200, description = "Table created successfully", body = TableResponse),
        (status = 400, description = "Bad request")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Table Management"
)]
pub async fn create_table(
    State(state): State<Arc<AppState>>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
    Json(req): Json<CreateTableRequest>,
) -> ApiResult<Json<TableResponse>> {
    tracing::info!("Creating table: {}.{}", req.database, req.name);
    
    let table = state.cluster_service.create_table(req).await?;
    Ok(Json(table))
}

// Get table
#[utoipa::path(
    get,
    path = "/api/clusters/tables/{database}/{table}",
    params(
        ("database" = String, Path, description = "Database name"),
        ("table" = String, Path, description = "Table name")
    ),
    responses(
        (status = 200, description = "Table details", body = TableDetailResponse),
        (status = 404, description = "Table not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Table Management"
)]
pub async fn get_table(
    State(state): State<Arc<AppState>>,
    Path((database, table)): Path<(String, String)>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<TableDetailResponse>> {
    tracing::debug!("Getting table: {}.{}", database, table);
    
    let table_detail = state.cluster_service.get_table(&database, &table).await?;
    Ok(Json(table_detail))
}

// Update table
#[utoipa::path(
    put,
    path = "/api/clusters/tables/{database}/{table}",
    params(
        ("database" = String, Path, description = "Database name"),
        ("table" = String, Path, description = "Table name")
    ),
    request_body = UpdateTableRequest,
    responses(
        (status = 200, description = "Table updated successfully", body = TableResponse),
        (status = 404, description = "Table not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Table Management"
)]
pub async fn update_table(
    State(state): State<Arc<AppState>>,
    Path((database, table)): Path<(String, String)>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
    Json(req): Json<UpdateTableRequest>,
) -> ApiResult<Json<TableResponse>> {
    tracing::info!("Updating table: {}.{}", database, table);
    
    let table = state.cluster_service.update_table(&database, &table, req).await?;
    Ok(Json(table))
}

// Delete table
#[utoipa::path(
    delete,
    path = "/api/clusters/tables/{database}/{table}",
    params(
        ("database" = String, Path, description = "Database name"),
        ("table" = String, Path, description = "Table name")
    ),
    responses(
        (status = 200, description = "Table deleted successfully"),
        (status = 404, description = "Table not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Table Management"
)]
pub async fn delete_table(
    State(state): State<Arc<AppState>>,
    Path((database, table)): Path<(String, String)>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::warn!("Deleting table: {}.{}", database, table);
    
    state.cluster_service.delete_table(&database, &table).await?;
    Ok(Json(serde_json::json!({"message": "Table deleted successfully"})))
}

// Execute table action
#[utoipa::path(
    post,
    path = "/api/clusters/tables/{database}/{table}/action",
    params(
        ("database" = String, Path, description = "Database name"),
        ("table" = String, Path, description = "Table name")
    ),
    request_body = TableActionRequest,
    responses(
        (status = 200, description = "Table action executed successfully"),
        (status = 404, description = "Table not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Table Management"
)]
pub async fn execute_table_action(
    State(state): State<Arc<AppState>>,
    Path((database, table)): Path<(String, String)>,
    Extension(org_ctx): Extension<crate::middleware::OrgContext>,
    Json(req): Json<TableActionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::info!("Executing action '{}' on table: {}.{}", req.action, database, table);
    
    state.cluster_service.execute_table_action(&database, &table, req).await?;
    Ok(Json(serde_json::json!({"message": "Table action executed successfully"})))
}