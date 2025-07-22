        assert!(comments.iter().any(|c| c.text.contains("This is a comment")));
        assert!(comments.iter().any(|c| c.text.contains("Inline comment")));
        assert!(comments.iter().any(|c| c.text.contains("Another comment")));
        assert!(comments.iter().any(|c| c.text.contains("Comment in section")));
    }

    #[test]
    fn parse_function_calls() {
        let source = r#"
        api_key = env("API_KEY")
        fallback = env("FALLBACK", "default")
        "#;

        let doc = parse_string(source, None).unwrap();
        // This would require evaluation to test properly
        // For now, just ensure it parses without error
        assert!(doc.root.value.is_table());
    }

    #[test]
    fn parse_native_types() {
        let source = r#"
        max_file_size = @size("10MB")
        timeout = @duration("30s")
        "#;

        let doc = parse_string(source, None).unwrap();
        // This would require evaluation to test properly
        // For now, just ensure it parses without error
        assert!(doc.root.value.is_table());
    }

    #[test]
    fn parse_dotted_keys() {
        let source = r#"
        a.b.c = "nested"
        "quoted.key" = "value"
        x.y = { z = 1 }
        "#;

        let doc = parse_string(source, None).unwrap();
        let value = doc.to_value().unwrap();

        assert_eq!(value.get("a.b.c").unwrap().as_string().unwrap(), "nested");
        assert_eq!(value.get("quoted.key").unwrap().as_string().unwrap(), "value");
        assert_eq!(value.get("x.y.z").unwrap().as_integer().unwrap(), 1);
    }
}