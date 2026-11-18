//! The limits and include settings of `Resolver`, and `Config::save` with
//! includes.

use noml::resolver::{
    DEFAULT_MAX_INCLUDES, DEFAULT_MAX_OUTPUT_BYTES, DEFAULT_MAX_REFERENCES,
    DEFAULT_MAX_REFERENCE_DEPTH,
};
use noml::{parse_string, Config, Resolver, ResolverConfig, ResolverLimits};
use std::fs;

#[test]
fn default_limits() {
    let limits = ResolverLimits::default();
    assert_eq!(limits, ResolverLimits::new());
    assert_eq!(limits.max_output_bytes(), 64 * 1024 * 1024);
    assert_eq!(limits.max_output_bytes(), DEFAULT_MAX_OUTPUT_BYTES);
    assert_eq!(limits.max_references(), DEFAULT_MAX_REFERENCES);
    assert_eq!(limits.max_reference_depth(), DEFAULT_MAX_REFERENCE_DEPTH);
    assert_eq!(limits.max_includes(), DEFAULT_MAX_INCLUDES);
    assert_eq!(limits.max_http_body_bytes(), 1024 * 1024);
    assert_eq!(Resolver::new().limits(), &limits);
}

#[test]
fn limits_can_be_lowered_and_raised() {
    let document = parse_string(
        "a = \"0123456789\"\nb = \"${a}${a}${a}\"\nc = ${b}\nd = ${c}",
        None,
    )
    .unwrap();

    let value = Resolver::new().resolve(&document).unwrap();
    assert_eq!(value.get("d").unwrap().as_string().unwrap().len(), 30);

    let small = ResolverLimits::new().with_max_output_bytes(40);
    let error = Resolver::new()
        .with_limits(small)
        .resolve(&document)
        .unwrap_err()
        .to_string();
    assert!(error.contains("limit of 40 bytes"), "{error}");

    let few = ResolverLimits::new().with_max_references(4);
    assert!(Resolver::new().with_limits(few).resolve(&document).is_err());

    // d -> c -> b -> a is a chain of three references
    let shallow = ResolverLimits::new().with_max_reference_depth(2);
    let error = Resolver::new()
        .with_limits(shallow)
        .resolve(&document)
        .unwrap_err()
        .to_string();
    assert!(error.contains("limit of 2 levels"), "{error}");
    let enough = ResolverLimits::new().with_max_reference_depth(3);
    assert!(Resolver::new()
        .with_limits(enough)
        .resolve(&document)
        .is_ok());

    // A longer chain than the default works when allowed
    let mut source = String::from("v0 = 1\n");
    for i in 1..100 {
        source.push_str(&format!("v{i} = ${{v{}}}\n", i - 1));
    }
    let document = parse_string(&source, None).unwrap();
    assert!(Resolver::new().resolve(&document).is_err());
    let deep = ResolverLimits::new().with_max_reference_depth(100);
    let value = Resolver::new()
        .with_limits(deep)
        .resolve(&document)
        .unwrap();
    assert_eq!(value.get("v99").unwrap().as_integer().unwrap(), 1);
}

#[test]
fn include_root_and_includes_anywhere() {
    let dir = tempfile::tempdir().unwrap();
    let conf = dir.path().join("conf");
    fs::create_dir(&conf).unwrap();
    fs::write(conf.join("db.noml"), "port = 5432\n").unwrap();
    fs::write(dir.path().join("shared.noml"), "region = \"eu\"\n").unwrap();

    // Text has no base path: refused unless a root is given
    let document = parse_string("db = include \"db.noml\"", None).unwrap();
    assert!(Resolver::new().resolve(&document).is_err());
    let value = Resolver::new()
        .with_include_root(&conf)
        .resolve(&document)
        .unwrap();
    assert_eq!(value.get("db.port").unwrap().as_integer().unwrap(), 5432);

    // A wider root allows `..` inside it
    let document = parse_string("s = include \"../shared.noml\"", None).unwrap();
    let narrow = Resolver::new().with_base_path(&conf).resolve(&document);
    assert!(narrow.unwrap_err().to_string().contains("outside"));
    let value = Resolver::new()
        .with_base_path(&conf)
        .with_include_root(dir.path())
        .resolve(&document)
        .unwrap();
    assert_eq!(value.get("s.region").unwrap().as_string().unwrap(), "eu");

    // No check at all
    let value = Resolver::new()
        .with_base_path(&conf)
        .allow_includes_anywhere(true)
        .resolve(&document)
        .unwrap();
    assert_eq!(value.get("s.region").unwrap().as_string().unwrap(), "eu");

    // max_include_depth = 0 refuses every include
    let mut config = ResolverConfig::default();
    config.max_include_depth = 0;
    let document = parse_string("db = include \"db.noml\"", None).unwrap();
    let error = Resolver::with_config(config)
        .with_include_root(&conf)
        .resolve(&document)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Maximum include depth"), "{error}");
}

#[test]
fn include_count_limit() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("part.noml"), "x = 1\n").unwrap();
    let mut source = String::new();
    for i in 0..5 {
        source.push_str(&format!("p{i} = include \"part.noml\"\n"));
    }
    let document = parse_string(&source, None).unwrap();
    let mut resolver = Resolver::new()
        .with_include_root(dir.path())
        .with_limits(ResolverLimits::new().with_max_includes(4));
    let error = resolver.resolve(&document).unwrap_err().to_string();
    assert!(error.contains("Maximum include count (4)"), "{error}");
    // The count starts again for each resolve
    let mut resolver = resolver.with_limits(ResolverLimits::new().with_max_includes(5));
    assert!(resolver.resolve(&document).is_ok());
    assert!(resolver.resolve(&document).is_ok());
}

#[test]
fn save_keeps_includes_that_did_not_change() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("db.noml"),
        "host = \"db\"\npassword = \"included-secret\"\n",
    )
    .unwrap();
    let path = dir.path().join("app.noml");
    fs::write(
        &path,
        "name = \"app\"\ndb = include \"db.noml\"\nports = [80, env(\"NOML_SAVE_PORT_UNSET\", 443)]\n",
    )
    .unwrap();

    let mut config = Config::from_file(&path).unwrap();
    config.set("name", "renamed").unwrap();
    config.save().unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("include \"db.noml\""), "{saved}");
    assert!(!saved.contains("included-secret"), "{saved}");
    assert!(
        saved.contains("env(\"NOML_SAVE_PORT_UNSET\", 443)"),
        "{saved}"
    );
    assert!(saved.contains("\"renamed\""), "{saved}");

    let reloaded = Config::from_file(&path).unwrap();
    assert_eq!(reloaded.as_value(), config.as_value());
}

#[test]
fn save_keeps_keys_merged_into_an_included_table() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("db.noml"), "host = \"db\"\n").unwrap();
    let path = dir.path().join("app.noml");
    fs::write(&path, "db = include \"db.noml\"\n\n[db]\nport = 5432\n").unwrap();

    let config = Config::from_file(&path).unwrap();
    assert_eq!(config.get("db.port").unwrap().as_integer().unwrap(), 5432);
    config.save().unwrap();
    let reloaded = Config::from_file(&path).unwrap();
    assert_eq!(reloaded.as_value(), config.as_value());
}
