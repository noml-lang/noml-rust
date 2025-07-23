<div align="center">
    <h1>
        <span>NOML +&nbsp; RUST</span>
        <br>
        <sub><sup>PARSER &amp; GENERATOR</sup></sub>
    </h1>
</div>

<div align="center">
    <div>
        <a href="https://crates.io/crates/noml" alt="NOML on Crates.io"><img alt="Crates.io" src="https://img.shields.io/crates/v/noml"></a>
        <span>&nbsp;</span>
        <a href="https://crates.io/crates/noml" alt="Download NOML"><img alt="Crates.io Downloads" src="https://img.shields.io/crates/d/noml?color=%230099ff"></a>
        <span>&nbsp;</span>
        <a href="https://docs.rs/noml" title="NOML Documentation"><img alt="docs.rs" src="https://img.shields.io/docsrs/noml"></a>
        <span>&nbsp;</span>
        <img alt="GitHub last commit" src="https://img.shields.io/github/last-commit/noml-lang/noml-rust?membase=%23347d39" alt="last commit badge">
    </div>
</div>
<br><br>
<p><strong>Nested-Object Markup Language</strong> (<strong>NOML</strong>) is a powerful, modern configuration language designed for clarity, ease of use, and a rich feature set. This crate provides a blazing-fast and full-fidelity parser and generator for <code>noml</code> in Rust.</p>
<p><strong>NOML</strong> combines the simplicity of <abbr title="Tom's Obvious, Minimal Language"><b>TOML</b></abbr> with advanced, developer-friendly features, making it an ideal choice for a wide range of applications, from simple configuration files to complex, dynamic settings.</p>

<br>


<h2 align="center">
    ⚠️<br>
    PRE-RELEASE<br>
    <sup><sub>PROJECT IN-DEVELOPMENT</sub></sup>
    <br><br>
</h2>
<br>

<h2>Key Features:</h2>
<ul>
    <li>
        <b>Intuitive, TOML-like Syntax:</b> &nbsp; Easy to read and write, with a familiar structure.
    </li>
    <li>
        <b>Environment Variable Interpolation:</b> &nbsp; Seamlessly pull in configuration from the environment with <code>env("VAR_NAME", "default_value")</code>.
    </li>
    <li>
        <b>File Imports:</b> &nbsp; Organize your configuration into multiple files with <code>include "path/to/file.noml"</code>.
    </li>
    <li>
        <b>Variable Interpolation:</b> &nbsp; Reference other values in your configuration with <code>${path.to.variable}</code>.
    </li>
    <li>
        <b>Native Types:</b> &nbsp; Go beyond simple primitives with built-in types like <code>@size("10MB")</code>, <code>@duration("30s")</code>, and <code>@url("https://example.com")</code>.
    </li>
    <li>
        <b>Full Fidelity Parsing:</b> &nbsp; The parser preserves all comments, whitespace, and formatting, allowing you to programmatically edit and save NOML files without losing any information.
    </li>
    <li>
        <b>Blazing Fast:</b> &nbsp; Built with performance in mind, featuring a zero-copy lexer and an efficient, hand-written parser.
    </li>
    <li>
        <b>Excellent Error Reporting:</b> &nbsp; Get clear, detailed error messages with precise source locations to quickly debug your configuration files.
    </li>
    <li>
        <b>High-Level Config Management:</b> &nbsp; A simple and powerful API for loading, modifying, and saving configurations.
    </li>
    <li>
        <b>Schema Validation:</b> &nbsp; Define a schema to validate your configuration files and ensure they have the correct structure and types. (Coming soon!).
    </li>
</ul>


<br><br><br>