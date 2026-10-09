//! Tests for `${...}` interpolation (issue #23) across every public entry point,
//! plus the table-building rules interpolation depends on.

use noml::{
    parse, parse_from_file, parse_raw, serialize_document, Config, NomlError, Resolver, Value,
};
use std::fs;
use tempfile::TempDir;

const SOURCE: &str = r#"
app_name = "my-app"
environment = "production"
port = 8080
log_file = "/var/log/${app_name}-${environment}.log"
copy_port = ${port}

[database]
name = "${app_name}_${environment}"
connection_string = "postgres://localhost:${port}/${database.name}"
"#;

fn check(value: &Value) {
    assert_eq!(
        value.get("log_file").unwrap().as_string().unwrap(),
        "/var/log/my-app-production.log"
    );
    assert_eq!(value.get("copy_port").unwrap(), &Value::Integer(8080));
    assert_eq!(
        value.get("database.name").unwrap().as_string().unwrap(),
        "my-app_production"
    );
    assert_eq!(
        value
            .get("database.connection_string")
            .unwrap()
            .as_string()
            .unwrap(),
        "postgres://localhost:8080/my-app_production"
    );
}

fn err(source: &str) -> NomlError {
    parse(source).expect_err("expected an error")
}

#[test]
fn issue_23_examples() {
    // Unquoted reference
    let config = parse("foo = \"my name\"\nbar = ${foo}").unwrap();
    assert_eq!(config.get("bar").unwrap().as_string().unwrap(), "my name");

    // Quoted reference
    let config = parse("foo = \"my name\"\nbar = \"${foo}\"").unwrap();
    assert_eq!(config.get("bar").unwrap().as_string().unwrap(), "my name");
}

#[test]
fn every_entry_point_interpolates() {
    check(&parse(SOURCE).unwrap());

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("app.noml");
    fs::write(&path, SOURCE).unwrap();
    check(&parse_from_file(&path).unwrap());

    let document = parse_raw(SOURCE).unwrap();
    check(&document.to_value().unwrap());
    check(&Resolver::new().resolve(&document).unwrap());
    check(&Resolver::new().resolve_with_context(&document).unwrap());

    check(Config::from_string(SOURCE).unwrap().as_value());
    check(Config::from_file(&path).unwrap().as_value());
    check(
        Config::builder()
            .build_from_string(SOURCE)
            .unwrap()
            .as_value(),
    );
    check(Config::builder().build_from_file(&path).unwrap().as_value());
}

#[test]
fn references_can_point_forward_and_chain() {
    let config = parse(
        r#"
        a = "${b}-a"
        b = "${c}-b"
        c = "c"
        "#,
    )
    .unwrap();
    assert_eq!(config.get("a").unwrap().as_string().unwrap(), "c-b-a");
}

#[test]
fn bare_references_keep_their_type() {
    let config = parse(
        r#"
        ratio = 0.5
        enabled = true
        tags = ["a", "b"]
        f = ${ratio}
        e = ${enabled}
        t = ${tags}
        server_copy = ${server}

        [server]
        host = "localhost"
        url = "http://${server.host}"
        "#,
    )
    .unwrap();
    assert_eq!(config.get("f").unwrap(), &Value::Float(0.5));
    assert_eq!(config.get("e").unwrap(), &Value::Bool(true));
    assert_eq!(config.get("t").unwrap().as_array().unwrap().len(), 2);
    // The copied table has its own interpolation already applied
    assert_eq!(
        config.get("server_copy.url").unwrap().as_string().unwrap(),
        "http://localhost"
    );
}

#[test]
fn values_are_converted_to_text_in_strings() {
    let config = parse(
        r#"
        i = 42
        f = 2.0
        b = false
        s = "${i} ${f} ${b}"
        "#,
    )
    .unwrap();
    assert_eq!(
        config.get("s").unwrap().as_string().unwrap(),
        "42 2.0 false"
    );
}

#[test]
fn array_indexes_and_arrays_of_tables() {
    let config = parse(
        r#"
        ports = [80, 443]
        first = "${ports.0}"
        second = "${ports[1]}"

        [[servers]]
        name = "alpha"

        [[servers]]
        name = "beta"
        peer = "${servers.0.name}"
        "#,
    )
    .unwrap();
    assert_eq!(config.get("first").unwrap().as_string().unwrap(), "80");
    assert_eq!(config.get("second").unwrap().as_string().unwrap(), "443");
    assert_eq!(
        config.get("servers.1.peer").unwrap().as_string().unwrap(),
        "alpha"
    );
}

#[test]
fn quoted_keys_in_paths() {
    let config =
        parse("\"dotted.key\" = \"v\"\nx = \"${\\\"dotted.key\\\"}\"\ny = ${\"dotted.key\"}")
            .unwrap();
    assert_eq!(config.get("x").unwrap().as_string().unwrap(), "v");
    assert_eq!(config.get("y").unwrap().as_string().unwrap(), "v");
}

#[test]
fn escaping_and_literal_strings() {
    let config = parse(
        r#"
        name = "x"
        escaped = "$${name} is ${name}"
        literal = '${name}'
        multi_literal = '''${name}'''
        raw = r"${name}"
        dollar = "cost: $5 and $"
        multi = """
${name}"""
        "#,
    )
    .unwrap();
    assert_eq!(
        config.get("escaped").unwrap().as_string().unwrap(),
        "${name} is x"
    );
    assert_eq!(
        config.get("literal").unwrap().as_string().unwrap(),
        "${name}"
    );
    assert_eq!(
        config.get("multi_literal").unwrap().as_string().unwrap(),
        "${name}"
    );
    assert_eq!(config.get("raw").unwrap().as_string().unwrap(), "${name}");
    assert_eq!(
        config.get("dollar").unwrap().as_string().unwrap(),
        "cost: $5 and $"
    );
    assert_eq!(config.get("multi").unwrap().as_string().unwrap(), "x");
}

#[test]
fn env_and_native_arguments_can_interpolate() {
    let config = parse(
        r#"
        unit = "MB"
        size = @size("10${unit}")
        home = env("NOML_TEST_SURELY_UNSET_VAR", "/srv/${unit}")
        "#,
    )
    .unwrap();
    assert_eq!(config.get("size").unwrap().as_integer().unwrap(), 10 << 20);
    assert_eq!(config.get("home").unwrap().as_string().unwrap(), "/srv/MB");
}

#[test]
fn environment_values_are_not_re_interpolated() {
    let mut vars = std::collections::HashMap::new();
    vars.insert("TEMPLATE".to_string(), "${secret}".to_string());
    let document = parse_raw("secret = \"s\"\nv = env(\"TEMPLATE\")").unwrap();
    let value = Resolver::new()
        .with_env_vars(vars)
        .resolve(&document)
        .unwrap();
    assert_eq!(value.get("v").unwrap().as_string().unwrap(), "${secret}");
}

#[test]
fn undefined_variable_reports_position() {
    match err("a = 1\n\nb = \"x ${missing}\"") {
        NomlError::Interpolation {
            message,
            expression,
            context,
        } => {
            assert_eq!(expression, "missing");
            assert!(message.contains("line 3"), "{message}");
            assert_eq!(context.as_deref(), Some("b"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
    assert!(matches!(
        err("b = ${nope.deeper}"),
        NomlError::Interpolation { .. }
    ));
    assert!(matches!(
        err("b = \"${}\""),
        NomlError::Interpolation { .. }
    ));
    assert!(matches!(
        err("b = \"${unclosed\""),
        NomlError::Interpolation { .. }
    ));
}

#[test]
fn cycles_are_reported() {
    for source in [
        "a = \"${a}\"",
        "a = \"${b}\"\nb = \"${a}\"",
        "a = ${b}\nb = ${c}\nc = ${a}",
        "t = { x = \"${t}\" }",
    ] {
        match err(source) {
            NomlError::CircularReference { chain } => assert!(chain.contains("->"), "{chain}"),
            other => panic!("{source}: unexpected error {other:?}"),
        }
    }
}

#[test]
fn tables_and_arrays_cannot_be_inserted_into_strings() {
    assert!(matches!(
        err("t = { a = 1 }\ns = \"${t}\""),
        NomlError::Interpolation { .. }
    ));
    assert!(matches!(
        err("t = [1]\ns = \"${t}\""),
        NomlError::Interpolation { .. }
    ));
}

#[test]
fn variables_from_the_resolver_fill_gaps() {
    let document = parse_raw("greeting = \"hello ${user.name}, ${count}\"").unwrap();
    let mut resolver = Resolver::new();
    let mut user = std::collections::BTreeMap::new();
    user.insert("name".to_string(), Value::String("ana".to_string()));
    resolver.set_variable("user".to_string(), Value::Table(user));
    resolver.set_variable("count".to_string(), Value::Integer(3));
    let value = resolver.resolve(&document).unwrap();
    assert_eq!(
        value.get("greeting").unwrap().as_string().unwrap(),
        "hello ana, 3"
    );
    // Variables survive across resolve calls
    assert!(resolver.resolve(&document).is_ok());

    // Document values win over variables
    let document = parse_raw("count = 7\nv = ${count}").unwrap();
    assert_eq!(
        resolver.resolve(&document).unwrap().get("v").unwrap(),
        &Value::Integer(7)
    );
}

#[test]
fn includes_resolve_their_own_paths_first() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("db.noml"),
        "host = \"db.local\"\nurl = \"postgres://${host}/${app_name}\"\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("main.noml"),
        "app_name = \"shop\"\ndatabase = include \"db.noml\"\ndb_url = \"${database.url}\"\n",
    )
    .unwrap();

    let value = parse_from_file(dir.path().join("main.noml")).unwrap();
    assert_eq!(
        value.get("database.url").unwrap().as_string().unwrap(),
        "postgres://db.local/shop"
    );
    assert_eq!(
        value.get("db_url").unwrap().as_string().unwrap(),
        "postgres://db.local/shop"
    );
}

#[test]
fn circular_includes_are_reported() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.noml"), "b = include \"b.noml\"\n").unwrap();
    fs::write(dir.path().join("b.noml"), "a = include \"a.noml\"\n").unwrap();
    let error = parse_from_file(dir.path().join("a.noml")).unwrap_err();
    assert!(error.to_string().contains("includes itself"), "{error}");
}

#[test]
fn round_trip_keeps_templates() {
    let document = parse_raw(SOURCE).unwrap();
    let text = serialize_document(&document).unwrap();
    check(&parse(&text).unwrap());

    // Config::save writes resolved values, escaping a literal `${`
    let config = Config::from_string("a = '${not_a_reference}'").unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("out.noml");
    config.save_to_file(&path).unwrap();
    let reloaded = Config::from_file(&path).unwrap();
    assert_eq!(
        reloaded.get("a").unwrap().as_string().unwrap(),
        "${not_a_reference}"
    );
}

#[test]
fn quoted_keys_are_single_keys() {
    let config = parse("\"a.b\" = 1\n'c d' = 2").unwrap();
    let table = config.as_table().unwrap();
    assert_eq!(table.get("a.b"), Some(&Value::Integer(1)));
    assert_eq!(table.get("c d"), Some(&Value::Integer(2)));
}

#[test]
fn tables_merge_in_any_order() {
    let config = parse("[a.b]\nx = 1\n\n[a]\ny = 2\n").unwrap();
    assert_eq!(config.get("a.b.x").unwrap(), &Value::Integer(1));
    assert_eq!(config.get("a.y").unwrap(), &Value::Integer(2));

    let config = parse("a.b = 1\n[a]\nc = 2").unwrap();
    assert_eq!(config.get("a.b").unwrap(), &Value::Integer(1));
    assert_eq!(config.get("a.c").unwrap(), &Value::Integer(2));
}

#[test]
fn sub_tables_of_arrays_of_tables_stay_with_their_element() {
    let config = parse(
        r#"
        [[s]]
        n = 1
        [s.extra]
        k = "first"

        [[s]]
        n = 2
        "#,
    )
    .unwrap();
    assert_eq!(
        config.get("s.0.extra.k").unwrap().as_string().unwrap(),
        "first"
    );
    assert!(config.get("s.1.extra").is_none());
}

#[test]
fn duplicate_keys_are_errors() {
    for source in [
        "a = 1\na = 2",
        "a = 1\n[a]\nb = 2",
        "[t]\nx = 1\n[t]\nx = 2",
    ] {
        let error = err(source);
        assert!(
            error.to_string().contains("Duplicate key")
                || error.to_string().contains("already defined"),
            "{source}: {error}"
        );
    }
}

#[test]
fn deep_nesting_is_an_error_not_a_crash() {
    let source = format!("a = {}{}", "[".repeat(10_000), "]".repeat(10_000));
    assert!(parse(&source).is_err());
    let source = format!("a = {}1{}", "{ b = ".repeat(10_000), " }".repeat(10_000));
    assert!(parse(&source).is_err());
    let source = format!("[{}]\nx = 1", vec!["k"; 10_000].join("."));
    assert!(parse(&source).is_err());

    // A chain of copies that keeps nesting deeper is stopped too
    let mut chain = String::from("t0 = 1\n");
    for i in 1..300 {
        chain.push_str(&format!("t{i} = {{ v = ${{t{}}} }}\n", i - 1));
    }
    assert!(parse(&chain).is_err());
}

#[cfg(feature = "async")]
mod async_entry_points {
    use super::*;

    #[tokio::test]
    async fn async_entry_points_interpolate() {
        check(&noml::parse_async(SOURCE).await.unwrap());

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("app.noml");
        fs::write(&path, SOURCE).unwrap();
        check(&noml::parse_from_file_async(&path).await.unwrap());
        check(Config::load_async(&path).await.unwrap().as_value());

        let document = parse_raw(SOURCE).unwrap();
        check(
            &Resolver::new()
                .resolve_document_async(&document)
                .await
                .unwrap(),
        );
    }
}
