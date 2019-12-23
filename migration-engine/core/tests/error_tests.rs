mod test_harness;

use migration_connector::steps::{DeleteModel, MigrationStep};
use migration_core::{
    api::{render_error, RpcApi},
    cli,
    commands::{ApplyMigrationCommand, ApplyMigrationInput, InferMigrationStepsCommand, InferMigrationStepsInput},
};
use pretty_assertions::assert_eq;
use quaint::{prelude::*, single::Quaint};
use serde_json::json;
use test_harness::*;
use url::Url;

#[tokio::test]
async fn authentication_failure_must_return_a_known_error_on_postgres() {
    let mut url: Url = postgres_10_url("test-db").parse().unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "postgres"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let user = url.username();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server at `{host}`, the provided database credentials for `postgres` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `{host}`.", host = host),
        "meta": {
            "database_user": user,
            "database_host": host,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn authentication_failure_must_return_a_known_error_on_mysql() {
    let mut url: Url = mysql_url("authentication_failure_must_return_a_known_error_on_mysql")
        .parse()
        .unwrap();

    url.set_password(Some("obviously-not-right")).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let user = url.username();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Authentication failed against database server at `{host}`, the provided database credentials for `{user}` are not valid.\n\nPlease make sure to provide valid database credentials for the database server at `{host}`.", host = host, user = user),
        "meta": {
            "database_user": user,
            "database_host": host,
        },
        "error_code": "P1000"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_mysql() {
    let mut url: Url = mysql_url("unreachable_database_must_return_a_proper_error_on_mysql")
        .parse()
        .unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let port = url.port().unwrap();
    let host = url.host().unwrap().to_string();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn unreachable_database_must_return_a_proper_error_on_postgres() {
    let mut url: Url = postgres_10_url("unreachable_database_must_return_a_proper_error_on_postgres")
        .parse()
        .unwrap();

    url.set_port(Some(8787)).unwrap();

    let dm = format!(
        r#"
            datasource db {{
              provider = "postgres"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let host = url.host().unwrap().to_string();
    let port = url.port().unwrap();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Can't reach database server at `{host}`:`{port}`\n\nPlease make sure your database server is running at `{host}`:`{port}`.", host = host, port = port),
        "meta": {
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1001"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_does_not_exist_must_return_a_proper_error() {
    let mut url: Url = mysql_url("database_does_not_exist_must_return_a_proper_error")
        .parse()
        .unwrap();
    let database_name = "notmydatabase";

    url.set_path(database_name);

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();

    let json_error = serde_json::to_value(&render_error(error)).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": format!("Database `{database_name}` does not exist on the database server at `{database_host}:{database_port}`.", database_name = database_name, database_host = url.host().unwrap(), database_port = url.port().unwrap()),
        "meta": {
            "database_name": database_name,
            "database_host": format!("{}", url.host().unwrap()),
            "database_port": url.port().unwrap(),
        },
        "error_code": "P1003"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_already_exists_must_return_a_proper_error() {
    let db_name = "database_already_exists_must_return_a_proper_error";
    let url = postgres_10_url(db_name);

    let conn = Quaint::new(&postgres_10_url("postgres")).await.unwrap();
    conn.execute_raw(
        "CREATE DATABASE \"database_already_exists_must_return_a_proper_error\"",
        &[],
    )
    .await
    .ok();

    let error = get_cli_error(&["migration-engine", "cli", "--datasource", &url, "--create_database"]).await;

    let (host, port) = {
        let url = Url::parse(&url).unwrap();
        (url.host().unwrap().to_string(), url.port().unwrap())
    };

    let json_error = serde_json::to_value(&error).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": format!("Database `database_already_exists_must_return_a_proper_error` already exists on the database server at `{host}:{port}`", host = host, port = port),
        "meta": {
            "database_name": "database_already_exists_must_return_a_proper_error",
            "database_host": host,
            "database_port": port,
        },
        "error_code": "P1009"
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_access_denied_must_return_a_proper_error_in_cli() {
    let db_name = "dbaccessdeniedincli";
    let url: Url = mysql_url(db_name).parse().unwrap();
    let conn = create_mysql_database(&url).await.unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanmichel", &[]).await.unwrap();
    conn.execute_raw("CREATE USER jeanmichel IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = url.clone();
    url.set_username("jeanmichel").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("access_denied_test");

    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        url.as_str(),
        "--can_connect_to_database",
    ])
    .await;

    let json_error = serde_json::to_value(&error).unwrap();
    let expected = json!({
        "is_panic": false,
        "message": "User `jeanmichel` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanmichel",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[tokio::test]
async fn database_access_denied_must_return_a_proper_error_in_rpc() {
    let db_name = "dbaccessdeniedinrpc";
    let url: Url = mysql_url(db_name).parse().unwrap();
    let conn = create_mysql_database(&url).await.unwrap();

    conn.execute_raw("DROP USER IF EXISTS jeanyves", &[]).await.unwrap();
    conn.execute_raw("CREATE USER jeanyves IDENTIFIED BY '1234'", &[])
        .await
        .unwrap();

    let mut url: Url = url.clone();
    url.set_username("jeanyves").unwrap();
    url.set_password(Some("1234")).unwrap();
    url.set_path("access_denied_test");

    let dm = format!(
        r#"
            datasource db {{
              provider = "mysql"
              url      = "{}"
            }}
        "#,
        url,
    );

    let error = RpcApi::new(&dm).await.map(|_| ()).unwrap_err();
    let json_error = serde_json::to_value(&render_error(error)).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": "User `jeanyves` was denied access on the database `access_denied_test`",
        "meta": {
            "database_user": "jeanyves",
            "database_name": "access_denied_test",
        },
        "error_code": "P1010",
    });

    assert_eq!(json_error, expected);
}

#[test_one_connector(connector = "postgres")]
async fn command_errors_must_return_an_unknown_error(api: &TestApi) {
    let input = ApplyMigrationInput {
        migration_id: "the-migration".to_owned(),
        steps: vec![MigrationStep::DeleteModel(DeleteModel {
            model: "abcd".to_owned(),
        })],
        force: Some(true),
    };

    let error = api.execute_command::<ApplyMigrationCommand>(&input).await.unwrap_err();

    let expected_error = user_facing_errors::Error::from(user_facing_errors::UnknownError {
        message: "Failure during a migration command: Generic error. (error: The model abcd does not exist in this Datamodel. It is not possible to delete it.)".to_owned(),
        backtrace: None,
    });

    assert_eq!(error, expected_error);
}

#[test_each_connector]
async fn unique_constraint_errors_in_migrations_must_return_a_known_error(api: &TestApi) {
    use quaint::ast::*;

    let dm = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String
        }
    "#;

    api.infer_and_apply(dm).await;

    let insert = Insert::multi_into(api.render_table_name("Fruit"), vec!["name"])
        .values(("banana",))
        .values(("apple",))
        .values(("banana",));

    api.database().execute(insert.into()).await.unwrap();

    let dm2 = r#"
        model Fruit {
            id Int @id @default(autoincrement())
            name String @unique
        }
    "#;

    let infer_migration_steps_input = InferMigrationStepsInput {
        assume_to_be_applied: vec![],
        datamodel: dm2.to_owned(),
        migration_id: "the-migration".to_owned(),
    };

    let steps = api
        .execute_command::<InferMigrationStepsCommand>(&infer_migration_steps_input)
        .await
        .unwrap()
        .datamodel_steps;

    let input = ApplyMigrationInput {
        migration_id: "the-migration".to_owned(),
        steps: steps,
        force: Some(true),
    };

    let error = api.execute_command::<ApplyMigrationCommand>(&input).await.unwrap_err();

    let json_error = serde_json::to_value(&error).unwrap();

    let field_name = match api.sql_family() {
        SqlFamily::Mysql => "Fruit.name",
        _ => "name",
    };

    let expected_json = json!({
        "is_panic": false,
        "message": format!("Unique constraint failed on the field: `{}`", field_name),
        "meta": {
            "field_name": field_name,
        },
        "error_code": "P2002",
    });

    assert_eq!(json_error, expected_json);
}

#[test_one_connector(connector = "postgres")]
async fn tls_errors_must_be_mapped_in_the_cli(_api: &TestApi) {
    let url = format!(
        "{}&sslmode=require&sslaccept=strict",
        postgres_10_url("tls_errors_must_be_mapped_in_the_cli")
    );
    let error = get_cli_error(&[
        "migration-engine",
        "cli",
        "--datasource",
        &url,
        "--can_connect_to_database",
    ])
    .await;

    let json_error = serde_json::to_value(&error).unwrap();

    let expected = json!({
        "is_panic": false,
        "message": format!("Error opening a TLS connection: error performing TLS handshake: server does not support TLS"),
        "meta": {
            "message": "error performing TLS handshake: server does not support TLS",
        },
        "error_code": "P1011"
    });

    assert_eq!(json_error, expected);
}

async fn get_cli_error(cli_args: &[&str]) -> user_facing_errors::Error {
    let app = cli::clap_app();
    let matches = app.get_matches_from(cli_args);
    let cli_matches = matches.subcommand_matches("cli").expect("cli subcommand is passed");
    let database_url = cli_matches.value_of("datasource").expect("datasource is provided");
    cli::run(&cli_matches, database_url)
        .await
        .map_err(|err| cli::render_error(err))
        .unwrap_err()
}
