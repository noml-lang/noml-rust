//! Documents written to exhaust memory, the stack or the file system, and
//! errors that must not repeat secret values.

use noml::{parse, parse_from_file, Config, Value};
use std::fmt::Write as _;
use std::fs;
use std::time::{Duration, Instant};

/// `l0` is ten bytes, each later level references the previous one ten times.
fn string_bomb(levels: usize) -> String {
    let mut s = String::from("l0 = \"xxxxxxxxxx\"\n");
    for level in 1..=levels {
        let _ = write!(s, "l{level} = \"");
        for _ in 0..10 {
            let _ = write!(s, "${{l{}}}", level - 1);
        }
        s.push_str("\"\n");
    }
    s
}

fn tree_bomb(levels: usize) -> String {
    let mut s = String::from("a0 = [1, 1, 1, 1, 1, 1, 1, 1, 1, 1]\n");
    for level in 1..=levels {
        let _ = write!(s, "a{level} = [");
        for i in 0..10 {
            if i > 0 {
                s.push_str(", ");
            }
            let _ = write!(s, "${{a{}}}", level - 1);
        }
        s.push_str("]\n");
    }
    s
}

/// A chain written head first: `v{n-1} = ${v{n-2}}` ... `v0 = 1`
fn head_first_chain(n: usize) -> String {
    let mut s = String::new();
    for i in (1..n).rev() {
        let _ = writeln!(s, "v{i} = ${{v{}}}", i - 1);
    }
    s.push_str("v0 = 1\n");
    s
}

fn forward_chain(n: usize) -> String {
    let mut s = String::from("v0 = 1\n");
    for i in 1..n {
        let _ = writeln!(s, "v{i} = ${{v{}}}", i - 1);
    }
    s
}

#[test]
fn string_expansion_is_bounded() {
    // Seven levels would be a 100,000,000-byte string from 424 bytes of text
    let start = Instant::now();
    let error = parse(&string_bomb(7)).unwrap_err().to_string();
    assert!(error.contains("limit"), "{error}");
    assert!(!error.contains("xxxxxxxxxx"), "{error}");
    assert!(start.elapsed() < Duration::from_secs(10));

    // Five levels (a 1,000,000-byte string) are within the default budget
    let value = parse(&string_bomb(5)).unwrap();
    assert_eq!(
        value.get("l5").unwrap().as_string().unwrap().len(),
        1_000_000
    );
}

#[test]
fn tree_expansion_is_bounded() {
    // Six levels would be 12,345,678 values
    let error = parse(&tree_bomb(6)).unwrap_err().to_string();
    assert!(error.contains("limit"), "{error}");
    let value = parse(&tree_bomb(3)).unwrap();
    assert_eq!(value.get("a3").unwrap().as_array().unwrap().len(), 10);
}

#[test]
fn reference_count_is_bounded() {
    let mut s = String::from("a = 1\nb = [");
    for i in 0..10_001 {
        if i > 0 {
            s.push(',');
        }
        s.push_str("${a}");
    }
    s.push_str("]\n");
    let error = parse(&s).unwrap_err().to_string();
    assert!(error.contains("limit"), "{error}");
}

fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn long_head_first_chain_is_an_error_not_a_stack_overflow() {
    for n in [1_000, 100_000] {
        let source = head_first_chain(n);
        let result = on_small_stack(move || parse(&source).map(|_| ()).map_err(|e| e.to_string()));
        let error = result.unwrap_err();
        assert!(error.contains("limit"), "{error}");
    }
}

#[test]
fn reference_chains_are_limited_in_any_order() {
    // 32 links are allowed, written either way round
    for source in [head_first_chain(33), forward_chain(33)] {
        let value = on_small_stack(move || parse(&source).map_err(|e| e.to_string())).unwrap();
        assert_eq!(value.get("v32").unwrap().as_integer().unwrap(), 1);
    }
    // 33 links are not
    for source in [head_first_chain(34), forward_chain(34)] {
        let error = on_small_stack(move || parse(&source).map(|_| ()).map_err(|e| e.to_string()))
            .unwrap_err();
        assert!(error.contains("limit"), "{error}");
    }
}

#[test]
fn includes_stay_in_the_directory_of_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app");
    fs::create_dir(&app).unwrap();
    fs::create_dir(app.join("conf.d")).unwrap();
    fs::write(
        dir.path().join("secret.noml"),
        "password = \"from-outside\"\n",
    )
    .unwrap();
    fs::write(app.join("conf.d").join("db.noml"), "port = 5432\n").unwrap();

    // Inside the directory: allowed
    fs::write(app.join("ok.noml"), "db = include \"conf.d/db.noml\"\n").unwrap();
    let value = parse_from_file(app.join("ok.noml")).unwrap();
    assert_eq!(value.get("db.port").unwrap().as_integer().unwrap(), 5432);

    // `..` and an absolute path outside it: refused
    fs::write(app.join("dotdot.noml"), "x = include \"../secret.noml\"\n").unwrap();
    let error = parse_from_file(app.join("dotdot.noml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("outside"), "{error}");

    let absolute = dir.path().join("secret.noml");
    fs::write(
        app.join("absolute.noml"),
        format!("x = include {:?}\n", absolute.to_string_lossy()),
    )
    .unwrap();
    let error = parse_from_file(app.join("absolute.noml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("outside"), "{error}");
    assert!(!error.contains("from-outside"), "{error}");
}

#[cfg(unix)]
#[test]
fn includes_cannot_leave_the_directory_through_a_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("app");
    fs::create_dir(&app).unwrap();
    fs::write(
        dir.path().join("secret.noml"),
        "password = \"from-outside\"\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(dir.path().join("secret.noml"), app.join("link.noml")).unwrap();
    fs::write(app.join("main.noml"), "x = include \"link.noml\"\n").unwrap();
    let error = parse_from_file(app.join("main.noml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("outside"), "{error}");
}

#[test]
fn text_without_a_base_path_cannot_include() {
    // The test runs in the package directory, which has a Cargo.toml
    let error = parse("x = include \"Cargo.toml\"").unwrap_err().to_string();
    assert!(error.contains("include"), "{error}");
    assert!(!error.contains("noml"), "{error}");
}

#[test]
fn include_cycles_are_found_however_the_path_is_spelled() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(
        dir.path().join("self.noml"),
        "x = include \"sub/../self.noml\"\n",
    )
    .unwrap();
    let error = parse_from_file(dir.path().join("self.noml")).unwrap_err();
    assert!(
        matches!(error, noml::NomlError::CircularReference { .. }),
        "{error}"
    );
}

#[test]
fn include_fan_out_is_bounded() {
    // Each file includes the next four times: 4^6 = 4096 includes in total
    let dir = tempfile::tempdir().unwrap();
    for level in 0..6 {
        let mut s = String::new();
        for i in 0..4 {
            let _ = writeln!(s, "k{i} = include \"f{}.noml\"", level + 1);
        }
        fs::write(dir.path().join(format!("f{level}.noml")), s).unwrap();
    }
    fs::write(dir.path().join("f6.noml"), "leaf = 1\n").unwrap();
    let start = Instant::now();
    let error = parse_from_file(dir.path().join("f0.noml"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("include"), "{error}");
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn include_errors_do_not_quote_the_included_file() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("credentials"),
        "[default]\naws_access_key_id = AKIAIOSFODNN7EXAMPLE\n",
    )
    .unwrap();
    fs::write(dir.path().join(".env"), "PASSWORD=hunter2-very-secret\n").unwrap();
    for target in ["credentials", ".env"] {
        fs::write(
            dir.path().join("main.noml"),
            format!("x = include \"{target}\"\n"),
        )
        .unwrap();
        let error = parse_from_file(dir.path().join("main.noml")).unwrap_err();
        let text = format!("{error} {error:?} {}", error.user_message());
        assert!(!text.contains("AKIAIOSFODNN7EXAMPLE"), "{text}");
        assert!(!text.contains("hunter2"), "{text}");
    }
}

/// Every form of an error: Display, Debug and the user message
fn error_text(source: &str) -> String {
    let error = parse(source).unwrap_err();
    format!("{error} {error:?} {}", error.user_message())
}

#[test]
fn parse_errors_do_not_quote_values() {
    for (source, secret) in [
        ("password = hunter2secret", "hunter2secret"),
        ("password = # hunter2secret", "hunter2secret"),
        (
            "pin = 123456789012345678901234567890",
            "123456789012345678901234567890",
        ),
        ("pin = 0x12345678ZZ", "12345678"),
        ("token = \"ab\\qhunter2\"", "hunter2"),
    ] {
        let text = error_text(source);
        assert!(!text.contains(secret), "{source}: {text}");
    }
}

#[test]
fn native_type_errors_do_not_quote_arguments() {
    for (source, secret) in [
        ("dsn = @url(\"postgres://user:p4ssw0rd@db\")", "p4ssw0rd"),
        ("a = @size(\"p4ssw0rd\")", "p4ssw0rd"),
        ("a = @duration(\"p4ssw0rd\")", "p4ssw0rd"),
        ("a = @ip(\"p4ssw0rd\")", "p4ssw0rd"),
        ("a = @semver(\"p4ssw0rd\")", "p4ssw0rd"),
        ("a = @base64(\"p4ssw0rd!\")", "p4ssw0rd"),
        ("a = @uuid(\"p4ssw0rd\")", "p4ssw0rd"),
    ] {
        let text = error_text(source);
        assert!(!text.contains(secret), "{source}: {text}");
    }
}

#[test]
fn conversion_errors_do_not_quote_values() {
    let secret = Value::String("hunter2".to_string());
    for error in [
        secret.as_integer().unwrap_err(),
        secret.as_float().unwrap_err(),
        secret.as_bool().unwrap_err(),
    ] {
        let text = format!("{error} {error:?}");
        assert!(!text.contains("hunter2"), "{text}");
    }
    let error = Value::Float(1234.5678).as_integer().unwrap_err();
    assert!(!format!("{error} {error:?}").contains("1234.5678"));
}

#[test]
fn save_writes_expressions_not_their_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.noml");
    std::env::set_var("NOML_SAVE_TEST_SECRET", "s3cr3t-value");
    fs::write(
        &path,
        "name = \"app\"\nsecret = env(\"NOML_SAVE_TEST_SECRET\")\nmax = @size(\"10MB\")\n\
         log = \"/var/log/${name}.log\"\n[db]\npassword = env(\"NOML_SAVE_TEST_SECRET\")\n",
    )
    .unwrap();

    let mut config = Config::from_file(&path).unwrap();
    assert_eq!(
        config.get("secret").unwrap().as_string().unwrap(),
        "s3cr3t-value"
    );
    config.set("version", "2.0").unwrap();
    config.save().unwrap();

    let saved = fs::read_to_string(&path).unwrap();
    assert!(!saved.contains("s3cr3t-value"), "{saved}");
    assert!(saved.contains("env(\"NOML_SAVE_TEST_SECRET\")"), "{saved}");
    assert!(saved.contains("@size(\"10MB\")"), "{saved}");
    assert!(saved.contains("${name}"), "{saved}");

    let reloaded = Config::from_file(&path).unwrap();
    assert_eq!(reloaded.as_value(), config.as_value());

    // A value that was changed is written as the new value
    let mut config = reloaded;
    config.set("db.password", "replaced").unwrap();
    config.save().unwrap();
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("\"replaced\""), "{saved}");
    assert!(!saved.contains("s3cr3t-value"), "{saved}");
}

#[cfg(unix)]
#[test]
fn save_replaces_a_symlink_instead_of_writing_through_it() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target.noml");
    fs::write(&target, "untouched = true\n").unwrap();
    let link = dir.path().join("link.noml");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let config = Config::from_string("name = \"x\"").unwrap();
    config.save_to_file(&link).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "untouched = true\n");
    assert!(fs::read_to_string(&link).unwrap().contains("name"));
}

#[test]
fn save_leaves_no_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.noml");
    fs::write(&path, "old = 1\n").unwrap();
    let config = Config::from_string("name = \"x\"").unwrap();
    config.save_to_file(&path).unwrap();
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["app.noml"]);
    assert!(fs::read_to_string(&path).unwrap().contains("name"));
}

#[test]
fn many_table_headers_parse_in_linear_time() {
    fn sections(n: usize, array: bool) -> String {
        let mut s = String::new();
        for i in 0..n {
            if array {
                let _ = writeln!(s, "[[log]]\nid = {i}");
            } else {
                let _ = writeln!(s, "[t{i}]\nid = {i}");
            }
        }
        s
    }
    for array in [false, true] {
        let small = sections(5_000, array);
        let large = sections(40_000, array);
        let start = Instant::now();
        noml::parse_raw(&small).unwrap();
        let small_time = start.elapsed();
        let start = Instant::now();
        noml::parse_raw(&large).unwrap();
        let large_time = start.elapsed();
        // Eight times the input; quadratic work would take about 64 times as long
        assert!(
            large_time < small_time * 30 + Duration::from_millis(200),
            "array={array}: {small_time:?} -> {large_time:?}"
        );
    }
}

#[test]
fn size_keeps_integer_precision() {
    let value = parse("a = @size(\"9007199254740993\")\nb = @size(\"1.5KB\")").unwrap();
    assert_eq!(
        value.get("a").unwrap().as_integer().unwrap(),
        9_007_199_254_740_993
    );
    assert_eq!(value.get("b").unwrap().as_integer().unwrap(), 1536);
}
