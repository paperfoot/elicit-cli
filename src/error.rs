/// Error types with semantic exit codes.
///
/// Every error maps to an exit code (1-4), a machine-readable code, and a
/// recovery suggestion that agents can follow literally.
///
/// Variants carry an optional override suggestion so the HTTP layer can attach
/// a context-specific recovery instruction (e.g. a parsed Retry-After) while
/// still falling back to a sane default for hand-constructed errors.

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // Some variants demonstrate the full exit code contract (0-4)
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Transient(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Update failed: {0}")]
    Update(String),

    /// Like the above but carries a tailored suggestion string. Used by the
    /// API client so the suggestion can embed parsed reset/Retry-After values
    /// or a specific settings URL.
    #[error("{message}")]
    Detailed {
        kind: Kind,
        message: String,
        suggestion: String,
    },
}

/// Logical category for a `Detailed` error, mirroring the simple variants so
/// exit-code and error-code mapping stays in one place.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    InvalidInput,
    Config,
    Transient,
    RateLimited,
}

impl AppError {
    /// Construct a config-class error (exit 2) with a custom suggestion.
    pub fn config_with(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::Detailed {
            kind: Kind::Config,
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Construct an invalid-input error (exit 3) with a custom suggestion.
    pub fn invalid_with(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::Detailed {
            kind: Kind::InvalidInput,
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Construct a rate-limited error (exit 4) with a custom suggestion.
    pub fn rate_limited_with(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::Detailed {
            kind: Kind::RateLimited,
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Construct a transient error (exit 1) with a custom suggestion.
    pub fn transient_with(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::Detailed {
            kind: Kind::Transient,
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 3,
            Self::Config(_) => 2,
            Self::RateLimited(_) => 4,
            Self::Transient(_) | Self::Io(_) | Self::Update(_) => 1,
            Self::Detailed { kind, .. } => match kind {
                Kind::InvalidInput => 3,
                Kind::Config => 2,
                Kind::RateLimited => 4,
                Kind::Transient => 1,
            },
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Config(_) => "config_error",
            Self::Transient(_) => "transient_error",
            Self::RateLimited(_) => "rate_limited",
            Self::Io(_) => "io_error",
            Self::Update(_) => "update_error",
            Self::Detailed { kind, .. } => match kind {
                Kind::InvalidInput => "invalid_input",
                Kind::Config => "config_error",
                Kind::Transient => "transient_error",
                Kind::RateLimited => "rate_limited",
            },
        }
    }

    pub fn suggestion(&self) -> &str {
        match self {
            Self::InvalidInput(_) => {
                concat!("Check arguments with: ", env!("CARGO_BIN_NAME"), " --help")
            }
            Self::Config(_) => concat!(
                "Set ELICIT_API_KEY or check config with: ",
                env!("CARGO_BIN_NAME"),
                " config show"
            ),
            Self::Transient(_) | Self::Io(_) => "Retry the command",
            Self::RateLimited(_) => "Wait a moment and retry",
            Self::Update(_) => concat!(
                "Retry later, or install manually via cargo install ",
                env!("CARGO_BIN_NAME")
            ),
            Self::Detailed { suggestion, .. } => suggestion,
        }
    }
}
