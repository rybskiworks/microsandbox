//! Establish the durable-config floor for disks that must remain unmounted.
//!
//! Older binaries ignore unknown mount fields. A schema-free marker makes
//! them refuse this database instead of interpreting attach-only disks as
//! ordinary filesystem mounts.

use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DatabaseBackend, Statement},
};

//--------------------------------------------------------------------------------------------------
// Types
//--------------------------------------------------------------------------------------------------

pub struct Migration;

//--------------------------------------------------------------------------------------------------
// Trait Implementations
//--------------------------------------------------------------------------------------------------

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260930_000001_attach_only_disk_config"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // The migration row itself establishes the compatibility floor.
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let rows = manager
            .get_connection()
            .query_all_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT id, config, active_config FROM sandbox".to_owned(),
            ))
            .await?;

        for row in rows {
            let id = row.try_get_by_index::<i32>(0)?;
            let config = row.try_get_by_index::<String>(1)?;
            reject_attach_only(id, "config", &config)?;
            if let Some(active_config) = row.try_get_by_index::<Option<String>>(2)? {
                reject_attach_only(id, "active_config", &active_config)?;
            }
        }
        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
// Functions
//--------------------------------------------------------------------------------------------------

fn reject_attach_only(sandbox_id: i32, column: &str, config: &str) -> Result<(), DbErr> {
    let value: serde_json::Value = serde_json::from_str(config).map_err(|error| {
        DbErr::Migration(format!(
            "attach_only_downgrade_unrepresentable: sandbox {sandbox_id} {column} is invalid JSON: {error}"
        ))
    })?;
    let config = value
        .as_object()
        .ok_or_else(|| malformed_config(sandbox_id, column, "configuration must be an object"))?;
    // SandboxConfig flattens SandboxSpec into the durable JSON object.
    reject_mounts(sandbox_id, column, config)?;
    // Check the nested form independently if present; neither location may hide the other.
    if let Some(spec) = config.get("spec") {
        let spec = spec
            .as_object()
            .ok_or_else(|| malformed_config(sandbox_id, column, "spec must be an object"))?;
        reject_mounts(sandbox_id, column, spec)?;
    }
    Ok(())
}

fn reject_mounts(
    sandbox_id: i32,
    column: &str,
    spec: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), DbErr> {
    let Some(mounts) = spec.get("mounts") else {
        return Ok(());
    };
    let mounts = mounts
        .as_array()
        .ok_or_else(|| malformed_config(sandbox_id, column, "mounts must be an array"))?;
    for mount in mounts {
        let mount = mount
            .as_object()
            .ok_or_else(|| malformed_config(sandbox_id, column, "mount must be an object"))?;
        match mount.get("attach_only") {
            None | Some(serde_json::Value::Bool(false)) => {}
            Some(serde_json::Value::Bool(true)) => {
                return Err(DbErr::Migration(format!(
                    "attach_only_downgrade_unrepresentable: sandbox {sandbox_id} {column} contains an attach-only disk that an older binary would mount"
                )));
            }
            Some(_) => {
                return Err(malformed_config(
                    sandbox_id,
                    column,
                    "attach_only must be a boolean",
                ));
            }
        }
    }
    Ok(())
}

fn malformed_config(sandbox_id: i32, column: &str, reason: &str) -> DbErr {
    DbErr::Migration(format!(
        "attach_only_downgrade_unrepresentable: sandbox {sandbox_id} {column} has malformed mount configuration: {reason}"
    ))
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use sea_orm_migration::sea_orm::{Database, DatabaseConnection};

    use super::*;

    struct MarkerMigrator;

    #[async_trait::async_trait]
    impl MigratorTrait for MarkerMigrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>> {
            vec![Box::new(Migration)]
        }
    }

    async fn fixture(
        config: &str,
        active_config: Option<&str>,
    ) -> Result<DatabaseConnection, DbErr> {
        let db = Database::connect("sqlite::memory:").await?;
        db.execute_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "CREATE TABLE sandbox (id INTEGER PRIMARY KEY, config TEXT NOT NULL, active_config TEXT)"
                .to_owned(),
        ))
        .await?;
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO sandbox (id, config, active_config) VALUES (1, ?, ?)",
            [config.into(), active_config.into()],
        ))
        .await?;
        MarkerMigrator::up(&db, None).await?;
        Ok(db)
    }

    #[tokio::test]
    async fn rollback_preserves_compatible_config_bytes() -> Result<(), Box<dyn std::error::Error>>
    {
        for config in [
            r#"{}"#,
            r#"{"mounts":[]}"#,
            r#"{"mounts":[{"type":"DiskImage","attach_only":false}]}"#,
            r#"{ "mounts": [{"type":"DiskImage"}]}"#,
            r#"{"mounts":[],"spec":{"mounts":[]}}"#,
            r#"{"spec":{"mounts":[]}}"#,
            r#"{"spec":{"mounts":[{"type":"DiskImage","attach_only":false}]}}"#,
            r#"{ "spec": {"mounts": [{"type":"DiskImage"}]}}"#,
        ] {
            let db = fixture(config, Some(config)).await?;
            MarkerMigrator::down(&db, Some(1)).await?;
            assert!(
                MarkerMigrator::get_applied_migrations(&db)
                    .await?
                    .is_empty()
            );
            let row = db
                .query_one_raw(Statement::from_string(
                    DatabaseBackend::Sqlite,
                    "SELECT config, active_config FROM sandbox WHERE id = 1".to_owned(),
                ))
                .await?
                .ok_or("sandbox disappeared during marker rollback")?;
            assert_eq!(row.try_get_by_index::<String>(0)?, config);
            assert_eq!(row.try_get_by_index::<String>(1)?, config);
        }
        Ok(())
    }

    #[tokio::test]
    async fn rollback_keeps_marker_when_desired_or_active_disk_is_attach_only()
    -> Result<(), Box<dyn std::error::Error>> {
        let empty = r#"{"spec":{"mounts":[]}}"#;
        let attached = r#"{"spec":{"mounts":[{"type":"DiskImage","attach_only":true}]}}"#;
        for column in ["config", "active_config"] {
            let (config, active_config) = if column == "config" {
                (attached, empty)
            } else {
                (empty, attached)
            };
            let db = fixture(config, Some(active_config)).await?;
            let result = MarkerMigrator::down(&db, Some(1)).await;
            assert!(matches!(result, Err(DbErr::Migration(ref message))
                if message.contains("attach_only_downgrade_unrepresentable") && message.contains(column)));
            assert_eq!(MarkerMigrator::get_applied_migrations(&db).await?.len(), 1);
        }
        Ok(())
    }

    #[tokio::test]
    async fn ambiguous_or_malformed_shapes_keep_marker() -> Result<(), Box<dyn std::error::Error>> {
        for config in [
            "not json",
            "null",
            "[]",
            r#"{"spec":null}"#,
            r#"{"spec":[]}"#,
            r#"{"mounts":null}"#,
            r#"{"mounts":{}}"#,
            r#"{"mounts":[null]}"#,
            r#"{"mounts":[[]]}"#,
            r#"{"mounts":[{"attach_only":null}]}"#,
            r#"{"mounts":[{"attach_only":"false"}]}"#,
            r#"{"mounts":[{"attach_only":0}]}"#,
            r#"{"spec":{"mounts":null}}"#,
            r#"{"spec":{"mounts":{}}}"#,
            r#"{"spec":{"mounts":[null]}}"#,
            r#"{"spec":{"mounts":[[]]}}"#,
            r#"{"spec":{"mounts":[{"attach_only":null}]}}"#,
            r#"{"spec":{"mounts":[{"attach_only":"false"}]}}"#,
            r#"{"spec":{"mounts":[{"attach_only":0}]}}"#,
            r#"{"mounts":[],"spec":{"mounts":[{"attach_only":true}]}}"#,
            r#"{"mounts":[{"attach_only":true}],"spec":{"mounts":[]}}"#,
        ] {
            for column in ["config", "active_config"] {
                let (desired, active) = if column == "config" {
                    (config, "{}")
                } else {
                    ("{}", config)
                };
                let db = fixture(desired, Some(active)).await?;
                let result = MarkerMigrator::down(&db, Some(1)).await;
                assert!(
                    matches!(result, Err(DbErr::Migration(ref message))
                    if message.contains("attach_only_downgrade_unrepresentable") && message.contains(column)),
                    "rollback accepted {column}: {config}"
                );
                assert_eq!(MarkerMigrator::get_applied_migrations(&db).await?.len(), 1);
                let row = db
                    .query_one_raw(Statement::from_string(
                        DatabaseBackend::Sqlite,
                        "SELECT config, active_config FROM sandbox WHERE id = 1".to_owned(),
                    ))
                    .await?
                    .ok_or("sandbox disappeared during refused marker rollback")?;
                assert_eq!(row.try_get_by_index::<String>(0)?, desired);
                assert_eq!(row.try_get_by_index::<String>(1)?, active);
            }
        }
        Ok(())
    }
}
