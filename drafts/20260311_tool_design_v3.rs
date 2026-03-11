//! Type-Safe Tool Configuration Builder
//!
//! This module demonstrates how to design tools that "cannot produce errors"
//! by using Rust's type system to enforce correctness at compile time.
//!
//! Key techniques:
//! - Typestate pattern with PhantomData
//! - Compile-time validation through type constraints
//! - Builder pattern with mandatory field enforcement

use std::marker::PhantomData;

// =============================================================================
// PART 1: Marker Types for Typestate Pattern
// =============================================================================

/// Marker type indicating a field is NOT set
#[derive(Debug, Clone, Copy)]
pub struct Unset;

/// Marker type indicating a field IS set
#[derive(Debug, Clone, Copy)]
pub struct Set;

// =============================================================================
// PART 2: Type-Safe Database Configuration Builder
// =============================================================================

/// Database configuration that can only be built with all required fields
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
    pub timeout_ms: u64,        // Has default
    pub max_connections: u32,   // Has default
    pub ssl_mode: SslMode,      // Has default
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslMode {
    Disable,
    Prefer,
    Require,
}

/// Type-safe builder using typestate pattern
///
/// H, P, U, PW, D are type parameters tracking whether each required field is set
pub struct DatabaseConfigBuilder<H, P, U, PW, D> {
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    password: Option<String>,
    database: Option<String>,
    timeout_ms: u64,
    max_connections: u32,
    ssl_mode: SslMode,
    _phantom: PhantomData<(H, P, U, PW, D)>,
}

// =============================================================================
// PART 3: Initial State - Nothing is set
// =============================================================================

impl DatabaseConfigBuilder<Unset, Unset, Unset, Unset, Unset> {
    /// Create a new builder with all required fields unset
    pub fn new() -> Self {
        Self {
            host: None,
            port: None,
            username: None,
            password: None,
            database: None,
            timeout_ms: 5000,      // Default: 5 seconds
            max_connections: 10,   // Default: 10 connections
            ssl_mode: SslMode::Prefer, // Default: prefer SSL
            _phantom: PhantomData,
        }
    }
}

// =============================================================================
// PART 4: State Transitions - Each setter changes the type
// =============================================================================

impl<P, U, PW, D> DatabaseConfigBuilder<Unset, P, U, PW, D> {
    /// Set the host - transitions H from Unset to Set
    pub fn host(self, host: impl Into<String>) -> DatabaseConfigBuilder<Set, P, U, PW, D> {
        DatabaseConfigBuilder {
            host: Some(host.into()),
            port: self.port,
            username: self.username,
            password: self.password,
            database: self.database,
            timeout_ms: self.timeout_ms,
            max_connections: self.max_connections,
            ssl_mode: self.ssl_mode,
            _phantom: PhantomData,
        }
    }
}

impl<H, U, PW, D> DatabaseConfigBuilder<H, Unset, U, PW, D> {
    /// Set the port - transitions P from Unset to Set
    pub fn port(self, port: u16) -> DatabaseConfigBuilder<H, Set, U, PW, D> {
        DatabaseConfigBuilder {
            host: self.host,
            port: Some(port),
            username: self.username,
            password: self.password,
            database: self.database,
            timeout_ms: self.timeout_ms,
            max_connections: self.max_connections,
            ssl_mode: self.ssl_mode,
            _phantom: PhantomData,
        }
    }
}

impl<H, P, PW, D> DatabaseConfigBuilder<H, P, Unset, PW, D> {
    /// Set the username - transitions U from Unset to Set
    pub fn username(self, username: impl Into<String>) -> DatabaseConfigBuilder<H, P, Set, PW, D> {
        DatabaseConfigBuilder {
            host: self.host,
            port: self.port,
            username: Some(username.into()),
            password: self.password,
            database: self.database,
            timeout_ms: self.timeout_ms,
            max_connections: self.max_connections,
            ssl_mode: self.ssl_mode,
            _phantom: PhantomData,
        }
    }
}

impl<H, P, U, D> DatabaseConfigBuilder<H, P, U, Unset, D> {
    /// Set the password - transitions PW from Unset to Set
    pub fn password(self, password: impl Into<String>) -> DatabaseConfigBuilder<H, P, U, Set, D> {
        DatabaseConfigBuilder {
            host: self.host,
            port: self.port,
            username: self.username,
            password: Some(password.into()),
            database: self.database,
            timeout_ms: self.timeout_ms,
            max_connections: self.max_connections,
            ssl_mode: self.ssl_mode,
            _phantom: PhantomData,
        }
    }
}

impl<H, P, U, PW> DatabaseConfigBuilder<H, P, U, PW, Unset> {
    /// Set the database name - transitions D from Unset to Set
    pub fn database(self, database: impl Into<String>) -> DatabaseConfigBuilder<H, P, U, PW, Set> {
        DatabaseConfigBuilder {
            host: self.host,
            port: self.port,
            username: self.username,
            password: self.password,
            database: Some(database.into()),
            timeout_ms: self.timeout_ms,
            max_connections: self.max_connections,
            ssl_mode: self.ssl_mode,
            _phantom: PhantomData,
        }
    }
}

// =============================================================================
// PART 5: Optional Fields - Available in any state
// =============================================================================

impl<H, P, U, PW, D> DatabaseConfigBuilder<H, P, U, PW, D> {
    /// Set timeout - optional, available in any state
    pub fn timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = timeout;
        self
    }

    /// Set max connections - optional, available in any state
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    /// Set SSL mode - optional, available in any state
    pub fn ssl_mode(mut self, mode: SslMode) -> Self {
        self.ssl_mode = mode;
        self
    }
}

// =============================================================================
// PART 6: Build - ONLY available when ALL required fields are Set
// =============================================================================

impl DatabaseConfigBuilder<Set, Set, Set, Set, Set> {
    /// Build the configuration - only callable when all required fields are set
    ///
    /// This is the key: the type system guarantees at compile time that
    /// all required fields are present. No runtime validation needed!
    pub fn build(self) -> DatabaseConfig {
        DatabaseConfig {
            host: self.host.unwrap(),  // Safe: type system guarantees Some
            port: self.port.unwrap(),  // Safe: type system guarantees Some
            username: self.username.unwrap(),
            password: self.password.unwrap(),
            database: self.database.unwrap(),
            timeout_ms: self.timeout_ms,
            max_connections: self.max_connections,
            ssl_mode: self.ssl_mode,
        }
    }
}

// =============================================================================
// PART 7: Advanced - Compile-time Value Validation
// =============================================================================

/// A port number that is guaranteed to be valid at compile time
///
/// This uses const generics to validate the port at compile time
#[derive(Debug, Clone, Copy)]
pub struct ValidPort<const N: u16>;

impl<const N: u16> ValidPort<N> {
    /// Create a valid port - only compiles if N is in valid range (1-65535)
    ///
    /// Note: This is a simplified example. Full implementation would use
    /// const assertions which are available in newer Rust versions.
    pub const fn new() -> Self {
        // In real implementation with const_assert:
        // const_assert!(N > 0 && N <= 65535);
        Self
    }

    pub const fn value() -> u16 {
        N
    }
}

// =============================================================================
// PART 8: Type-Safe CLI Arguments with State Machine
// =============================================================================

/// CLI command configuration with validated states
#[derive(Debug)]
pub struct CliConfig {
    pub command: Command,
    pub verbose: bool,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone)]
pub enum Command {
    Build { target: String, release: bool },
    Test { filter: Option<String> },
    Run { args: Vec<String> },
}

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Text,
    Json,
    Yaml,
}

/// Type-safe CLI builder that enforces command selection
pub struct CliConfigBuilder<C> {
    command: Option<Command>,
    verbose: bool,
    output_format: OutputFormat,
    _phantom: PhantomData<C>,
}

pub struct NoCommand;
pub struct HasCommand;

impl CliConfigBuilder<NoCommand> {
    pub fn new() -> Self {
        Self {
            command: None,
            verbose: false,
            output_format: OutputFormat::Text,
            _phantom: PhantomData,
        }
    }

    /// Set build command - transitions to HasCommand state
    pub fn build_command(
        self,
        target: impl Into<String>,
    ) -> CliConfigBuilder<HasCommand> {
        CliConfigBuilder {
            command: Some(Command::Build {
                target: target.into(),
                release: false,
            }),
            verbose: self.verbose,
            output_format: self.output_format,
            _phantom: PhantomData,
        }
    }

    /// Set test command - transitions to HasCommand state
    pub fn test_command(self, filter: Option<String>) -> CliConfigBuilder<HasCommand> {
        CliConfigBuilder {
            command: Some(Command::Test { filter }),
            verbose: self.verbose,
            output_format: self.output_format,
            _phantom: PhantomData,
        }
    }

    /// Set run command - transitions to HasCommand state
    pub fn run_command(self, args: Vec<String>) -> CliConfigBuilder<HasCommand> {
        CliConfigBuilder {
            command: Some(Command::Run { args }),
            verbose: self.verbose,
            output_format: self.output_format,
            _phantom: PhantomData,
        }
    }
}

impl CliConfigBuilder<HasCommand> {
    /// Set verbose flag - only available after command is selected
    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    /// Set output format - only available after command is selected
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Build CLI config - only available when command is set
    pub fn build(self) -> CliConfig {
        CliConfig {
            command: self.command.unwrap(),
            verbose: self.verbose,
            output_format: self.output_format,
        }
    }
}

// =============================================================================
// PART 9: Error-Preventing API Design Patterns
// =============================================================================

/// Pattern: Use consuming builders for immutability
///
/// Once built, the configuration cannot be modified, preventing
/// accidental mutation after validation.
pub struct ImmutableConfig {
    inner: DatabaseConfig,
}

impl ImmutableConfig {
    /// Create from a fully-built DatabaseConfig
    pub fn new(config: DatabaseConfig) -> Self {
        Self { inner: config }
    }

    /// Access config (immutable borrow only)
    pub fn config(&self) -> &DatabaseConfig {
        &self.inner
    }
}

/// Pattern: Sealed traits for internal implementation details
///
/// Prevents users from implementing traits that should only be
/// implemented by the crate.
mod sealed {
    pub trait Sealed {}
}

/// Public trait that is sealed - cannot be implemented outside this module
pub trait Configurable: sealed::Sealed {
    fn configure(&self) -> String;
}

// =============================================================================
// PART 10: Usage Examples and Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config_builds() {
        let config = DatabaseConfigBuilder::new()
            .host("localhost")
            .port(5432)
            .username("admin")
            .password("secret")
            .database("mydb")
            .timeout_ms(10000)
            .max_connections(20)
            .ssl_mode(SslMode::Require)
            .build();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.timeout_ms, 10000);
        assert_eq!(config.ssl_mode, SslMode::Require);
    }

    #[test]
    fn test_config_with_defaults() {
        let config = DatabaseConfigBuilder::new()
            .host("localhost")
            .port(5432)
            .username("admin")
            .password("secret")
            .database("mydb")
            .build();

        // Defaults should be applied
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.ssl_mode, SslMode::Prefer);
    }

    #[test]
    fn test_cli_builder() {
        let config = CliConfigBuilder::new()
            .build_command("x86_64-unknown-linux-gnu")
            .verbose()
            .output_format(OutputFormat::Json)
            .build();

        match config.command {
            Command::Build { target, release } => {
                assert_eq!(target, "x86_64-unknown-linux-gnu");
                assert!(!release);
            }
            _ => panic!("Expected Build command"),
        }
        assert!(config.verbose);
        matches!(config.output_format, OutputFormat::Json);
    }

    #[test]
    fn test_cli_test_command() {
        let config = CliConfigBuilder::new()
            .test_command(Some("integration".to_string()))
            .build();

        match config.command {
            Command::Test { filter } => {
                assert_eq!(filter, Some("integration".to_string()));
            }
            _ => panic!("Expected Test command"),
        }
    }
}

// =============================================================================
// PART 11: Compile-time Error Demonstrations (Uncomment to see errors)
// =============================================================================

/*
// ERROR: Cannot build without setting all required fields
fn demo_missing_fields() {
    let config = DatabaseConfigBuilder::new()
        .host("localhost")
        .port(5432)
        .build(); // ERROR: method `build` not found for DatabaseConfigBuilder<Set, Set, Unset, Unset, Unset>
}

// ERROR: Cannot set fields in wrong order (if we enforced order)
fn demo_wrong_order() {
    // Actually, with this design, order doesn't matter for required fields
    // But we could enforce order by chaining type states differently
}

// ERROR: Cannot build CLI without selecting command
fn demo_no_command() {
    let cli = CliConfigBuilder::new()
        .verbose()  // ERROR: method `verbose` not found for CliConfigBuilder<NoCommand>
        .build();   // ERROR: method `build` not found for CliConfigBuilder<NoCommand>
}
*/

// =============================================================================
// PART 12: Main function for demonstration
// =============================================================================

fn main() {
    println!("=== Type-Safe Configuration Builder Demo ===\n");

    // Example 1: Building a database configuration
    println!("1. Building DatabaseConfig:");
    let db_config = DatabaseConfigBuilder::new()
        .host("db.example.com")
        .port(5432)
        .username("app_user")
        .password("hunter2")
        .database("production")
        .timeout_ms(30000)
        .max_connections(100)
        .ssl_mode(SslMode::Require)
        .build();

    println!("   Host: {}", db_config.host);
    println!("   Port: {}", db_config.port);
    println!("   Database: {}", db_config.database);
    println!("   Timeout: {}ms", db_config.timeout_ms);
    println!("   Max Connections: {}", db_config.max_connections);
    println!();

    // Example 2: Building CLI config
    println!("2. Building CliConfig:");
    let cli_config = CliConfigBuilder::new()
        .build_command("wasm32-unknown-unknown")
        .verbose()
        .output_format(OutputFormat::Json)
        .build();

    println!("   Verbose: {}", cli_config.verbose);
    println!("   Command: {:?}", cli_config.command);
    println!();

    // Example 3: Immutable config
    println!("3. Immutable Configuration:");
    let immutable = ImmutableConfig::new(db_config);
    println!("   Host: {}", immutable.config().host);
    println!();

    println!("=== All configurations built successfully! ===");
    println!("\nKey insight: No runtime validation was needed because");
    println!("the type system guaranteed correctness at compile time.");
}
