//! Verify the downgrade guard against the SDK's actual durable configuration layout.

use microsandbox::{SandboxConfig, sandbox::MountBuilder};
use microsandbox_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, DbErr, Statement};

#[tokio::test]
async fn rollback_checks_serialized_desired_and_active_configs()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ordinary = SandboxConfig::default();
    ordinary.spec.name = "rollback-check".to_owned();
    ordinary.spec.mounts = vec![MountBuilder::new("/probe").disk("/unused.raw").build()?];
    let mut attached = ordinary.clone();
    attached.spec.mounts = vec![
        MountBuilder::new("/probe")
            .disk("/unused.raw")
            .attach_only()
            .build()?,
    ];
    let ordinary = serde_json::to_string(&ordinary)?;
    let attached = serde_json::to_string(&attached)?;
    let serialized: serde_json::Value = serde_json::from_str(&attached)?;
    assert!(serialized.get("spec").is_none());
    assert_eq!(
        serialized.pointer("/mounts/0/attach_only"),
        Some(&serde_json::Value::Bool(true))
    );

    for (config, active_config, blocked_column) in [
        (&attached, &ordinary, Some("config")),
        (&ordinary, &attached, Some("active_config")),
        (&ordinary, &ordinary, None),
    ] {
        let db = Database::connect("sqlite::memory:").await?;
        Migrator::up(&db, None).await?;
        let applied_before = Migrator::get_applied_migrations(&db).await?.len();
        db.execute_raw(Statement::from_sql_and_values(
            DatabaseBackend::Sqlite,
            "INSERT INTO sandbox (name, config, active_config, status) VALUES (?, ?, ?, ?)",
            [
                "rollback-check".into(),
                config.as_str().into(),
                active_config.as_str().into(),
                "Stopped".into(),
            ],
        ))
        .await?;

        let result = Migrator::down(&db, Some(1)).await;
        if let Some(column) = blocked_column {
            assert!(matches!(result, Err(DbErr::Migration(ref message))
                if message.contains("attach_only_downgrade_unrepresentable") && message.contains(column)));
            assert_eq!(
                Migrator::get_applied_migrations(&db).await?.len(),
                applied_before
            );
        } else {
            result?;
            assert_eq!(
                Migrator::get_applied_migrations(&db).await?.len(),
                applied_before - 1
            );
        }
        let row = db
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "SELECT config, active_config FROM sandbox WHERE name = 'rollback-check'"
                    .to_owned(),
            ))
            .await?
            .ok_or("sandbox disappeared during marker rollback")?;
        assert_eq!(row.try_get_by_index::<String>(0)?, *config);
        assert_eq!(row.try_get_by_index::<String>(1)?, *active_config);
    }
    Ok(())
}
