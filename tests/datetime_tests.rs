//! TOML date-time literals, typed native values and the interpolation switch
//! (0.10.0).

use noml::{
    parse, parse_raw, serialize_document, Config, Datetime, FieldType, Offset, Resolver,
    ResolverConfig, Schema, Value,
};

#[test]
fn toml_date_time_literals_parse() {
    let config = parse(
        r#"
        odt1 = 1979-05-27T07:32:00Z
        odt2 = 1979-05-27T00:32:00-07:00
        odt3 = 1979-05-27 07:32:00.999999Z
        ldt = 1979-05-27T07:32:00
        ld = 1979-05-27
        lt = 07:32:00
        list = [1979-05-27, 2000-01-01]
        inline = { at = 12:30:00 }
        "#,
    )
    .unwrap();

    let odt2 = config.get("odt2").unwrap().as_datetime().unwrap();
    assert_eq!(odt2.offset, Some(Offset::Custom { minutes: -420 }));
    assert_eq!(odt2.date.unwrap().year, 1979);
    assert_eq!(
        config
            .get("odt3")
            .unwrap()
            .as_datetime()
            .unwrap()
            .time
            .unwrap()
            .nanosecond,
        999_999_000
    );
    assert!(config
        .get("ld")
        .unwrap()
        .as_datetime()
        .unwrap()
        .time
        .is_none());
    assert!(config
        .get("lt")
        .unwrap()
        .as_datetime()
        .unwrap()
        .date
        .is_none());
    assert_eq!(config.get("list").unwrap().as_array().unwrap().len(), 2);
    assert!(config.get("inline.at").unwrap().is_datetime());
}

#[test]
fn invalid_dates_are_parse_errors_with_positions() {
    let error = parse("a = 1\nb = 1979-02-30\n").unwrap_err();
    let text = error.to_string();
    assert!(text.contains("line 2"), "{text}");
    assert!(text.contains("1979-02-30"), "{text}");
    assert!(parse("t = 25:00:00").is_err());
}

#[test]
fn date_keys_and_numbers_still_work() {
    let config = parse("2024-01-01 = \"new year\"\nyear = 2024\nneg = -1979").unwrap();
    assert_eq!(
        config.get("2024-01-01").unwrap().as_string().unwrap(),
        "new year"
    );
    assert_eq!(config.get("year").unwrap(), &Value::Integer(2024));
    assert_eq!(config.get("neg").unwrap(), &Value::Integer(-1979));
}

#[test]
fn date_times_round_trip_through_both_writers() {
    let source = "# when\ncreated = 1979-05-27 07:32:00z\nday = 1979-05-27\n";
    let document = parse_raw(source).unwrap();
    let text = serialize_document(&document).unwrap();
    assert!(
        text.contains("1979-05-27 07:32:00z"),
        "original spelling kept: {text}"
    );
    assert_eq!(parse(&text).unwrap(), parse(source).unwrap());

    let config = Config::from_string(source).unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("out.noml");
    config.save_to_file(&path).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("created = 1979-05-27T07:32:00Z"), "{saved}");
    assert_eq!(
        Config::from_file(&path).unwrap().as_value(),
        config.as_value()
    );
}

#[test]
fn date_times_interpolate_as_text() {
    let config = parse("day = 1979-05-27\nlabel = \"released ${day}\"").unwrap();
    assert_eq!(
        config.get("label").unwrap().as_string().unwrap(),
        "released 1979-05-27"
    );
}

#[test]
fn datetime_parses_from_str_and_serializes_as_text() {
    let dt: Datetime = "1979-05-27T07:32:00Z".parse().unwrap();
    assert_eq!(Value::from(dt).to_string(), "1979-05-27T07:32:00Z");
}

#[test]
fn sizes_and_durations_are_typed() {
    let config = parse("max = @size(\"10MB\")\nwait = @duration(\"1m30s\")").unwrap();
    assert_eq!(config.get("max").unwrap(), &Value::Size(10 << 20));
    assert_eq!(config.get("max").unwrap().as_integer().unwrap(), 10 << 20);
    assert_eq!(config.get("wait").unwrap(), &Value::Duration(90.0));
    assert_eq!(config.get("wait").unwrap().as_float().unwrap(), 90.0);

    let schema = Schema::new()
        .required_field("max", FieldType::Integer)
        .required_field("wait", FieldType::Float);
    assert!(schema.validate(&config).is_ok());
    let schema = Schema::new()
        .required_field("max", FieldType::Size)
        .required_field("wait", FieldType::Duration);
    assert!(schema.validate(&config).is_ok());
}

#[test]
fn interpolation_can_be_turned_off() {
    let mut config = ResolverConfig::default();
    config.interpolation = false;
    let document = parse_raw("a = 1\nb = \"${a}\"\nc = @size(\"${a}MB\")").unwrap();
    let error = Resolver::with_config(config.clone()).resolve(&document);
    // `@size("${a}MB")` is taken literally and is not a valid size
    assert!(error.is_err());

    let document = parse_raw("a = 1\nb = \"${a}\"").unwrap();
    let value = Resolver::with_config(config.clone())
        .resolve(&document)
        .unwrap();
    assert_eq!(value.get("b").unwrap().as_string().unwrap(), "${a}");

    let document = parse_raw("a = 1\nb = ${a}").unwrap();
    assert!(Resolver::with_config(config).resolve(&document).is_err());
}

#[cfg(feature = "chrono")]
#[test]
fn chrono_conversion() {
    let config = parse("at = 1979-05-27T07:32:00-08:00").unwrap();
    let at = config.get("at").unwrap().as_datetime().unwrap();
    assert_eq!(
        at.to_chrono().unwrap().to_rfc3339(),
        "1979-05-27T07:32:00-08:00"
    );
    let now = chrono::Utc::now();
    assert!(Value::from(now).is_datetime());
}
