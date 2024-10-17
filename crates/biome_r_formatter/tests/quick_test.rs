use biome_formatter_test::check_reformat::CheckReformat;
use biome_r_formatter::context::RFormatOptions;
use biome_r_formatter::format_node;
use biome_r_formatter::RFormatLanguage;
use biome_r_parser::parse;
use biome_r_parser::RParserOptions;

mod language {
    include!("language.rs");
}

// Use this test check if your snippet prints as you wish, without using a snapshot
#[ignore]
#[test]
fn quick_test() {
    let src = r#"1 + 1"#;

    let parse = parse(src, RParserOptions::default());

    let options = RFormatOptions::default();
    let result = format_node(options.clone(), &parse.syntax())
        .unwrap()
        .print()
        .unwrap();

    let root = &parse.syntax();
    let language = language::RTestFormatLanguage::default();

    // Does a second pass of formatting to ensure nothing changes (i.e. stable)
    let check_reformat = CheckReformat::new(
        root,
        result.as_code(),
        "quick_test",
        &language,
        RFormatLanguage::new(options),
    );
    check_reformat.check_reformat();

    assert_eq!(result.as_code(), r#"1 + 1"#);
}
