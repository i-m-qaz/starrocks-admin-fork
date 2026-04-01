use crate::handlers::db_table_management::{    ColumnDefinition, ColumnInfo, CreateDatabaseRequest, CreateTableRequest, DatabaseResponse,    PartitionDefinition, PartitionInfo, PartitionSpec, TableActionRequest, TableDetailResponse,    TableResponse, UpdateDatabaseRequest, UpdateTableRequest, BucketDefinition, BucketInfo,};use crate::models::{    Cluster, ClusterHealth, CreateClusterRequest, HealthCheck, HealthStatus, UpdateClusterRequest,};
use crate::services::{MySQLPoolManager, StarRocksClient};
use crate::utils::{ApiError, ApiResult};
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClusterService {
    pool: SqlitePool,
    mysql_pool_manager: Arc<MySQLPoolManager>,
}

impl ClusterService {
    pub fn new(pool: SqlitePool, mysql_pool_manager: Arc<MySQLPoolManager>) -> Self {
        Self { pool, mysql_pool_manager }
    }

    // Create a new cluster
    pub async fn create_cluster(
        &self,
        mut req: CreateClusterRequest,
        user_id: i64,
        requestor_org: Option<i64>,
        is_super_admin: bool,
    ) -> ApiResult<Cluster> {
        // Clean input data - trim whitespace
        req.name = req.name.trim().to_string();
        req.fe_host = req.fe_host.trim().to_string();
        req.username = req.username.trim().to_string();
        req.catalog = req.catalog.trim().to_string();
        if let Some(ref mut desc) = req.description {
            *desc = desc.trim().to_string();
        }

        // Validate cleaned data
        if req.name.is_empty() {
            return Err(ApiError::validation_error("Cluster name cannot be empty"));
        }
        if req.fe_host.is_empty() {
            return Err(ApiError::validation_error("FE host cannot be empty"));
        }
        if req.username.is_empty() {
            return Err(ApiError::validation_error("Username cannot be empty"));
        }

        // Check if cluster name already exists
        let existing: Option<Cluster> = sqlx::query_as("SELECT * FROM clusters WHERE name = ?")
            .bind(&req.name)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            return Err(ApiError::validation_error("Cluster name already exists"));
        }

        let target_org_id = self
            .resolve_target_org(req.organization_id, requestor_org, is_super_admin)
            .await?;

        // Convert tags to JSON string
        let tags_json = req
            .tags
            .map(|t| serde_json::to_string(&t).unwrap_or_default());

        // Check if this will be the first cluster within the organization
        let existing_cluster_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM clusters WHERE organization_id = ?")
                .bind(target_org_id)
                .fetch_one(&self.pool)
                .await?;

        let is_first_cluster = existing_cluster_count.0 == 0;

        // Insert cluster (password is stored as-is for now, should be encrypted in production)
        let result = sqlx::query(
            "INSERT INTO clusters (name, description, fe_host, fe_http_port, fe_query_port, 
             username, password_encrypted, enable_ssl, connection_timeout, tags, catalog, 
             is_active, created_by, organization_id, deployment_mode)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&req.name)
        .bind(&req.description)
        .bind(&req.fe_host)
        .bind(req.fe_http_port)
        .bind(req.fe_query_port)
        .bind(&req.username)
        .bind(&req.password) // TODO: Encrypt in production
        .bind(req.enable_ssl)
        .bind(req.connection_timeout)
        .bind(&tags_json)
        .bind(&req.catalog)
        .bind(if is_first_cluster { 1 } else { 0 }) // Set as active if first cluster in org
        .bind(user_id)
        .bind(target_org_id)
        .bind(req.deployment_mode.to_string())
        .execute(&self.pool)
        .await?;

        let cluster_id = result.last_insert_rowid();

        // Ensure each organization always has one active cluster if none exists
        if !is_first_cluster {
            let active_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM clusters WHERE is_active = 1 AND organization_id = ?",
            )
            .bind(target_org_id)
            .fetch_one(&self.pool)
            .await?;

            if active_count.0 == 0 {
                sqlx::query("UPDATE clusters SET is_active = 1 WHERE id = ?")
                    .bind(cluster_id)
                    .execute(&self.pool)
                    .await?;
                tracing::info!(
                    "Automatically activated newly created cluster for organization {} (no active cluster existed)",
                    target_org_id
                );
            }
        }

        // Fetch and return the created cluster
        let cluster: Cluster = sqlx::query_as("SELECT * FROM clusters WHERE id = ?")
            .bind(cluster_id)
            .fetch_one(&self.pool)
            .await?;

        tracing::info!("Cluster created successfully: {} (ID: {})", cluster.name, cluster.id);
        tracing::debug!(
            "Cluster details: host={}, port={}, ssl={}, catalog={}, active={}",
            cluster.fe_host,
            cluster.fe_http_port,
            cluster.enable_ssl,
            cluster.catalog,
            cluster.is_active
        );

        Ok(cluster)
    }

    // Get all clusters (unfiltered, handlers apply org filter)
    pub async fn list_clusters(&self) -> ApiResult<Vec<Cluster>> {
        let clusters: Vec<Cluster> =
            sqlx::query_as("SELECT * FROM clusters ORDER BY created_at DESC")
                .fetch_all(&self.pool)
                .await?;

        Ok(clusters)
    }

    // Get cluster by ID
    pub async fn get_cluster(&self, cluster_id: i64) -> ApiResult<Cluster> {
        let cluster: Option<Cluster> = sqlx::query_as("SELECT * FROM clusters WHERE id = ?")
            .bind(cluster_id)
            .fetch_optional(&self.pool)
            .await?;

        cluster.ok_or_else(|| ApiError::cluster_not_found(cluster_id))
    }

    // Get the currently active cluster (global)
    pub async fn get_active_cluster(&self) -> ApiResult<Cluster> {
        let cluster: Option<Cluster> =
            sqlx::query_as("SELECT * FROM clusters WHERE is_active = 1 LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        cluster.ok_or_else(|| {
            ApiError::not_found("No active cluster found. Please activate a cluster first.")
        })
    }

    // Get active cluster scoped by organization
    pub async fn get_active_cluster_by_org(&self, org_id: Option<i64>) -> ApiResult<Cluster> {
        let cluster: Option<Cluster> = if let Some(org) = org_id {
            sqlx::query_as(
                "SELECT * FROM clusters WHERE is_active = 1 AND organization_id = ? LIMIT 1",
            )
            .bind(org)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        cluster.ok_or_else(|| {
            ApiError::not_found(
                "No active cluster found for your organization. Please activate a cluster first.",
            )
        })
    }

    // Set a cluster as active (deactivating all others in the same organization)
    pub async fn set_active_cluster(&self, cluster_id: i64) -> ApiResult<Cluster> {
        // Check if cluster exists and fetch its org
        let cluster = self.get_cluster(cluster_id).await?;
        let org_id = cluster.organization_id;

        // Start transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;

        // First, deactivate all clusters in the same organization
        if let Some(org) = org_id {
            sqlx::query("UPDATE clusters SET is_active = 0 WHERE organization_id = ?")
                .bind(org)
                .execute(&mut *tx)
                .await?;
        } else {
            // If cluster has no org, deactivate all clusters with NULL org
            sqlx::query("UPDATE clusters SET is_active = 0 WHERE organization_id IS NULL")
                .execute(&mut *tx)
                .await?;
        }

        // Then, activate the target cluster
        sqlx::query(
            "UPDATE clusters SET is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(cluster_id)
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        tracing::info!("Cluster activated: ID {} (org: {:?})", cluster_id, org_id);

        // Fetch and return the updated cluster
        self.get_cluster(cluster_id).await
    }

    // Update cluster
    pub async fn update_cluster(
        &self,
        cluster_id: i64,
        req: UpdateClusterRequest,
    ) -> ApiResult<Cluster> {
        // Check if cluster exists
        let _cluster = self.get_cluster(cluster_id).await?;

        // Build dynamic SQL update query
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(name) = &req.name {
            updates.push("name = ?");
            params.push(name.clone());
        }
        if let Some(desc) = &req.description {
            updates.push("description = ?");
            params.push(desc.clone());
        }
        if let Some(host) = &req.fe_host {
            updates.push("fe_host = ?");
            params.push(host.clone());
        }
        if let Some(http_port) = req.fe_http_port {
            updates.push("fe_http_port = ?");
            params.push(http_port.to_string());
        }
        if let Some(query_port) = req.fe_query_port {
            updates.push("fe_query_port = ?");
            params.push(query_port.to_string());
        }
        if let Some(username) = &req.username {
            updates.push("username = ?");
            params.push(username.clone());
        }
        if let Some(password) = &req.password {
            updates.push("password_encrypted = ?");
            params.push(password.clone());
        }
        if let Some(ssl) = req.enable_ssl {
            updates.push("enable_ssl = ?");
            params.push((ssl as i32).to_string());
        }
        if let Some(timeout) = req.connection_timeout {
            updates.push("connection_timeout = ?");
            params.push(timeout.to_string());
        }
        if let Some(tags) = &req.tags {
            updates.push("tags = ?");
            params.push(serde_json::to_string(tags).unwrap_or_default());
        }
        if let Some(catalog) = &req.catalog {
            updates.push("catalog = ?");
            params.push(catalog.clone());
        }
        if let Some(org_id) = req.organization_id {
            updates.push("organization_id = ?");
            params.push(org_id.to_string());
        }
        if let Some(mode) = &req.deployment_mode {
            updates.push("deployment_mode = ?");
            params.push(mode.to_string());
        }

        if updates.is_empty() {
            return self.get_cluster(cluster_id).await;
        }

        updates.push("updated_at = CURRENT_TIMESTAMP");

        let sql = format!("UPDATE clusters SET {} WHERE id = ?", updates.join(", "));

        let mut query = sqlx::query(&sql);
        for param in params {
            query = query.bind(param);
        }
        query = query.bind(cluster_id);

        query.execute(&self.pool).await?;

        tracing::info!("Cluster updated: ID {}", cluster_id);

        self.get_cluster(cluster_id).await
    }

    // Delete cluster
    pub async fn delete_cluster(&self, cluster_id: i64) -> ApiResult<()> {
        // Check if this is the active cluster and capture organization
        let cluster_record: Option<(bool, Option<i64>)> =
            sqlx::query_as("SELECT is_active, organization_id FROM clusters WHERE id = ?")
                .bind(cluster_id)
                .fetch_optional(&self.pool)
                .await?;

        let is_active = cluster_record.map(|r| r.0).unwrap_or(false);
        let cluster_org_id = cluster_record.and_then(|r| r.1);

        // Delete the cluster
        let result = sqlx::query("DELETE FROM clusters WHERE id = ?")
            .bind(cluster_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(ApiError::cluster_not_found(cluster_id));
        }

        tracing::info!("Cluster deleted: ID {}", cluster_id);

        // If we deleted the active cluster, activate another one (first by creation time)
        if is_active {
            let next_cluster: Option<(i64,)> = if let Some(org_id) = cluster_org_id {
                sqlx::query_as(
                    "SELECT id FROM clusters WHERE organization_id = ? ORDER BY created_at DESC LIMIT 1",
                )
                .bind(org_id)
                .fetch_optional(&self.pool)
                .await?
            } else {
                sqlx::query_as(
                    "SELECT id FROM clusters WHERE organization_id IS NULL ORDER BY created_at DESC LIMIT 1",
                )
                .fetch_optional(&self.pool)
                .await?
            };

            if let Some((next_id,)) = next_cluster {
                sqlx::query("UPDATE clusters SET is_active = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(next_id)
                    .execute(&self.pool)
                    .await?;
                tracing::info!("Automatically activated cluster ID {} after deletion", next_id);
            }
        }

        Ok(())
    }

    async fn resolve_target_org(
        &self,
        requested_org: Option<i64>,
        requestor_org: Option<i64>,
        is_super_admin: bool,
    ) -> ApiResult<i64> {
        if is_super_admin {
            if let Some(id) = requested_org.or(requestor_org) {
                return Ok(id);
            }
            return self.fetch_default_org_id().await;
        }

        requestor_org.ok_or_else(|| {
            ApiError::forbidden("Organization context required for cluster operations")
        })
    }

    async fn fetch_default_org_id(&self) -> ApiResult<i64> {
        if let Some(id) =
            sqlx::query_scalar("SELECT id FROM organizations WHERE code = 'default_org'")
                .fetch_optional(&self.pool)
                .await?
        {
            return Ok(id);
        }

        sqlx::query("INSERT INTO organizations (code, name, description, is_system) VALUES ('default_org', 'Default Organization', 'Auto-created default organization', 1)")
            .execute(&self.pool)
            .await?;

        sqlx::query_scalar("SELECT id FROM organizations WHERE code = 'default_org'")
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| ApiError::not_found("Default organization not found"))
    }

    // Get cluster health
    pub async fn get_cluster_health(&self, cluster_id: i64) -> ApiResult<ClusterHealth> {
        let cluster = self.get_cluster(cluster_id).await?;
        let is_shared_data = cluster.is_shared_data();
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let mut checks = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        match client.get_frontends().await {
            Ok(frontends) => {
                let alive_count = frontends.iter().filter(|f| f.alive == "true").count();
                let total_count = frontends.len();

                if total_count == 0 {
                    checks.push(HealthCheck {
                        name: "Frontend Nodes".to_string(),
                        status: "critical".to_string(),
                        message: "No FE nodes found".to_string(),
                    });
                    overall_status = HealthStatus::Critical;
                } else if alive_count == total_count {
                    checks.push(HealthCheck {
                        name: "Frontend Nodes".to_string(),
                        status: "ok".to_string(),
                        message: format!("All {} FE nodes are online", total_count),
                    });
                } else if alive_count > 0 {
                    checks.push(HealthCheck {
                        name: "Frontend Nodes".to_string(),
                        status: "warning".to_string(),
                        message: format!("{}/{} FE nodes are online", alive_count, total_count),
                    });
                    if overall_status == HealthStatus::Healthy {
                        overall_status = HealthStatus::Warning;
                    }
                } else {
                    checks.push(HealthCheck {
                        name: "Frontend Nodes".to_string(),
                        status: "critical".to_string(),
                        message: "No FE nodes are online".to_string(),
                    });
                    overall_status = HealthStatus::Critical;
                }
            },
            Err(e) => {
                checks.push(HealthCheck {
                    name: "Frontend Nodes".to_string(),
                    status: "critical".to_string(),
                    message: format!("Failed to check FE nodes: {}", e),
                });
                overall_status = HealthStatus::Critical;
            },
        }

        // Check compute nodes (BE in shared-nothing, CN in shared-data)
        let node_type = if is_shared_data { "CN" } else { "BE" };
        match client.get_backends().await {
            Ok(backends) => {
                let alive_count = backends.iter().filter(|b| b.alive == "true").count();
                let total_count = backends.len();

                if alive_count == total_count {
                    checks.push(HealthCheck {
                        name: "Compute Nodes".to_string(),
                        status: "ok".to_string(),
                        message: format!("All {} {} nodes are online", total_count, node_type),
                    });
                } else if alive_count > 0 {
                    checks.push(HealthCheck {
                        name: "Compute Nodes".to_string(),
                        status: "warning".to_string(),
                        message: format!(
                            "{}/{} {} nodes are online",
                            alive_count, total_count, node_type
                        ),
                    });
                    if overall_status == HealthStatus::Healthy {
                        overall_status = HealthStatus::Warning;
                    }
                } else {
                    checks.push(HealthCheck {
                        name: "Compute Nodes".to_string(),
                        status: "critical".to_string(),
                        message: format!("No {} nodes are online", node_type),
                    });
                    overall_status = HealthStatus::Critical;
                }
            },
            Err(e) => {
                checks.push(HealthCheck {
                    name: "Compute Nodes".to_string(),
                    status: "warning".to_string(),
                    message: format!("Failed to check {} nodes: {}", node_type, e),
                });
                if overall_status == HealthStatus::Healthy {
                    overall_status = HealthStatus::Warning;
                }
            },
        }

        Ok(ClusterHealth { status: overall_status, checks, last_check_time: Utc::now() })
    }

    // Get cluster health for a specific cluster object (used for testing new clusters)
    pub async fn get_cluster_health_for_cluster(
        &self,
        cluster: &Cluster,
    ) -> ApiResult<ClusterHealth> {
        use crate::services::MySQLClient;

        let mut checks = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        // Check connection by getting pool
        match self.mysql_pool_manager.get_pool(cluster).await {
            Ok(pool) => {
                let mysql_client = MySQLClient::from_pool(pool);

                // Test basic connection
                match mysql_client.query("SELECT 1").await {
                    Ok(_) => {
                        checks.push(HealthCheck {
                            name: "Database Connection".to_string(),
                            status: "ok".to_string(),
                            message: "Connection successful".to_string(),
                        });

                        // Try to check FE availability via HTTP
                        let client = StarRocksClient::new(cluster.clone(), self.mysql_pool_manager.clone());
                        match client.get_runtime_info().await {
                            Ok(_) => {
                                checks.push(HealthCheck {
                                    name: "FE Availability".to_string(),
                                    status: "ok".to_string(),
                                    message: "FE is reachable and responding".to_string(),
                                });
                            },
                            Err(e) => {
                                checks.push(HealthCheck {
                                    name: "FE Availability".to_string(),
                                    status: "warning".to_string(),
                                    message: format!("FE HTTP check failed: {}", e),
                                });
                                if overall_status == HealthStatus::Healthy {
                                    overall_status = HealthStatus::Warning;
                                }
                            },
                        }

                        // Try to check compute nodes (BE in shared-nothing, CN in shared-data)
                        let node_type = if cluster.is_shared_data() { "CN" } else { "BE" };
                        match client.get_backends().await {
                            Ok(backends) => {
                                let alive_count = backends.iter().filter(|b| b.alive == "true").count();
                                let total_count = backends.len();

                                if total_count == 0 {
                                    checks.push(HealthCheck {
                                        name: "Compute Nodes".to_string(),
                                        status: "warning".to_string(),
                                        message: format!("No {} nodes found", node_type),
                                    });
                                    if overall_status == HealthStatus::Healthy {
                                        overall_status = HealthStatus::Warning;
                                    }
                                } else if alive_count == total_count {
                                    checks.push(HealthCheck {
                                        name: "Compute Nodes".to_string(),
                                        status: "ok".to_string(),
                                        message: format!("All {} {} nodes are online", total_count, node_type),
                                    });
                                } else if alive_count > 0 {
                                    checks.push(HealthCheck {
                                        name: "Compute Nodes".to_string(),
                                        status: "warning".to_string(),
                                        message: format!("{}/{}{} nodes are online", alive_count, total_count, node_type),
                                    });
                                    if overall_status == HealthStatus::Healthy {
                                        overall_status = HealthStatus::Warning;
                                    }
                                } else {
                                    checks.push(HealthCheck {
                                        name: "Compute Nodes".to_string(),
                                        status: "critical".to_string(),
                                        message: format!("No {} nodes are online", node_type),
                                    });
                                    overall_status = HealthStatus::Critical;
                                }
                            },
                            Err(e) => {
                                checks.push(HealthCheck {
                                    name: "Compute Nodes".to_string(),
                                    status: "warning".to_string(),
                                    message: format!("Failed to check {} nodes: {}", node_type, e),
                                });
                                if overall_status == HealthStatus::Healthy {
                                    overall_status = HealthStatus::Warning;
                                }
                            },
                        }
                    },
                    Err(e) => {
                        checks.push(HealthCheck {
                            name: "Database Connection".to_string(),
                            status: "critical".to_string(),
                            message: format!("Connection failed: {}", e),
                        });
                        overall_status = HealthStatus::Critical;
                    },
                }
            },
            Err(e) => {
                checks.push(HealthCheck {
                    name: "Connection Pool".to_string(),
                    status: "critical".to_string(),
                    message: format!("Failed to create connection pool: {}", e),
                });
                overall_status = HealthStatus::Critical;
            },
        }

        Ok(ClusterHealth { status: overall_status, checks, last_check_time: Utc::now() })
    }

    // ==============================================
    // Database Management
    // ==============================================

    pub async fn list_databases(&self) -> ApiResult<Vec<DatabaseResponse>> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let sql = "SHOW DATABASES";
        let result = client.show_proc_raw(sql).await?;

        let mut databases = Vec::new();
        for row in result {
            if let Some(name) = row.get(0).and_then(|v| v.as_str()) {
                // Skip system databases
                if name == "information_schema" || name == "performance_schema" || name == "mysql" {
                    continue;
                }

                let database = self.get_database(name).await?;
                databases.push(database);
            }
        }

        Ok(databases)
    }

    pub async fn create_database(&self, req: CreateDatabaseRequest) -> ApiResult<DatabaseResponse> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let mut sql = format!("CREATE DATABASE IF NOT EXISTS {}", req.name);

        if let Some(comment) = req.comment {
            sql.push_str(&format!(" COMMENT '{}'", comment));
        }

        if let Some(properties) = req.properties {
            if let Some(props) = properties.as_object() {
                if !props.is_empty() {
                    sql.push_str(" PROPERTIES (");
                    let mut props_str = Vec::new();
                    for (key, value) in props {
                        if let Some(v) = value.as_str() {
                            props_str.push(format!("'{}' = '{}'", key, v));
                        }
                    }
                    sql.push_str(&props_str.join(", "));
                    sql.push_str(")");
                }
            }
        }

        client.execute_sql(&sql).await?;
        self.get_database(&req.name).await
    }

    pub async fn get_database(&self, name: &str) -> ApiResult<DatabaseResponse> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        // Get database comment and create time
        let sql = format!("SHOW CREATE DATABASE {}", name);
        let create_result = client.show_proc_raw(&sql).await?;

        let mut comment = None;
        let mut create_time = "".to_string();

        if let Some(row) = create_result.first() {
            if let Some(create_stmt) = row.get(1).and_then(|v| v.as_str()) {
                // Extract comment from CREATE DATABASE statement
                if let Some(comment_match) = create_stmt.match_indices("COMMENT").next() {
                    let rest = &create_stmt[comment_match.0..];
                    if let Some(start) = rest.find("'") {
                        if let Some(end) = rest[start + 1..].find("'") {
                            comment = Some(rest[start + 1..start + 1 + end].to_string());
                        }
                    }
                }
            }
        }

        // Get tables count
        let tables_sql = format!("SHOW TABLES FROM {}", name);
        let tables_result = client.show_proc_raw(&tables_sql).await?;
        let tables_count = tables_result.len() as i64;

        Ok(DatabaseResponse {
            name: name.to_string(),
            comment,
            create_time,
            tables_count,
        })
    }

    pub async fn update_database(&self, name: &str, req: UpdateDatabaseRequest) -> ApiResult<DatabaseResponse> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let mut sql = format!("ALTER DATABASE {}", name);

        if let Some(comment) = req.comment {
            sql.push_str(&format!(" COMMENT '{}'", comment));
        }

        if let Some(properties) = req.properties {
            if let Some(props) = properties.as_object() {
                if !props.is_empty() {
                    sql.push_str(" SET PROPERTIES (");
                    let mut props_str = Vec::new();
                    for (key, value) in props {
                        if let Some(v) = value.as_str() {
                            props_str.push(format!("'{}' = '{}'", key, v));
                        }
                    }
                    sql.push_str(&props_str.join(", "));
                    sql.push_str(")");
                }
            }
        }

        client.execute_sql(&sql).await?;
        self.get_database(name).await
    }

    pub async fn delete_database(&self, name: &str) -> ApiResult<()> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let sql = format!("DROP DATABASE IF EXISTS {}", name);
        client.execute_sql(&sql).await?;

        Ok(())
    }

    // ==============================================
    // Table Management
    // ==============================================

    pub async fn list_tables(&self) -> ApiResult<Vec<TableResponse>> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let sql = "SHOW DATABASES";
        let databases_result = client.show_proc_raw(sql).await?;

        let mut tables = Vec::new();
        for db_row in databases_result {
            if let Some(db_name) = db_row.get(0).and_then(|v| v.as_str()) {
                // Skip system databases
                if db_name == "information_schema" || db_name == "performance_schema" || db_name == "mysql" {
                    continue;
                }

                let tables_sql = format!("SHOW TABLES FROM {}", db_name);
                let tables_result = client.show_proc_raw(&tables_sql).await?;

                for table_row in tables_result {
                    if let Some(table_name) = table_row.get(0).and_then(|v| v.as_str()) {
                        let table = self.get_table(db_name, table_name).await?;
                        tables.push(TableResponse {
                            database: db_name.to_string(),
                            name: table.name,
                            table_type: table.table_type,
                            engine: table.engine,
                            create_time: table.create_time,
                            rows: table.rows,
                            data_size: table.data_size,
                            comment: table.comment,
                        });
                    }
                }
            }
        }

        Ok(tables)
    }

    pub async fn create_table(&self, req: CreateTableRequest) -> ApiResult<TableResponse> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let mut sql = format!("CREATE TABLE {}.{}", req.database, req.name);

        // Add table type
        match req.table_type.as_str() {
            "duplicate" => sql.push_str(" ("),
            "aggregate" => sql.push_str(" AGGREGATE TABLE ("),
            "unique" => sql.push_str(" UNIQUE TABLE ("),
            "primary" => sql.push_str(" PRIMARY KEY TABLE ("),
            "update" => sql.push_str(" UPDATE TABLE ("),
            _ => return Err(ApiError::validation_error("Invalid table type")),
        }

        // Add columns
        let mut columns = Vec::new();
        for col in req.columns {
            let mut col_def = format!("{} {}", col.name, col.data_type);
            if !col.is_nullable {
                col_def.push_str(" NOT NULL");
            }
            if let Some(default) = col.default_value {
                col_def.push_str(&format!(" DEFAULT {}", default));
            }
            if let Some(comment) = col.comment {
                col_def.push_str(&format!(" COMMENT '{}'", comment));
            }
            if let Some(aggregate) = col.aggregate_type {
                col_def.push_str(&format!(" {}", aggregate));
            }
            columns.push(col_def);
        }
        sql.push_str(&columns.join(", "));
        sql.push_str(")");

        // Add partition info
        if let Some(partition_info) = req.partition_info {
            sql.push_str(&format!(" PARTITION BY {} ({})", partition_info.partition_type, partition_info.partition_key));
            if !partition_info.partitions.is_empty() {
                sql.push_str(" PARTITIONS (");
                let mut partitions = Vec::new();
                for part in partition_info.partitions {
                    partitions.push(format!("PARTITION {} VALUES ({})", part.name, part.values));
                }
                sql.push_str(&partitions.join(", "));
                sql.push_str(")");
            }
        }

        // Add bucket info
        if let Some(bucket_info) = req.bucket_info {
            sql.push_str(&format!(" DISTRIBUTED BY {} ({}) BUCKETS {}", bucket_info.bucket_type, bucket_info.bucket_keys.join(", "), bucket_info.bucket_count));
        }

        // Add comment
        if let Some(comment) = req.comment {
            sql.push_str(&format!(" COMMENT '{}'", comment));
        }

        // Add properties
        if let Some(properties) = req.properties {
            if let Some(props) = properties.as_object() {
                if !props.is_empty() {
                    sql.push_str(" PROPERTIES (");
                    let mut props_str = Vec::new();
                    for (key, value) in props {
                        if let Some(v) = value.as_str() {
                            props_str.push(format!("'{}' = '{}'", key, v));
                        }
                    }
                    sql.push_str(&props_str.join(", "));
                    sql.push_str(")");
                }
            }
        }

        client.execute_sql(&sql).await?;

        let table_detail = self.get_table(&req.database, &req.name).await?;
        Ok(TableResponse {
            database: table_detail.database,
            name: table_detail.name,
            table_type: table_detail.table_type,
            engine: table_detail.engine,
            create_time: table_detail.create_time,
            rows: table_detail.rows,
            data_size: table_detail.data_size,
            comment: table_detail.comment,
        })
    }

    pub async fn get_table(&self, database: &str, table: &str) -> ApiResult<TableDetailResponse> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        // Get table basic info
        let sql = format!("SHOW CREATE TABLE {}.{}", database, table);
        let create_result = client.show_proc_raw(&sql).await?;

        let mut table_type = "duplicate".to_string();
        let mut engine = "OLAP".to_string();
        let mut create_time = "".to_string();
        let mut comment = None;

        if let Some(row) = create_result.first() {
            if let Some(create_stmt) = row.get(1).and_then(|v| v.as_str()) {
                // Extract table type
                if create_stmt.contains("AGGREGATE TABLE") {
                    table_type = "aggregate".to_string();
                } else if create_stmt.contains("UNIQUE TABLE") {
                    table_type = "unique".to_string();
                } else if create_stmt.contains("PRIMARY KEY TABLE") {
                    table_type = "primary".to_string();
                } else if create_stmt.contains("UPDATE TABLE") {
                    table_type = "update".to_string();
                }

                // Extract comment
                if let Some(comment_match) = create_stmt.match_indices("COMMENT").next() {
                    let rest = &create_stmt[comment_match.0..];
                    if let Some(start) = rest.find("'") {
                        if let Some(end) = rest[start + 1..].find("'") {
                            comment = Some(rest[start + 1..start + 1 + end].to_string());
                        }
                    }
                }
            }
        }

        // Get columns
        let columns_sql = format!("SHOW COLUMNS FROM {}.{}", database, table);
        let columns_result = client.show_proc_raw(&columns_sql).await?;

        let mut columns = Vec::new();
        for col_row in columns_result {
            if let Some(name) = col_row.get(0).and_then(|v| v.as_str()) {
                let type_str = col_row.get(1).and_then(|v| v.as_str()).unwrap_or("");
                let is_nullable = col_row.get(2).and_then(|v| v.as_str()) == Some("YES");
                let default_value = col_row.get(3).and_then(|v| v.as_str()).map(|s| s.to_string());
                let col_comment = col_row.get(4).and_then(|v| v.as_str()).map(|s| s.to_string());

                columns.push(ColumnInfo {
                    name: name.to_string(),
                    data_type: type_str.to_string(),
                    comment: col_comment,
                    default_value,
                    is_nullable,
                });
            }
        }

        // Get partitions
        let partitions_sql = format!("SHOW PARTITIONS FROM {}.{}", database, table);
        let partitions_result = client.show_proc_raw(&partitions_sql).await?;

        let mut partitions = Vec::new();
        for part_row in partitions_result {
            if let Some(name) = part_row.get(0).and_then(|v| v.as_str()) {
                let values = part_row.get(1).and_then(|v| v.as_str()).unwrap_or("");
                let status = part_row.get(2).and_then(|v| v.as_str()).unwrap_or("");
                let data_size = part_row.get(3).and_then(|v| v.as_i64());
                let rows = part_row.get(4).and_then(|v| v.as_i64());

                partitions.push(PartitionInfo {
                    name: name.to_string(),
                    values: values.to_string(),
                    status: status.to_string(),
                    data_size,
                    rows,
                });
            }
        }

        let partitions = if partitions.is_empty() { None } else { Some(partitions) };

        // Get buckets info (simplified - actual bucket info requires more complex parsing)
        let bucket_info = Some(BucketInfo {
            bucket_type: "hash".to_string(),
            bucket_keys: columns.iter().filter(|c| c.is_nullable).take(1).map(|c| c.name.clone()).collect(),
            bucket_count: 10,
        });

        Ok(TableDetailResponse {
            database: database.to_string(),
            name: table.to_string(),
            table_type,
            engine,
            create_time,
            rows: None,
            data_size: None,
            comment,
            columns,
            partitions,
            buckets: bucket_info,
        })
    }

    pub async fn update_table(&self, database: &str, table: &str, req: UpdateTableRequest) -> ApiResult<TableResponse> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        // For simplicity, we'll just recreate the table with new settings
        // In a production environment, you would use ALTER TABLE statements
        let drop_sql = format!("DROP TABLE IF EXISTS {}.{}", database, table);
        client.execute_sql(&drop_sql).await?;

        // Get current table info to preserve settings
        let current_table = self.get_table(database, table).await.ok();

        // Create new table with updated settings
        let create_req = CreateTableRequest {
            database: database.to_string(),
            name: table.to_string(),
            table_type: current_table.as_ref().map(|t| t.table_type.clone()).unwrap_or("duplicate".to_string()),
            columns: req.columns.unwrap_or_else(|| current_table.as_ref().map(|t| t.columns.iter().map(|c| ColumnDefinition {
                name: c.name.clone(),
                data_type: c.data_type.clone(),
                comment: c.comment.clone(),
                default_value: c.default_value.clone(),
                is_nullable: c.is_nullable,
                is_key: false,
                aggregate_type: None,
            }).collect()).unwrap_or(Vec::new())),
            partition_info: req.partition_info.or_else(|| current_table.as_ref().and_then(|t| {
                t.partitions.as_ref().map(|parts| PartitionDefinition {
                    partition_type: "range".to_string(),
                    partition_key: "".to_string(),
                    partitions: parts.iter().map(|p| PartitionSpec {
                        name: p.name.clone(),
                        values: p.values.clone(),
                    }).collect(),
                })
            })),
            bucket_info: req.bucket_info.or_else(|| current_table.as_ref().and_then(|t| {
                t.buckets.as_ref().map(|b| BucketDefinition {
                    bucket_type: b.bucket_type.clone(),
                    bucket_keys: b.bucket_keys.clone(),
                    bucket_count: b.bucket_count,
                })
            })),
            comment: req.comment.or_else(|| current_table.as_ref().and_then(|t| {
                t.comment.clone()
            })),
            properties: req.properties,
        };

        self.create_table(create_req).await
    }

    pub async fn delete_table(&self, database: &str, table: &str) -> ApiResult<()> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let sql = format!("DROP TABLE IF EXISTS {}.{}", database, table);
        client.execute_sql(&sql).await?;

        Ok(())
    }

    pub async fn execute_table_action(&self, database: &str, table: &str, req: TableActionRequest) -> ApiResult<()> {
        let cluster = self.get_active_cluster().await?;
        let client = StarRocksClient::new(cluster, self.mysql_pool_manager.clone());

        let sql = match req.action.as_str() {
            "truncate" => format!("TRUNCATE TABLE {}.{}", database, table),
            "optimize" => format!("OPTIMIZE TABLE {}.{}", database, table),
            "repair" => format!("REPAIR TABLE {}.{}", database, table),
            _ => return Err(ApiError::validation_error("Invalid table action")),
        };

        client.execute_sql(&sql).await?;

        Ok(())
    }
}
