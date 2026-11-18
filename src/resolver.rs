//! # NOML Resolver
//!
//! This module turns a parsed [`Document`] into a [`Value`] tree and resolves
//! the dynamic parts of NOML on the way:
//!
//! - Environment variable lookups via `env("NAME", default)`
//! - File inclusion via `include "path"` (local, and HTTP with the `async` feature)
//! - Variable interpolation via `${path}`
//! - Native types via `@type(...)`
//!
//! ## Interpolation
//!
//! `${path}` refers to another value in the same document by its dotted path
//! from the document root, for example `${app_name}`, `${database.host}` or
//! `${servers.0.name}` (array index). It works in two places:
//!
//! - inside double-quoted strings (`"..."` and `"""..."""`), where the value is
//!   converted to text: `log = "/var/log/${app_name}.log"`;
//! - as a bare value, where the referenced value is copied with its type, so
//!   `port = ${defaults.port}` stays an integer and `${server}` can copy a whole
//!   table.
//!
//! Single-quoted (`'...'`, `'''...'''`) and raw (`r"..."`) strings are literal
//! and never interpolated. Inside a double-quoted string, `$${` produces a
//! literal `${`.
//!
//! References may point at values that use interpolation themselves, in any
//! order. A reference cycle (`a = "${b}"`, `b = "${a}"`) is reported as
//! [`NomlError::CircularReference`]; a path that does not exist is reported as
//! [`NomlError::Interpolation`] with its line and column.
//!
//! Inside a file pulled in with `include`, paths are looked up in the included
//! file first and then in the including document. Variables registered with
//! [`Resolver::set_variable`] are used when the document has no value at the
//! path.
//!
//! ## Limits
//!
//! A few lines of `${...}` can describe a value far larger than the text
//! (each reference copies the value it names), so every resolve is bounded
//! by [`ResolverLimits`]. By default the values copied by references may add
//! up to [`DEFAULT_MAX_OUTPUT_BYTES`] (64 MiB), a document may use
//! [`DEFAULT_MAX_REFERENCES`] (10,000) references, a reference to a value
//! that is itself a reference may be chained [`DEFAULT_MAX_REFERENCE_DEPTH`]
//! (32) levels deep, and [`DEFAULT_MAX_INCLUDES`] (256) files may be included
//! in total. Going over a limit is a [`NomlError::Parse`] error with the line
//! and column of the reference or include; the message does not quote the
//! document. Change the limits with [`Resolver::with_limits`].
//!
//! ## Includes
//!
//! `include "path"` reads a file relative to the including file. By default
//! the target must be inside the include root: the directory set with
//! [`Resolver::with_include_root`], or else the base path (the directory of
//! the file being parsed, for [`parse_from_file`](crate::parse_from_file)).
//! The check is made on the canonical path, so neither `..`, an absolute
//! path nor a symbolic link can leave the root. Text parsed without a base
//! path or include root (for example with [`parse`](crate::parse)) cannot
//! include files at all. [`Resolver::allow_includes_anywhere`] turns the
//! check off, and `max_include_depth = 0` in [`ResolverConfig`] refuses every
//! include.
//!
//! With the `async` feature, `include "https://..."` fetches a remote
//! document. Remote documents are fetched over HTTPS only, redirects are not
//! followed, and the body is limited to
//! [`ResolverLimits::max_http_body_bytes`]. A remote document cannot use
//! `include` or `env()`, and its `${...}` references can only name values in
//! the remote document itself.

use crate::error::{NomlError, Result};
use crate::parser::ast::{AstNode, AstValue, Document, Span, StringStyle};
use crate::parser::parse_file;
use crate::tree::{self, Seg};
use crate::value::Value;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "async")]
use std::time::Duration;

/// Configuration for the resolver.
///
/// Start from [`ResolverConfig::default()`] and set the fields you need; the
/// struct is `#[non_exhaustive]` so options can be added without breaking
/// your code.
///
/// ```rust
/// use noml::ResolverConfig;
///
/// let mut config = ResolverConfig::default();
/// config.allow_missing_env = true;
/// config.interpolation = false; // read `${...}` as plain text
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResolverConfig {
    /// Base path for resolving relative includes
    pub base_path: Option<PathBuf>,
    /// Environment variables to use (if None, uses std::env)
    pub env_vars: Option<HashMap<String, String>>,
    /// Maximum include depth to prevent infinite recursion
    pub max_include_depth: usize,
    /// Whether to allow missing environment variables
    pub allow_missing_env: bool,
    /// Custom native type resolvers
    pub native_resolvers: HashMap<String, NativeResolver>,
    /// Whether `${...}` is interpolated (default `true`).
    ///
    /// With `false`, double-quoted strings are taken literally, as TOML
    /// does, and a bare `${path}` value is an error.
    pub interpolation: bool,
    /// HTTP client timeout for remote includes (async feature only)
    #[cfg(feature = "async")]
    pub http_timeout: Duration,
    /// Cache for HTTP includes to avoid repeated requests
    #[cfg(feature = "async")]
    pub http_cache: Option<HashMap<String, String>>,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        let mut native_resolvers = HashMap::new();

        // Register built-in native types
        native_resolvers.insert("size".to_string(), NativeResolver::new(resolve_size));
        native_resolvers.insert(
            "duration".to_string(),
            NativeResolver::new(resolve_duration),
        );
        native_resolvers.insert("regex".to_string(), NativeResolver::new(resolve_regex));
        native_resolvers.insert("url".to_string(), NativeResolver::new(resolve_url));
        native_resolvers.insert("ip".to_string(), NativeResolver::new(resolve_ip));
        native_resolvers.insert("semver".to_string(), NativeResolver::new(resolve_semver));
        native_resolvers.insert("base64".to_string(), NativeResolver::new(resolve_base64));
        native_resolvers.insert("uuid".to_string(), NativeResolver::new(resolve_uuid));

        Self {
            base_path: None,
            env_vars: None,
            max_include_depth: 10,
            allow_missing_env: false,
            native_resolvers,
            interpolation: true,
            #[cfg(feature = "async")]
            http_timeout: Duration::from_secs(30),
            #[cfg(feature = "async")]
            http_cache: Some(HashMap::new()),
        }
    }
}

/// Default for [`ResolverLimits::max_output_bytes`]: 64 MiB.
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// Default for [`ResolverLimits::max_references`]: 10,000.
pub const DEFAULT_MAX_REFERENCES: usize = 10_000;

/// Default for [`ResolverLimits::max_reference_depth`]: 32.
pub const DEFAULT_MAX_REFERENCE_DEPTH: usize = 32;

/// Default for [`ResolverLimits::max_includes`]: 256.
pub const DEFAULT_MAX_INCLUDES: usize = 256;

/// Default for [`ResolverLimits::max_http_body_bytes`]: 1 MiB.
pub const DEFAULT_MAX_HTTP_BODY_BYTES: usize = 1024 * 1024;

/// Limits that bound the work and memory of one resolve.
///
/// Start from [`ResolverLimits::new`] (the defaults) and change single
/// limits with the `with_*` methods, then pass the result to
/// [`Resolver::with_limits`]. See the [module documentation](self#limits).
///
/// # Example
///
/// ```rust
/// use noml::{parse_string, Resolver, ResolverLimits};
///
/// let limits = ResolverLimits::new()
///     .with_max_output_bytes(1024 * 1024)
///     .with_max_reference_depth(8);
/// let mut resolver = Resolver::new().with_limits(limits);
///
/// let document = parse_string(r#"a = "x"
/// b = "${a}${a}""#, None)?;
/// let value = resolver.resolve(&document)?;
/// assert_eq!(value.get("b").unwrap().as_string()?, "xx");
/// # Ok::<(), noml::NomlError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolverLimits {
    max_output_bytes: usize,
    max_references: usize,
    max_reference_depth: usize,
    max_includes: usize,
    max_http_body_bytes: usize,
}

impl Default for ResolverLimits {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolverLimits {
    /// The default limits.
    pub const fn new() -> Self {
        Self {
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            max_references: DEFAULT_MAX_REFERENCES,
            max_reference_depth: DEFAULT_MAX_REFERENCE_DEPTH,
            max_includes: DEFAULT_MAX_INCLUDES,
            max_http_body_bytes: DEFAULT_MAX_HTTP_BODY_BYTES,
        }
    }

    /// Set the largest total size, in bytes, of the values that `${...}`
    /// references copy in one resolve. Default [`DEFAULT_MAX_OUTPUT_BYTES`].
    ///
    /// The size is an estimate of the memory the copies use: the bytes of
    /// each string and key plus a fixed cost for every value, so a copied
    /// array or table is charged for each of its elements.
    pub fn with_max_output_bytes(mut self, bytes: usize) -> Self {
        self.max_output_bytes = bytes;
        self
    }

    /// Set the largest number of `${...}` references resolved in one
    /// resolve, included files counted. Default [`DEFAULT_MAX_REFERENCES`].
    pub fn with_max_references(mut self, count: usize) -> Self {
        self.max_references = count;
        self
    }

    /// Set the longest chain of references: a value that refers to a value
    /// that is itself a reference, and so on. A reference to a plain value
    /// is a chain of one. Default [`DEFAULT_MAX_REFERENCE_DEPTH`].
    pub fn with_max_reference_depth(mut self, depth: usize) -> Self {
        self.max_reference_depth = depth;
        self
    }

    /// Set the largest number of files (and HTTP documents) included in one
    /// resolve, over all levels. Default [`DEFAULT_MAX_INCLUDES`].
    pub fn with_max_includes(mut self, count: usize) -> Self {
        self.max_includes = count;
        self
    }

    /// Set the largest body, in bytes, read for an HTTP include (`async`
    /// feature). Default [`DEFAULT_MAX_HTTP_BODY_BYTES`].
    pub fn with_max_http_body_bytes(mut self, bytes: usize) -> Self {
        self.max_http_body_bytes = bytes;
        self
    }

    /// Largest total size of the values copied by references, in bytes.
    pub const fn max_output_bytes(&self) -> usize {
        self.max_output_bytes
    }

    /// Largest number of references.
    pub const fn max_references(&self) -> usize {
        self.max_references
    }

    /// Longest chain of references.
    pub const fn max_reference_depth(&self) -> usize {
        self.max_reference_depth
    }

    /// Largest number of included files.
    pub const fn max_includes(&self) -> usize {
        self.max_includes
    }

    /// Largest body read for an HTTP include, in bytes.
    pub const fn max_http_body_bytes(&self) -> usize {
        self.max_http_body_bytes
    }
}

/// Type alias for native resolver functions
type NativeResolverFn = Arc<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>;

/// A native type resolver function, used for `@name(...)` values.
///
/// Cloning is cheap: clones share the same function.
#[derive(Clone)]
pub struct NativeResolver {
    resolver: NativeResolverFn,
}

impl NativeResolver {
    /// Creates a new `NativeResolver` from the given resolver function.
    pub fn new<F>(resolver: F) -> Self
    where
        F: Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
    {
        Self {
            resolver: Arc::new(resolver),
        }
    }

    /// Resolves the native type using the provided arguments.
    pub fn resolve(&self, args: &[Value]) -> Result<Value> {
        (self.resolver)(args)
    }
}

impl std::fmt::Debug for NativeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NativeResolver")
    }
}

/// The main resolver for NOML documents
pub struct Resolver {
    config: ResolverConfig,
    include_stack: Vec<PathBuf>,
    variables: IndexMap<String, Value>,
    limits: ResolverLimits,
    /// Directory includes must stay in (the base path when `None`)
    include_root: Option<PathBuf>,
    /// Skip the include root check
    includes_anywhere: bool,
    /// Canonical include root, worked out on the first include of a resolve
    canonical_root: Option<PathBuf>,
    /// Includes read so far in this resolve
    include_count: usize,
    /// Greater than zero while building a document fetched over HTTP
    remote_depth: usize,
    /// When set, the dynamic expressions of the top-level document and
    /// their locations are recorded here (used by `Config::save`)
    record: Option<Vec<Recorded>>,
    /// Bodies of HTTP includes fetched by [`Resolver::resolve_document_async`]
    #[cfg(feature = "async")]
    http_content: HashMap<String, String>,
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}

/// An expression of the top-level document, where its value went, and the
/// value it produced if it was known while building
pub(crate) type Recorded = (Vec<Seg>, AstNode, Option<Value>);

/// Lookup scopes for `${...}`: innermost include location first, the document
/// root last.
struct ScopeSet {
    paths: Vec<Vec<Seg>>,
    /// The value comes from a document fetched over HTTP: no `env()`, and
    /// references only within that document
    remote: bool,
}

type Scopes = Rc<ScopeSet>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Active,
    Done,
}

/// A value that needs interpolation, resolved after the rest of the tree is built.
struct Pending {
    /// Where the value goes in the output tree
    location: Vec<Seg>,
    /// The AST node to evaluate
    node: AstNode,
    /// Lookup scopes for `${...}` in this value
    scopes: Scopes,
    state: State,
    /// Length of the longest reference chain ending here (set when done)
    depth: usize,
}

/// State for a single resolve call
struct Run {
    root: Value,
    pending: Vec<Pending>,
    by_location: BTreeMap<Vec<Seg>, usize>,
    /// Pending values being worked on, outermost first (for cycle reports).
    /// Each one waits on the next.
    active: Vec<usize>,
    /// For each evaluation in progress, the deepest chain it has used so far
    dep_depth: Vec<usize>,
    /// Bytes charged to `${...}` copies so far
    output_bytes: usize,
    /// References resolved so far
    references: usize,
}

impl Run {
    fn new() -> Self {
        Self {
            root: Value::Null,
            pending: Vec::new(),
            by_location: BTreeMap::new(),
            active: Vec::new(),
            dep_depth: Vec::new(),
            output_bytes: 0,
            references: 0,
        }
    }

    fn defer(&mut self, location: Vec<Seg>, node: &AstNode, scopes: &Scopes) {
        self.by_location
            .insert(location.clone(), self.pending.len());
        self.pending.push(Pending {
            location,
            node: node.clone(),
            scopes: Rc::clone(scopes),
            state: State::Waiting,
            depth: 0,
        });
    }

    /// The evaluation in progress used the finished pending value `index`
    fn note_dependency(&mut self, index: usize) {
        let depth = self.pending[index].depth;
        if let Some(deepest) = self.dep_depth.last_mut() {
            *deepest = (*deepest).max(depth);
        }
    }
}

/// Outcome of looking for a path without settling anything
enum Probe {
    Found,
    Missing,
    /// A pending value is in the way; it has to be settled first
    Blocked,
}

/// Result of the first pass over a node
enum Built {
    Value(Value),
    /// The node needs interpolation; it is evaluated in the second pass
    Deferred,
}

impl Resolver {
    /// Create a new resolver with default configuration
    pub fn new() -> Self {
        Self::with_config(ResolverConfig::default())
    }

    /// Create a new resolver with custom configuration
    pub fn with_config(config: ResolverConfig) -> Self {
        Self {
            config,
            include_stack: Vec::new(),
            variables: IndexMap::new(),
            limits: ResolverLimits::new(),
            include_root: None,
            includes_anywhere: false,
            canonical_root: None,
            include_count: 0,
            remote_depth: 0,
            record: None,
            #[cfg(feature = "async")]
            http_content: HashMap::new(),
        }
    }

    /// Set the base path for resolving relative includes.
    ///
    /// Unless [`Resolver::with_include_root`] names another directory, the
    /// base path is also the include root: included files must be inside it.
    pub fn with_base_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.config.base_path = Some(path.into());
        self
    }

    /// Set the limits for each resolve (see [`ResolverLimits`]).
    pub fn with_limits(mut self, limits: ResolverLimits) -> Self {
        self.limits = limits;
        self
    }

    /// The limits applied to each resolve.
    pub fn limits(&self) -> &ResolverLimits {
        &self.limits
    }

    /// Allow includes only inside `root` (and its subdirectories).
    ///
    /// Relative includes are still resolved against the including file, or
    /// the base path for the top-level document (`root` when no base path is
    /// set). The check is made on canonical paths, so `..`, absolute paths
    /// and symbolic links cannot leave the root. Without this setting the
    /// root is the base path.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use noml::{parse_string, Resolver};
    ///
    /// // `config/app.noml` may include files from all of `config/`
    /// let source = std::fs::read_to_string("config/app.noml")?;
    /// let document = parse_string(&source, None)?;
    /// let mut resolver = Resolver::new().with_include_root("config");
    /// let value = resolver.resolve(&document)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_include_root<P: Into<PathBuf>>(mut self, root: P) -> Self {
        self.include_root = Some(root.into());
        self
    }

    /// Allow includes of any file the process can read, with no root check.
    ///
    /// Relative includes in text without a base path are then resolved
    /// against the current directory, as before version 0.9.4. Only use this
    /// for documents you trust as much as the program itself.
    pub fn allow_includes_anywhere(mut self, allow: bool) -> Self {
        self.includes_anywhere = allow;
        self
    }

    /// Set custom environment variables
    pub fn with_env_vars(mut self, env_vars: HashMap<String, String>) -> Self {
        self.config.env_vars = Some(env_vars);
        self
    }

    /// Add a custom native type resolver
    pub fn with_native_resolver<S: Into<String>>(
        mut self,
        name: S,
        resolver: NativeResolver,
    ) -> Self {
        self.config.native_resolvers.insert(name.into(), resolver);
        self
    }

    /// Resolve a document, processing all includes, interpolations, and function calls.
    ///
    /// See the [module documentation](self) for how `${...}` is resolved.
    pub fn resolve(&mut self, document: &Document) -> Result<Value> {
        self.include_stack.clear();
        self.resolve_root(&document.root)
    }

    /// Resolve a document.
    ///
    /// Same as [`Resolver::resolve`], which builds the interpolation context
    /// itself; kept for compatibility.
    pub fn resolve_with_context(&mut self, document: &Document) -> Result<Value> {
        self.resolve(document)
    }

    /// Resolve a single node as if it were the root of a document.
    pub(crate) fn resolve_root(&mut self, root: &AstNode) -> Result<Value> {
        self.canonical_root = None;
        self.include_count = 0;
        self.remote_depth = 0;
        let mut run = Run::new();
        let scopes: Scopes = Rc::new(ScopeSet {
            paths: vec![Vec::new()],
            remote: false,
        });
        let mut location = Vec::new();
        run.root = match self.build(root, &mut location, &scopes, &mut run)? {
            Built::Value(value) => value,
            Built::Deferred => {
                run.defer(Vec::new(), root, &scopes);
                Value::Null
            }
        };

        let mut i = 0;
        while i < run.pending.len() {
            self.settle(i, &mut run)?;
            i += 1;
        }
        Ok(run.root)
    }

    /// Resolve `document` and also return the dynamic expressions of the
    /// document itself (`env()`, `@type()`, `${...}`, `include`, templates)
    /// with their locations, for writing them back on save.
    pub(crate) fn resolve_recording(
        &mut self,
        document: &Document,
    ) -> Result<(Value, Vec<Recorded>)> {
        self.record = Some(Vec::new());
        let result = self.resolve(document);
        let recorded = self.record.take().unwrap_or_default();
        result.map(|value| (value, recorded))
    }

    /// Set a variable for interpolation.
    ///
    /// Variables are used when the document itself has no value at the
    /// referenced path. A variable named `app` holding a table can be reached
    /// with `${app.name}`.
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Get all variables set with [`Resolver::set_variable`]
    pub fn variables(&self) -> &IndexMap<String, Value> {
        &self.variables
    }

    /// Clear all variables
    pub fn clear_variables(&mut self) {
        self.variables.clear();
    }

    // ------------------------------------------------------------------
    // First pass: build the value tree, deferring anything that interpolates
    // ------------------------------------------------------------------

    fn build(
        &mut self,
        node: &AstNode,
        location: &mut Vec<Seg>,
        scopes: &Scopes,
        run: &mut Run,
    ) -> Result<Built> {
        let recording = self.record.is_some()
            && self.include_stack.is_empty()
            && self.remote_depth == 0
            && is_expression(node);
        let built = self.build_node(node, location, scopes, run)?;
        if recording {
            // The value an include or a call produced, before any section
            // with the same key is merged into it; deferred values are read
            // from the finished tree
            let value = match &built {
                Built::Value(value) => Some(value.clone()),
                Built::Deferred => None,
            };
            if let Some(record) = &mut self.record {
                record.push((location.clone(), node.clone(), value));
            }
        }
        Ok(built)
    }

    fn build_node(
        &mut self,
        node: &AstNode,
        location: &mut Vec<Seg>,
        scopes: &Scopes,
        run: &mut Run,
    ) -> Result<Built> {
        let value = match &node.value {
            AstValue::Null => Value::Null,
            AstValue::Bool(b) => Value::Bool(*b),
            AstValue::Integer { value, .. } => Value::Integer(*value),
            AstValue::Float { value, .. } => Value::Float(*value),
            AstValue::DateTime { value, .. } => Value::DateTime(*value),
            AstValue::String { value, style, .. } => {
                if self.config.interpolation && is_template(value, *style) {
                    return Ok(Built::Deferred);
                }
                Value::String(value.clone())
            }
            AstValue::Array { elements, .. } => {
                let mut items = Vec::with_capacity(elements.len());
                for (i, element) in elements.iter().enumerate() {
                    location.push(Seg::Index(i));
                    check_depth(location.len(), &element.span)?;
                    let item = match self.build(element, location, scopes, run)? {
                        Built::Value(v) => v,
                        Built::Deferred => {
                            run.defer(location.clone(), element, scopes);
                            Value::Null
                        }
                    };
                    location.pop();
                    items.push(item);
                }
                Value::Array(items)
            }
            AstValue::Table { entries, .. } => {
                let mut table = BTreeMap::new();
                for entry in entries {
                    let base = location.len();
                    location.extend(tree::physical_path(&table, &entry.key));
                    check_depth(location.len(), &entry.key.span)?;
                    let item = match self.build(&entry.value, location, scopes, run)? {
                        Built::Value(v) => v,
                        Built::Deferred => {
                            run.defer(location.clone(), &entry.value, scopes);
                            Value::Null
                        }
                    };
                    location.truncate(base);
                    tree::insert(&mut table, &entry.key, item)?;
                }
                Value::Table(table)
            }
            AstValue::FunctionCall { args, .. } | AstValue::Native { args, .. } => {
                if self.config.interpolation && args.iter().any(has_template) {
                    return Ok(Built::Deferred);
                }
                let values = args
                    .iter()
                    .map(|arg| self.literal(arg))
                    .collect::<Result<Vec<_>>>()?;
                self.call(node, values, self.remote_depth > 0)?
            }
            AstValue::Interpolation { path } => {
                if !self.config.interpolation {
                    return Err(interpolation_error(
                        format!(
                            "Interpolation is disabled, but '${{{path}}}' is used at line {}, column {}",
                            node.span.start_line, node.span.start_column
                        ),
                        path,
                    ));
                }
                return Ok(Built::Deferred);
            }
            AstValue::Include { path } => {
                self.resolve_include(path, &node.span, location, scopes, run)?
            }
        };
        Ok(Built::Value(value))
    }

    /// Evaluate a function argument that contains no interpolation
    fn literal(&mut self, node: &AstNode) -> Result<Value> {
        self.evaluate(node, &mut None)
    }

    /// Evaluate `env(...)` or `@native(...)` with already-evaluated arguments.
    /// `remote` is set for values from a document fetched over HTTP, which
    /// may not read the environment.
    fn call(&self, node: &AstNode, args: Vec<Value>, remote: bool) -> Result<Value> {
        let span = &node.span;
        match &node.value {
            AstValue::FunctionCall { name, .. } if name == "env" && remote => {
                Err(NomlError::parse(
                    "env() is not allowed in a document included over HTTP",
                    span.start_line,
                    span.start_column,
                ))
            }
            AstValue::FunctionCall { name, .. } if name == "env" => self.call_env(args, span),
            AstValue::FunctionCall { name, .. } => Err(NomlError::unknown_function(
                name,
                span.start_line,
                span.start_column,
            )),
            AstValue::Native { type_name, .. } => {
                let resolver = self.config.native_resolvers.get(type_name).ok_or_else(|| {
                    NomlError::unknown_native_type(type_name, span.start_line, span.start_column)
                })?;
                resolver.resolve(&args).map_err(|e| locate(e, span))
            }
            _ => Err(NomlError::internal("call() on a non-call node")),
        }
    }

    fn call_env(&self, args: Vec<Value>, span: &Span) -> Result<Value> {
        if args.is_empty() || args.len() > 2 {
            return Err(NomlError::parse(
                "env() requires 1 or 2 arguments",
                span.start_line,
                span.start_column,
            ));
        }
        let mut args = args.into_iter();
        let var_name = match args.next() {
            Some(Value::String(name)) => name,
            _ => {
                return Err(NomlError::parse(
                    "env() first argument must be a string",
                    span.start_line,
                    span.start_column,
                ))
            }
        };
        let default_value = args.next();

        let env_value = if let Some(ref env_vars) = self.config.env_vars {
            env_vars.get(&var_name).cloned()
        } else {
            env::var(&var_name).ok()
        };

        if let Some(val) = env_value {
            Ok(Value::String(val))
        } else if let Some(default) = default_value {
            Ok(default)
        } else if self.config.allow_missing_env {
            Ok(Value::Null)
        } else {
            Err(NomlError::parse(
                format!("Environment variable '{var_name}' not found and no default provided"),
                span.start_line,
                span.start_column,
            ))
        }
    }

    // ------------------------------------------------------------------
    // Second pass: evaluate deferred values, resolving references on demand
    // ------------------------------------------------------------------

    /// Evaluate the pending value `index` (and first everything it refers
    /// to) and write it into the tree.
    ///
    /// The values a pending value refers to are found before it is
    /// evaluated and settled first, from an explicit work stack, so a long
    /// chain of references never deepens the Rust stack. A reference chain
    /// longer than [`ResolverLimits::max_reference_depth`] is an error.
    fn settle(&mut self, index: usize, run: &mut Run) -> Result<()> {
        match run.pending[index].state {
            State::Done => {
                run.note_dependency(index);
                return Ok(());
            }
            State::Active => return Err(cycle_error(run, index)),
            State::Waiting => {}
        }

        let mut work = vec![index];
        let mut blockers = Vec::new();
        while let Some(&top) = work.last() {
            match run.pending[top].state {
                State::Done => {
                    work.pop();
                    continue;
                }
                State::Waiting => {
                    // Every active value waits on the next one, so this is
                    // a lower bound for the length of the chain
                    if run.active.len() >= self.limits.max_reference_depth {
                        return Err(reference_depth_error(
                            self.limits.max_reference_depth,
                            &run.pending[top].node.span,
                        ));
                    }
                    run.pending[top].state = State::Active;
                    run.active.push(top);
                }
                // Its dependencies have been settled; look again
                State::Active => {}
            }

            blockers.clear();
            self.collect_blockers(top, run, &mut blockers);
            if blockers.is_empty() {
                self.evaluate_pending(top, run)?;
                work.pop();
            } else {
                work.extend(blockers.drain(..).rev());
            }
        }
        run.note_dependency(index);
        Ok(())
    }

    /// Evaluate the active pending value `index` whose references are all
    /// settled, and write it into the tree.
    fn evaluate_pending(&mut self, index: usize, run: &mut Run) -> Result<()> {
        run.dep_depth.push(0);
        let node = run.pending[index].node.clone();
        let value = self.evaluate(&node, &mut Some((index, &mut *run)))?;
        if matches!(node.value, AstValue::Interpolation { .. }) {
            // A bare ${...} copies a subtree; keep the result within the limit
            check_depth(
                run.pending[index].location.len() + value_depth(&value),
                &node.span,
            )?;
        }
        let depth = run.dep_depth.pop().unwrap_or(0) + 1;
        if depth > self.limits.max_reference_depth {
            return Err(reference_depth_error(
                self.limits.max_reference_depth,
                &node.span,
            ));
        }

        if let Some(position) = run.active.iter().rposition(|&i| i == index) {
            run.active.remove(position);
        }
        let pending = &mut run.pending[index];
        pending.state = State::Done;
        pending.depth = depth;
        let slot = tree::get_mut(&mut run.root, &pending.location)
            .ok_or_else(|| NomlError::internal("interpolation target disappeared"))?;
        *slot = value;
        Ok(())
    }

    /// Add to `out` the waiting pending values that the references in
    /// pending value `index` would have to settle. Nothing is settled.
    fn collect_blockers(&self, index: usize, run: &Run, out: &mut Vec<usize>) {
        let pending = &run.pending[index];
        self.node_blockers(&pending.node, &pending.scopes, run, out);
    }

    fn node_blockers(&self, node: &AstNode, scopes: &ScopeSet, run: &Run, out: &mut Vec<usize>) {
        match &node.value {
            AstValue::String { value, style, .. } if is_template(value, *style) => {
                for_each_reference(value, |expression| {
                    self.reference_blockers(expression, scopes, run, out);
                });
            }
            AstValue::Interpolation { path } => self.reference_blockers(path, scopes, run, out),
            AstValue::Array { elements, .. } => {
                for element in elements {
                    self.node_blockers(element, scopes, run, out);
                }
            }
            AstValue::Table { entries, .. } => {
                for entry in entries {
                    self.node_blockers(&entry.value, scopes, run, out);
                }
            }
            AstValue::FunctionCall { args, .. } | AstValue::Native { args, .. } => {
                for arg in args {
                    self.node_blockers(arg, scopes, run, out);
                }
            }
            _ => {}
        }
    }

    /// The same search as [`Resolver::lookup`], without settling anything
    fn reference_blockers(
        &self,
        expression: &str,
        scopes: &ScopeSet,
        run: &Run,
        out: &mut Vec<usize>,
    ) {
        // A malformed reference is reported when the value is evaluated
        let Ok(names) = parse_reference(expression) else {
            return;
        };
        for scope in &scopes.paths {
            match probe(scope, &names, run, out) {
                Probe::Found | Probe::Blocked => return,
                Probe::Missing => {}
            }
        }
    }

    /// Fully evaluate a node. With `ctx` set, `${...}` is resolved against the
    /// tree being built; without it, any interpolation is an error.
    fn evaluate(&mut self, node: &AstNode, ctx: &mut Option<(usize, &mut Run)>) -> Result<Value> {
        let span = &node.span;
        let remote = match ctx {
            Some((index, run)) => run.pending[*index].scopes.remote,
            None => self.remote_depth > 0,
        };
        match &node.value {
            AstValue::Null => Ok(Value::Null),
            AstValue::Bool(b) => Ok(Value::Bool(*b)),
            AstValue::Integer { value, .. } => Ok(Value::Integer(*value)),
            AstValue::Float { value, .. } => Ok(Value::Float(*value)),
            AstValue::DateTime { value, .. } => Ok(Value::DateTime(*value)),
            AstValue::String { value, style, .. } => {
                if self.config.interpolation && is_template(value, *style) {
                    Ok(Value::String(self.render(value, span, ctx)?))
                } else {
                    Ok(Value::String(value.clone()))
                }
            }
            AstValue::Interpolation { path } => self.lookup(path, span, ctx),
            AstValue::Array { elements, .. } => {
                let mut items = Vec::with_capacity(elements.len());
                for element in elements {
                    items.push(self.evaluate(element, ctx)?);
                }
                Ok(Value::Array(items))
            }
            AstValue::Table { entries, .. } => {
                let mut table = BTreeMap::new();
                for entry in entries {
                    let value = self.evaluate(&entry.value, ctx)?;
                    tree::insert(&mut table, &entry.key, value)?;
                }
                Ok(Value::Table(table))
            }
            AstValue::FunctionCall { args, .. } | AstValue::Native { args, .. } => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.evaluate(arg, ctx)?);
                }
                self.call(node, values, remote)
            }
            AstValue::Include { .. } => Err(NomlError::parse(
                "include can only be used as the value of a key or array element",
                span.start_line,
                span.start_column,
            )),
        }
    }

    /// Expand `${...}` in a double-quoted string
    fn render(
        &mut self,
        text: &str,
        span: &Span,
        ctx: &mut Option<(usize, &mut Run)>,
    ) -> Result<String> {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(pos) = rest.find('$') {
            out.push_str(&rest[..pos]);
            let tail = &rest[pos..];
            if let Some(after) = tail.strip_prefix("$${") {
                // Escaped: `$${x}` is the literal text `${x}`
                out.push_str("${");
                rest = after;
            } else if tail.starts_with("${") {
                let close = reference_end(tail).ok_or_else(|| {
                    interpolation_error(
                        format!(
                            "Unclosed '${{' in string at line {}, column {}",
                            span.start_line, span.start_column
                        ),
                        tail,
                    )
                })?;
                let expression = tail[2..close].trim();
                let value = self.lookup(expression, span, ctx)?;
                append_text(&mut out, &value, expression, span)?;
                rest = &tail[close + 1..];
            } else {
                out.push('$');
                rest = &tail[1..];
            }
        }
        out.push_str(rest);
        Ok(out)
    }

    /// Resolve `${path}` to a value
    fn lookup(
        &mut self,
        expression: &str,
        span: &Span,
        ctx: &mut Option<(usize, &mut Run)>,
    ) -> Result<Value> {
        let names = parse_reference(expression).map_err(|message| {
            interpolation_error(
                format!(
                    "{message} in '${{{expression}}}' at line {}, column {}",
                    span.start_line, span.start_column
                ),
                expression,
            )
        })?;

        let mut remote = self.remote_depth > 0;
        if let Some((index, run)) = ctx {
            run.references += 1;
            if run.references > self.limits.max_references {
                return Err(NomlError::parse(
                    format!(
                        "Interpolation uses more than the limit of {} references",
                        self.limits.max_references
                    ),
                    span.start_line,
                    span.start_column,
                ));
            }
            let scopes = Rc::clone(&run.pending[*index].scopes);
            remote = scopes.remote;
            for scope in &scopes.paths {
                if let Some(path) = self.find(scope, &names, run)? {
                    if let Some(value) = tree::get(&run.root, &path) {
                        charge_output(
                            &mut run.output_bytes,
                            value,
                            self.limits.max_output_bytes,
                            span,
                        )?;
                        return Ok(value.clone());
                    }
                }
            }
        }

        // Fall back to variables registered with set_variable(), which a
        // document fetched over HTTP cannot see
        let variable = if remote {
            None
        } else {
            self.variable(expression, &names)
        };
        if let Some(value) = variable {
            if let Some((_, run)) = ctx {
                charge_output(
                    &mut run.output_bytes,
                    value,
                    self.limits.max_output_bytes,
                    span,
                )?;
            }
            return Ok(value.clone());
        }

        let mut error = interpolation_error(
            format!(
                "Undefined variable '{expression}' at line {}, column {}; paths are looked up from the document root",
                span.start_line, span.start_column
            ),
            expression,
        );
        if let (Some((index, run)), NomlError::Interpolation { context, .. }) = (ctx, &mut error) {
            *context = Some(tree::render_path(&run.pending[*index].location));
        }
        Err(error)
    }

    /// A variable registered with [`Resolver::set_variable`], by the whole
    /// expression or by its path below the first segment
    fn variable(&self, expression: &str, names: &[String]) -> Option<&Value> {
        if let Some(value) = self.variables.get(expression) {
            return Some(value);
        }
        if let Some(value) = self.variables.get(&names[0]) {
            let mut current = value;
            let mut found = true;
            for name in &names[1..] {
                let next = match current {
                    Value::Table(t) => t.get(name),
                    Value::Array(a) => name.parse::<usize>().ok().and_then(|i| a.get(i)),
                    _ => None,
                };
                match next {
                    Some(v) => current = v,
                    None => {
                        found = false;
                        break;
                    }
                }
            }
            if found {
                return Some(current);
            }
        }
        None
    }

    /// Find `names` below `scope`, settling any pending values on the way.
    /// Returns the physical path of the value, or `None` if it does not exist.
    fn find(&mut self, scope: &[Seg], names: &[String], run: &mut Run) -> Result<Option<Vec<Seg>>> {
        let mut path = scope.to_vec();
        for name in names {
            self.settle_at(&path, run)?;
            let next = match tree::get(&run.root, &path) {
                Some(Value::Table(t)) if t.contains_key(name) => Seg::Key(name.clone()),
                Some(Value::Array(a)) => match name.parse::<usize>() {
                    Ok(i) if i < a.len() => Seg::Index(i),
                    _ => return Ok(None),
                },
                _ => return Ok(None),
            };
            path.push(next);
        }

        // Settle the value itself and everything below it
        self.settle_at(&path, run)?;
        let below: Vec<usize> = run
            .by_location
            .range(path.clone()..)
            .take_while(|(location, _)| location.starts_with(&path))
            .map(|(_, &i)| i)
            .collect();
        for i in below {
            self.settle(i, run)?;
        }
        Ok(Some(path))
    }

    fn settle_at(&mut self, path: &[Seg], run: &mut Run) -> Result<()> {
        if let Some(&i) = run.by_location.get(path) {
            self.settle(i, run)?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Includes
    // ------------------------------------------------------------------

    fn resolve_include(
        &mut self,
        include_path: &str,
        span: &Span,
        location: &mut Vec<Seg>,
        scopes: &Scopes,
        run: &mut Run,
    ) -> Result<Value> {
        if self.include_stack.len() >= self.config.max_include_depth {
            return Err(NomlError::parse(
                format!(
                    "Maximum include depth ({}) exceeded",
                    self.config.max_include_depth
                ),
                span.start_line,
                span.start_column,
            ));
        }

        if self.remote_depth > 0 {
            return Err(NomlError::parse(
                "Includes are not allowed in a document included over HTTP",
                span.start_line,
                span.start_column,
            ));
        }
        self.include_count += 1;
        if self.include_count > self.limits.max_includes {
            return Err(NomlError::parse(
                format!(
                    "Maximum include count ({}) exceeded",
                    self.limits.max_includes
                ),
                span.start_line,
                span.start_column,
            ));
        }

        let remote = include_path.starts_with("http://") || include_path.starts_with("https://");
        let document = if remote {
            self.http_document(include_path, span)?
        } else {
            let resolved_path = self.resolve_include_path(include_path, span)?;
            if self.include_stack.contains(&resolved_path) {
                return Err(NomlError::circular_reference(format!(
                    "'{include_path}' includes itself (line {}, column {})",
                    span.start_line, span.start_column
                )));
            }
            let document = parse_file(&resolved_path)
                .map_err(|e| include_error("include", include_path, &e, span))?;
            self.include_stack.push(resolved_path);
            document
        };
        let pushed = !remote;

        // `${...}` in the included file looks in the included file first. A
        // remote document sees only itself.
        let mut paths: Vec<Vec<Seg>> = Vec::with_capacity(scopes.paths.len() + 1);
        paths.push(location.clone());
        if !remote {
            paths.extend(scopes.paths.iter().cloned());
        }
        let inner: Scopes = Rc::new(ScopeSet {
            paths,
            remote: remote || scopes.remote,
        });

        if remote {
            self.remote_depth += 1;
        }
        let result = self.build(&document.root, location, &inner, run);
        if remote {
            self.remote_depth -= 1;
        }
        if pushed {
            self.include_stack.pop();
        }
        match result? {
            Built::Value(value) => Ok(value),
            Built::Deferred => Err(NomlError::internal("include root was deferred")),
        }
    }

    #[cfg(feature = "async")]
    fn http_document(&mut self, url: &str, span: &Span) -> Result<Document> {
        if !url.starts_with("https://") {
            return Err(NomlError::parse(
                "HTTP includes must use https://",
                span.start_line,
                span.start_column,
            ));
        }
        match self.http_content.get(url) {
            Some(content) => crate::parser::parse(content)
                .map_err(|e| include_error("HTTP include", url, &e, span)),
            None => Err(NomlError::parse(
                "HTTP includes require async resolver. Use resolve_document_async() instead.",
                span.start_line,
                span.start_column,
            )),
        }
    }

    #[cfg(not(feature = "async"))]
    fn http_document(&mut self, _url: &str, span: &Span) -> Result<Document> {
        Err(NomlError::parse(
            "HTTP includes require the 'async' feature to be enabled",
            span.start_line,
            span.start_column,
        ))
    }

    /// Resolve an include path relative to the current file or base path,
    /// and check it against the include root. Returns the canonical path.
    fn resolve_include_path(&mut self, include_path: &str, span: &Span) -> Result<PathBuf> {
        let path = Path::new(include_path);

        let joined = if path.is_absolute() {
            path.to_path_buf()
        } else if let Some(current_file) = self.include_stack.last() {
            current_file.parent().unwrap_or(Path::new(".")).join(path)
        } else if let Some(base) = self
            .config
            .base_path
            .as_ref()
            .or(self.include_root.as_ref())
        {
            base.join(path)
        } else if self.includes_anywhere {
            path.to_path_buf()
        } else {
            return Err(no_include_root(span));
        };

        if self.includes_anywhere {
            // Canonical when possible, so that cycles are found however the
            // path is spelled; a missing file is reported when it is read
            return Ok(std::fs::canonicalize(&joined).unwrap_or(joined));
        }

        let root = self.canonical_root(span)?;
        let canonical = std::fs::canonicalize(&joined).map_err(|e| {
            NomlError::parse(
                format!("Failed to read include '{include_path}': {e}"),
                span.start_line,
                span.start_column,
            )
        })?;
        if !canonical.starts_with(&root) {
            return Err(NomlError::parse(
                "Include refused: the file is outside the include root",
                span.start_line,
                span.start_column,
            ));
        }
        Ok(canonical)
    }

    /// The canonical include root: the include root if set, else the base path
    fn canonical_root(&mut self, span: &Span) -> Result<PathBuf> {
        if let Some(root) = &self.canonical_root {
            return Ok(root.clone());
        }
        let root = self
            .include_root
            .as_ref()
            .or(self.config.base_path.as_ref())
            .ok_or_else(|| no_include_root(span))?;
        // An empty base path (a file name with no directory) is the current directory
        let root = if root.as_os_str().is_empty() {
            Path::new(".")
        } else {
            root.as_path()
        };
        let canonical = std::fs::canonicalize(root).map_err(|e| {
            NomlError::parse(
                format!("Failed to read the include root: {e}"),
                span.start_line,
                span.start_column,
            )
        })?;
        self.canonical_root = Some(canonical.clone());
        Ok(canonical)
    }

    // ------------------------------------------------------------------
    // Async HTTP includes
    // ------------------------------------------------------------------

    /// Resolve a document, fetching HTTP includes first.
    ///
    /// HTTP includes in the document itself are fetched; HTTP includes inside
    /// included files are not followed. Only `https://` URLs are fetched,
    /// redirects are not followed, and a body larger than
    /// [`ResolverLimits::max_http_body_bytes`] is an error. A fetched
    /// document cannot use `include` or `env()`, and its `${...}` references
    /// can only name values inside it.
    #[cfg(feature = "async")]
    pub async fn resolve_document_async(&mut self, document: &Document) -> Result<Value> {
        let mut urls = Vec::new();
        collect_http_includes(&document.root, &mut urls);
        if urls.len() > self.limits.max_includes {
            return Err(NomlError::parse(
                format!(
                    "Maximum include count ({}) exceeded",
                    self.limits.max_includes
                ),
                document.root.span.start_line,
                document.root.span.start_column,
            ));
        }
        urls.retain(|url| !self.http_content.contains_key(url));
        for url in urls {
            let content = self.fetch_http_content(&url).await?;
            self.http_content.insert(url, content);
        }
        self.resolve(document)
    }

    /// Fetch content from HTTP URL with caching
    #[cfg(feature = "async")]
    async fn fetch_http_content(&mut self, url: &str) -> Result<String> {
        if !url.starts_with("https://") {
            return Err(NomlError::import(url, "HTTP includes must use https://"));
        }
        if let Some(ref cache) = self.config.http_cache {
            if let Some(cached_content) = cache.get(url) {
                return Ok(cached_content.clone());
            }
        }

        let client = reqwest::Client::builder()
            .timeout(self.config.http_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true)
            .build()
            .map_err(|e| NomlError::import(url, format!("failed to create HTTP client: {e}")))?;

        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| NomlError::import(url, format!("request failed: {e}")))?;

        if !response.status().is_success() {
            return Err(NomlError::import(
                url,
                format!("server returned {}", response.status()),
            ));
        }

        let limit = self.limits.max_http_body_bytes;
        let too_large = || {
            NomlError::import(
                url,
                format!("the response is larger than the limit of {limit} bytes"),
            )
        };
        if response
            .content_length()
            .is_some_and(|length| length > limit as u64)
        {
            return Err(too_large());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| NomlError::import(url, format!("failed to read body: {e}")))?
        {
            if body.len() + chunk.len() > limit {
                return Err(too_large());
            }
            body.extend_from_slice(&chunk);
        }
        let content = String::from_utf8(body)
            .map_err(|_| NomlError::import(url, "the response is not valid UTF-8"))?;

        if let Some(ref mut cache) = self.config.http_cache {
            cache.insert(url.to_string(), content.clone());
        }

        Ok(content)
    }
}

/// Collect HTTP include URLs from an AST node
#[cfg(feature = "async")]
fn collect_http_includes(node: &AstNode, urls: &mut Vec<String>) {
    match &node.value {
        AstValue::Include { path }
            if (path.starts_with("http://") || path.starts_with("https://"))
                && !urls.contains(path) =>
        {
            urls.push(path.clone());
        }
        AstValue::Table { entries, .. } => {
            for entry in entries {
                collect_http_includes(&entry.value, urls);
            }
        }
        AstValue::Array { elements, .. } => {
            for element in elements {
                collect_http_includes(element, urls);
            }
        }
        _ => {}
    }
}

/// Maximum depth of the resolved value tree (keys and array indices).
///
/// Values nested deeper than this are rejected so that walking or dropping
/// the tree cannot overflow the stack, whatever the input.
const MAX_VALUE_DEPTH: usize = 128;

fn check_depth(depth: usize, span: &Span) -> Result<()> {
    if depth > MAX_VALUE_DEPTH {
        return Err(NomlError::parse(
            format!("Value is nested deeper than the limit of {MAX_VALUE_DEPTH} levels"),
            span.start_line,
            span.start_column,
        ));
    }
    Ok(())
}

/// Nesting depth of a value (0 for scalars)
fn value_depth(value: &Value) -> usize {
    match value {
        Value::Table(t) => 1 + t.values().map(value_depth).max().unwrap_or(0),
        Value::Array(a) => 1 + a.iter().map(value_depth).max().unwrap_or(0),
        _ => 0,
    }
}

/// True for a node whose value is computed: `env()`, `@type()`, `${...}`,
/// `include`, or a string with `${...}` in it
fn is_expression(node: &AstNode) -> bool {
    match &node.value {
        AstValue::FunctionCall { .. }
        | AstValue::Native { .. }
        | AstValue::Interpolation { .. }
        | AstValue::Include { .. } => true,
        AstValue::String { value, style, .. } => is_template(value, *style),
        _ => false,
    }
}

/// Look for `names` below `scope` the way [`Resolver::find`] does, without
/// settling anything. Waiting pending values in the way go to `out`.
fn probe(scope: &[Seg], names: &[String], run: &Run, out: &mut Vec<usize>) -> Probe {
    let blocked_at = |path: &[Seg], out: &mut Vec<usize>| -> bool {
        match run.by_location.get(path) {
            Some(&i) => match run.pending[i].state {
                State::Waiting => {
                    out.push(i);
                    true
                }
                // An active value here is a cycle, reported on evaluation
                State::Active => true,
                State::Done => false,
            },
            None => false,
        }
    };

    let mut path = scope.to_vec();
    for name in names {
        if blocked_at(&path, out) {
            return Probe::Blocked;
        }
        let next = match tree::get(&run.root, &path) {
            Some(Value::Table(t)) if t.contains_key(name) => Seg::Key(name.clone()),
            Some(Value::Array(a)) => match name.parse::<usize>() {
                Ok(i) if i < a.len() => Seg::Index(i),
                _ => return Probe::Missing,
            },
            _ => return Probe::Missing,
        };
        path.push(next);
    }

    let mut blocked = false;
    for (_, &i) in run
        .by_location
        .range(path.clone()..)
        .take_while(|(location, _)| location.starts_with(&path))
    {
        match run.pending[i].state {
            State::Waiting => {
                out.push(i);
                blocked = true;
            }
            State::Active => blocked = true,
            State::Done => {}
        }
    }
    if blocked {
        Probe::Blocked
    } else {
        Probe::Found
    }
}

/// Call `f` with each `${...}` expression in a template string, the same
/// way [`Resolver::render`] reads them
fn for_each_reference<'t>(text: &'t str, mut f: impl FnMut(&'t str)) {
    let mut rest = text;
    while let Some(pos) = rest.find('$') {
        let tail = &rest[pos..];
        if let Some(after) = tail.strip_prefix("$${") {
            rest = after;
        } else if tail.starts_with("${") {
            match tail.find('}') {
                Some(close) => {
                    f(tail[2..close].trim());
                    rest = &tail[close + 1..];
                }
                None => return,
            }
        } else {
            rest = &tail[1..];
        }
    }
}

/// Charge the copy of `value` made by a reference to the output budget
fn charge_output(used: &mut usize, value: &Value, limit: usize, span: &Span) -> Result<()> {
    let remaining = limit.saturating_sub(*used);
    let cost = value_cost(value, remaining);
    if cost > remaining {
        return Err(NomlError::parse(
            format!("Interpolation output exceeds the limit of {limit} bytes"),
            span.start_line,
            span.start_column,
        ));
    }
    *used += cost;
    Ok(())
}

/// Estimated memory of a copy of `value`: string and key bytes plus a fixed
/// cost per value. Stops counting once the total passes `budget`.
fn value_cost(value: &Value, budget: usize) -> usize {
    const NODE: usize = std::mem::size_of::<Value>();
    const KEY: usize = std::mem::size_of::<String>();
    let mut total = 0usize;
    let mut stack = vec![value];
    while let Some(value) = stack.pop() {
        let own = match value {
            Value::String(s) => s.len(),
            Value::Binary(b) => b.len(),
            Value::Array(items) => {
                stack.extend(items);
                0
            }
            Value::Table(table) => {
                let mut keys = 0usize;
                for (key, item) in table {
                    keys = keys.saturating_add(KEY + key.len());
                    stack.push(item);
                }
                keys
            }
            _ => 0,
        };
        total = total.saturating_add(NODE).saturating_add(own);
        if total > budget {
            return total;
        }
    }
    total
}

fn reference_depth_error(limit: usize, span: &Span) -> NomlError {
    NomlError::parse(
        format!("Interpolation references are chained deeper than the limit of {limit} levels"),
        span.start_line,
        span.start_column,
    )
}

fn no_include_root(span: &Span) -> NomlError {
    NomlError::parse(
        "Include refused: the document has no base path or include root (see Resolver::with_include_root)",
        span.start_line,
        span.start_column,
    )
}

/// The error for an included document that could not be read or parsed.
///
/// The included file may not be NOML at all (a credentials file, say), so
/// only the position and kind of the inner error are kept, never its text.
fn include_error(what: &str, path: &str, error: &NomlError, span: &Span) -> NomlError {
    let detail = match error {
        NomlError::Parse { line, column, .. } => {
            format!("invalid NOML at line {line}, column {column} of the included file")
        }
        NomlError::Io { source, .. } => source.to_string(),
        other => format!("{} error in the included file", other.category()),
    };
    NomlError::parse(
        format!("Failed to parse {what} '{path}': {detail}"),
        span.start_line,
        span.start_column,
    )
}

/// True if a string value needs interpolation (double-quoted and contains `${`)
fn is_template(value: &str, style: StringStyle) -> bool {
    matches!(style, StringStyle::Double | StringStyle::TripleDouble) && value.contains("${")
}

/// True if a node or anything inside it needs interpolation
fn has_template(node: &AstNode) -> bool {
    match &node.value {
        AstValue::String { value, style, .. } => is_template(value, *style),
        AstValue::Interpolation { .. } => true,
        AstValue::Array { elements, .. } => elements.iter().any(has_template),
        AstValue::Table { entries, .. } => entries.iter().any(|e| has_template(&e.value)),
        AstValue::FunctionCall { args, .. } | AstValue::Native { args, .. } => {
            args.iter().any(has_template)
        }
        _ => false,
    }
}

/// Byte position of the `}` that closes the `${` at the start of `tail`,
/// skipping braces inside quoted path segments.
fn reference_end(tail: &str) -> Option<usize> {
    let mut in_quote = false;
    let mut escaped = false;
    for (i, c) in tail.char_indices().skip(2) {
        if in_quote {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_quote = false,
                _ => {}
            }
        } else if c == '"' {
            in_quote = true;
        } else if c == '}' {
            return Some(i);
        }
    }
    None
}

/// Split a reference like `a.b`, `servers.0.name`, `servers[0].name` or
/// `"dotted.key".x` into its segments.
fn parse_reference(expression: &str) -> std::result::Result<Vec<String>, &'static str> {
    let mut names = Vec::new();
    let mut rest = expression.trim();
    if rest.is_empty() {
        return Err("Empty reference");
    }
    loop {
        rest = rest.trim_start();
        let (name, after) = if let Some(quoted) = rest.strip_prefix('"') {
            // Quoted segment; `\"` and `\\` are escapes
            let mut name = String::new();
            let mut chars = quoted.char_indices();
            let mut end = None;
            while let Some((i, c)) = chars.next() {
                match c {
                    '"' => {
                        end = Some(i);
                        break;
                    }
                    '\\' => match chars.next() {
                        Some((_, escaped)) => name.push(escaped),
                        None => return Err("Unclosed quote"),
                    },
                    c => name.push(c),
                }
            }
            let end = end.ok_or("Unclosed quote")?;
            (name, &quoted[end + 1..])
        } else {
            let end = rest.find(['.', '[']).unwrap_or(rest.len());
            let name = rest[..end].trim();
            if name.is_empty() {
                return Err("Empty path segment");
            }
            (name.to_string(), &rest[end..])
        };
        names.push(name);
        rest = after.trim_start();

        // Any number of [index] suffixes
        while let Some(inner) = rest.strip_prefix('[') {
            let end = inner.find(']').ok_or("Unclosed '['")?;
            let index = inner[..end].trim();
            if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
                return Err("Array index must be a number");
            }
            names.push(index.to_string());
            rest = inner[end + 1..].trim_start();
        }

        if rest.is_empty() {
            return Ok(names);
        }
        rest = rest
            .strip_prefix('.')
            .ok_or("Expected '.' between path segments")?;
    }
}

/// Append an interpolated value to a string
fn append_text(out: &mut String, value: &Value, expression: &str, span: &Span) -> Result<()> {
    use std::fmt::Write;
    match value {
        Value::String(s) => out.push_str(s),
        Value::Integer(i) => {
            let _ = write!(out, "{i}");
        }
        Value::Float(f) => {
            let _ = write!(out, "{f}");
            if f.is_finite() && f.fract() == 0.0 && !out.ends_with('e') {
                out.push_str(".0");
            }
        }
        Value::Bool(b) => {
            let _ = write!(out, "{b}");
        }
        Value::Size(bytes) => {
            let _ = write!(out, "{bytes}");
        }
        Value::Duration(seconds) => {
            let _ = write!(out, "{seconds}");
        }
        Value::DateTime(dt) => {
            let _ = write!(out, "{dt}");
        }
        other => {
            return Err(interpolation_error(
                format!(
                    "Cannot insert {} '{expression}' into a string at line {}, column {}; use a bare ${{{expression}}} value to copy it",
                    article(other.type_name()),
                    span.start_line,
                    span.start_column
                ),
                expression,
            ))
        }
    }
    Ok(())
}

fn article(type_name: &str) -> String {
    match type_name.chars().next() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => format!("an {type_name}"),
        _ => format!("a {type_name}"),
    }
}

fn interpolation_error(message: String, expression: &str) -> NomlError {
    NomlError::Interpolation {
        message,
        expression: expression.to_string(),
        context: None,
    }
}

fn cycle_error(run: &Run, index: usize) -> NomlError {
    let start = run
        .active
        .iter()
        .position(|&i| i == index)
        .unwrap_or_default();
    let mut chain: Vec<String> = run.active[start..]
        .iter()
        .map(|&i| {
            let p = &run.pending[i];
            format!(
                "{} (line {})",
                tree::render_path(&p.location),
                p.node.span.start_line
            )
        })
        .collect();
    chain.push(tree::render_path(&run.pending[index].location));
    NomlError::circular_reference(chain.join(" -> "))
}

/// Give a location-less parse error from a native resolver the position of the call
fn locate(error: NomlError, span: &Span) -> NomlError {
    match error {
        NomlError::Parse {
            message,
            line: 0,
            column: 0,
            snippet,
        } => NomlError::Parse {
            message,
            line: span.start_line,
            column: span.start_column,
            snippet,
        },
        other => other,
    }
}

// Built-in native type resolvers

fn single_string_arg<'v>(name: &str, args: &'v [Value]) -> Result<&'v str> {
    if args.len() != 1 {
        return Err(NomlError::parse(
            format!("@{name}() requires exactly 1 argument"),
            0,
            0,
        ));
    }
    match &args[0] {
        Value::String(s) => Ok(s),
        other => Err(NomlError::parse(
            format!(
                "@{name}() argument must be a string, found {}",
                other.type_name()
            ),
            0,
            0,
        )),
    }
}

fn resolve_size(args: &[Value]) -> Result<Value> {
    let size_str = single_string_arg("size", args)?;
    match parse_size(size_str) {
        // parse_size never returns a negative value
        Some(n) => Ok(Value::Size(n.unsigned_abs())),
        None => Err(NomlError::parse(
            "Invalid size (expected a non-negative number with an optional unit such as KB, MB or GiB, up to 8 EiB)",
            0,
            0,
        )),
    }
}

/// Parse size strings like "10MB", "1.5GB", "512 KiB" into bytes (1 KB = 1024 bytes).
///
/// The arithmetic is done on integers, so every size up to `i64::MAX` is
/// exact. A fraction of a byte is dropped (`0.1KB` is 102 bytes).
pub(crate) fn parse_size(size_str: &str) -> Option<i64> {
    let s = size_str.trim();
    let pos = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '_'))
        .unwrap_or(s.len());
    let (number, unit) = s.split_at(pos);
    let number = number.replace('_', "");
    let (whole, fraction) = number.split_once('.').unwrap_or((&number, ""));
    if fraction.contains('.') || (whole.is_empty() && fraction.is_empty()) {
        return None;
    }
    let multiplier: u128 = match unit.trim().to_lowercase().as_str() {
        "" | "b" | "byte" | "bytes" => 1,
        "k" | "kb" | "kib" => 1 << 10,
        "m" | "mb" | "mib" => 1 << 20,
        "g" | "gb" | "gib" => 1 << 30,
        "t" | "tb" | "tib" => 1 << 40,
        "p" | "pb" | "pib" => 1 << 50,
        "e" | "eb" | "eib" => 1 << 60,
        _ => return None,
    };
    let whole: u128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let mut bytes = whole.checked_mul(multiplier)?;
    // 19 fraction digits times the largest multiplier still fit in a u128;
    // later digits are dropped
    let fraction = &fraction[..fraction.len().min(19)];
    if !fraction.is_empty() {
        let digits: u128 = fraction.parse().ok()?;
        let scale = 10u128.pow(fraction.len() as u32);
        bytes = bytes.checked_add(digits * multiplier / scale)?;
    }
    i64::try_from(bytes).ok()
}

fn resolve_duration(args: &[Value]) -> Result<Value> {
    let duration_str = single_string_arg("duration", args)?;
    match parse_duration(duration_str) {
        Some(n) => Ok(Value::Duration(n)),
        None => Err(NomlError::parse(
            "Invalid duration (expected a non-negative number with a unit such as ms, s, m, h or d)",
            0,
            0,
        )),
    }
}

/// Seconds per duration unit
fn duration_unit(unit: &str) -> Option<f64> {
    Some(match unit {
        "ns" | "nanosecond" | "nanoseconds" => 1e-9,
        "us" | "µs" | "μs" | "microsecond" | "microseconds" => 1e-6,
        "ms" | "millisecond" | "milliseconds" => 1e-3,
        "" | "s" | "sec" | "secs" | "second" | "seconds" => 1.0,
        "m" | "min" | "mins" | "minute" | "minutes" => 60.0,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3600.0,
        "d" | "day" | "days" => 86400.0,
        "w" | "week" | "weeks" => 604800.0,
        _ => return None,
    })
}

/// Parse duration strings like "30s", "5m", "250ms" or "1h30m" into seconds.
///
/// A bare number is seconds. Compound forms add their parts: "1h30m" is 5400.
pub(crate) fn parse_duration(duration_str: &str) -> Option<f64> {
    let text = duration_str.trim().to_lowercase();
    if text.is_empty() {
        return None;
    }
    let mut rest = text.as_str();
    let mut total = 0.0;
    let mut parts = 0;
    while !rest.is_empty() {
        let number_end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '_'))
            .unwrap_or(rest.len());
        let (number, after) = rest.split_at(number_end);
        if number.is_empty() {
            return None;
        }
        let number: f64 = number.replace('_', "").parse().ok()?;
        let after = after.trim_start();
        let unit_end = after
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(after.len());
        let (unit, next) = after.split_at(unit_end);
        let unit = unit.trim();
        // A bare number is only allowed on its own ("15" is 15 seconds)
        if unit.is_empty() && (parts > 0 || !next.is_empty()) {
            return None;
        }
        total += number * duration_unit(unit)?;
        parts += 1;
        rest = next;
    }
    (parts > 0 && total.is_finite()).then_some(total)
}

/// `@regex(...)`: the pattern is passed through as a string. NOML does not
/// compile it; validate it in your application with the regex engine you use.
fn resolve_regex(args: &[Value]) -> Result<Value> {
    let regex_str = single_string_arg("regex", args)?;
    Ok(Value::String(regex_str.to_string()))
}

fn resolve_url(args: &[Value]) -> Result<Value> {
    let url_str = single_string_arg("url", args)?;
    let valid = ["http://", "https://"]
        .iter()
        .any(|scheme| url_str.len() > scheme.len() && url_str.starts_with(scheme))
        && !url_str.chars().any(char::is_whitespace);
    if valid {
        Ok(Value::String(url_str.to_string()))
    } else {
        Err(NomlError::parse(
            "Invalid URL (expected an http:// or https:// URL)",
            0,
            0,
        ))
    }
}

/// `@ip(...)`: an IPv4 or IPv6 address, optionally in CIDR form (`10.0.0.0/8`)
fn resolve_ip(args: &[Value]) -> Result<Value> {
    let ip_str = single_string_arg("ip", args)?;
    let (address, prefix) = match ip_str.split_once('/') {
        Some((address, prefix)) => (address, Some(prefix)),
        None => (ip_str, None),
    };
    let valid = match address.parse::<std::net::IpAddr>() {
        Ok(ip) => prefix.is_none_or(|p| {
            let max = if ip.is_ipv4() { 32 } else { 128 };
            !p.is_empty()
                && p.bytes().all(|b| b.is_ascii_digit())
                && p.parse::<u8>().is_ok_and(|n| n <= max)
        }),
        Err(_) => false,
    };
    if valid {
        Ok(Value::String(ip_str.to_string()))
    } else {
        Err(NomlError::parse(
            "Invalid IP address (expected IPv4 or IPv6, optionally with a /prefix)",
            0,
            0,
        ))
    }
}

fn resolve_semver(args: &[Value]) -> Result<Value> {
    let version_str = single_string_arg("semver", args)?;

    // MAJOR.MINOR[.PATCH][-PRERELEASE][+BUILD]
    let (core, build) = match version_str.split_once('+') {
        Some((core, build)) => (core, Some(build)),
        None => (version_str, None),
    };
    let (core, pre) = match core.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (core, None),
    };
    let ident_ok = |s: &str| {
        !s.is_empty()
            && s.split('.').all(|part| {
                !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
    };
    let parts: Vec<&str> = core.split('.').collect();
    let valid = (2..=3).contains(&parts.len())
        && parts.iter().all(|p| {
            !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u64>().is_ok()
        })
        && pre.is_none_or(ident_ok)
        && build.is_none_or(ident_ok);

    if valid {
        Ok(Value::String(version_str.to_string()))
    } else {
        Err(NomlError::parse(
            "Invalid semantic version (expected MAJOR.MINOR[.PATCH][-PRE][+BUILD])",
            0,
            0,
        ))
    }
}

fn resolve_base64(args: &[Value]) -> Result<Value> {
    let base64_str = single_string_arg("base64", args)?;

    let body = base64_str.trim_end_matches('=');
    let padding = base64_str.len() - body.len();
    let valid = base64_str.len() % 4 == 0
        && padding <= 2
        && body
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/');

    if valid {
        Ok(Value::String(base64_str.to_string()))
    } else {
        Err(NomlError::parse(
            "Invalid base64 (expected standard base64 with = padding)",
            0,
            0,
        ))
    }
}

fn resolve_uuid(args: &[Value]) -> Result<Value> {
    let uuid_str = single_string_arg("uuid", args)?;

    // Format: 8-4-4-4-12 hex digits
    let parts: Vec<&str> = uuid_str.split('-').collect();
    let lengths = [8, 4, 4, 4, 12];
    if parts.len() == 5
        && parts
            .iter()
            .zip(lengths)
            .all(|(part, len)| part.len() == len && part.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        Ok(Value::String(uuid_str.to_string()))
    } else {
        Err(NomlError::parse(
            "Invalid UUID (expected 8-4-4-4-12 hex digits)",
            0,
            0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1KB"), Some(1024));
        assert_eq!(parse_size("1MB"), Some(1024 * 1024));
        assert_eq!(parse_size("512 KiB"), Some(512 * 1024));
        assert_eq!(parse_size("10mb"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("42"), Some(42));
        assert_eq!(
            parse_size("1.5GB"),
            Some((1.5 * 1024.0 * 1024.0 * 1024.0) as i64)
        );
        assert_eq!(parse_size("invalid"), None);
        assert_eq!(parse_size("-5MB"), None);
        assert_eq!(parse_size("MB"), None);
        assert_eq!(parse_size("8EB"), None, "2^63 bytes does not fit in i64");
        assert_eq!(parse_size("99999999999PB"), None);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s"), Some(30.0));
        assert_eq!(parse_duration("5m"), Some(300.0));
        assert_eq!(parse_duration("2h"), Some(7200.0));
        assert_eq!(parse_duration("1d"), Some(86400.0));
        assert_eq!(parse_duration("250ms"), Some(0.25));
        assert_eq!(parse_duration("10 seconds"), Some(10.0));
        assert_eq!(parse_duration("15"), Some(15.0));
        assert_eq!(parse_duration("1h30m"), Some(5400.0));
        assert_eq!(parse_duration("1d 12h"), Some(129600.0));
        assert_eq!(parse_duration("2m30"), None, "ambiguous: 30 what?");
        assert_eq!(parse_duration("30 1h"), None);
        assert_eq!(parse_duration("h"), None);
        assert_eq!(parse_duration("invalid"), None);
        assert_eq!(parse_duration("-5s"), None);
    }

    #[test]
    fn test_resolve_size_duration_url() {
        let size_result = resolve_size(&[Value::String("10MB".to_string())]).unwrap();
        assert_eq!(size_result.as_integer().unwrap(), 10 * 1024 * 1024);

        let duration_result = resolve_duration(&[Value::String("30s".to_string())]).unwrap();
        let duration_val = duration_result.as_float().unwrap();
        assert!(
            (duration_val - 30.0).abs() < f64::EPSILON,
            "Expected 30.0, got {duration_val}"
        );

        let url_result = resolve_url(&[Value::String("https://example.com".to_string())]).unwrap();
        assert_eq!(url_result.as_string().unwrap(), "https://example.com");
        assert!(resolve_url(&[Value::String("https://".to_string())]).is_err());
        assert!(resolve_url(&[Value::Integer(1)]).is_err());
    }

    #[test]
    fn test_semver_base64_uuid() {
        let s = |v: &str| [Value::String(v.to_string())];
        assert!(resolve_semver(&s("1.2.3")).is_ok());
        assert!(resolve_semver(&s("1.2.3-beta.1+build.5")).is_ok());
        assert!(resolve_semver(&s("1.2")).is_ok());
        assert!(resolve_semver(&s("1")).is_err());
        assert!(resolve_semver(&s("1.2.x")).is_err());
        assert!(resolve_semver(&s("1.2.3-")).is_err());

        assert!(resolve_base64(&s("aGVsbG8=")).is_ok());
        assert!(resolve_base64(&s("a=bc")).is_err());
        assert!(resolve_base64(&s("abc")).is_err());

        assert!(resolve_uuid(&s("123e4567-e89b-12d3-a456-426614174000")).is_ok());
        assert!(resolve_uuid(&s("123e4567-e89b-12d3-a456-42661417400g")).is_err());

        assert!(resolve_ip(&s("10.0.0.0/8")).is_ok());
        assert!(resolve_ip(&s("::1")).is_ok());
        assert!(resolve_ip(&s("fd00::/64")).is_ok());
        assert!(resolve_ip(&s("10.0.0.0/33")).is_err());
        assert!(resolve_ip(&s("10.0.0.0/")).is_err());
        assert!(resolve_ip(&s("300.0.0.1")).is_err());
    }

    #[test]
    fn test_parse_reference() {
        assert_eq!(parse_reference("a").unwrap(), ["a"]);
        assert_eq!(parse_reference(" a.b ").unwrap(), ["a", "b"]);
        assert_eq!(parse_reference("s[0].name").unwrap(), ["s", "0", "name"]);
        assert_eq!(parse_reference("s.0.name").unwrap(), ["s", "0", "name"]);
        assert_eq!(parse_reference("\"a.b\".c").unwrap(), ["a.b", "c"]);
        assert_eq!(parse_reference(r#""a\"b".c"#).unwrap(), ["a\"b", "c"]);
        assert_eq!(reference_end(r#"${"x}y".z} rest"#), Some(9));
        assert!(parse_reference("").is_err());
        assert!(parse_reference("a..b").is_err());
        assert!(parse_reference("a[x]").is_err());
        assert!(parse_reference("a.").is_err());
    }

    #[test]
    fn native_resolver_config_clones_custom_resolvers() {
        let config = ResolverConfig::default();
        let mut config = config.clone();
        config.native_resolvers.insert(
            "upper".to_string(),
            NativeResolver::new(|args| Ok(Value::String(args[0].to_string().to_uppercase()))),
        );
        let cloned = config.clone();
        assert!(cloned.native_resolvers.contains_key("upper"));
        assert!(cloned.native_resolvers.contains_key("size"));
    }

    #[test]
    fn size_arithmetic_is_exact() {
        assert_eq!(parse_size("9007199254740993"), Some(9_007_199_254_740_993));
        assert_eq!(parse_size("9223372036854775807"), Some(i64::MAX));
        assert_eq!(parse_size("9223372036854775808"), None);
        assert_eq!(parse_size("7.99999EB"), Some(9_223_360_507_639_729_739));
        assert_eq!(parse_size("0.1KB"), Some(102));
        assert_eq!(parse_size(".5KB"), Some(512));
        assert_eq!(parse_size("1.KB"), Some(1024));
        assert_eq!(parse_size("1_024 B"), Some(1024));
        assert_eq!(parse_size("."), None);
        assert_eq!(parse_size("1.2.3"), None);
    }

    #[cfg(feature = "async")]
    #[test]
    fn remote_documents_are_confined() {
        let url = "https://config.example.com/defaults.noml";
        let source = format!("local = \"secret\"\nremote = include \"{url}\"");
        let resolve = |body: &str| {
            let mut resolver = Resolver::new().allow_includes_anywhere(true);
            resolver
                .http_content
                .insert(url.to_string(), body.to_string());
            let document = crate::parser::parse(&source).unwrap();
            resolver.resolve(&document)
        };

        let error = resolve("x = include \"Cargo.toml\"")
            .unwrap_err()
            .to_string();
        assert!(error.contains("not allowed"), "{error}");
        let error = resolve("x = env(\"PATH\")").unwrap_err().to_string();
        assert!(error.contains("env()"), "{error}");
        let error = resolve("x = env(\"PATH\", \"${a}\")\na = 1")
            .unwrap_err()
            .to_string();
        assert!(error.contains("env()"), "{error}");
        // References only see the remote document
        assert!(resolve("x = \"${local}\"").is_err());
        let value = resolve("a = 1\nb = \"${a}\"").unwrap();
        assert_eq!(value.get("remote.b").unwrap().as_string().unwrap(), "1");

        let document = crate::parser::parse("r = include \"http://example.com/x.noml\"").unwrap();
        let error = Resolver::new().resolve(&document).unwrap_err().to_string();
        assert!(error.contains("https://"), "{error}");
    }
}
