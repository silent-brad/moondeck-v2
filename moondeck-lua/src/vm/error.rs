use std::fmt;

#[derive(Debug)]
pub enum VmError {
    /// Compile-time error
    Compile {
        message: String,
        line: usize,
        col: usize,
    },
    /// Runtime error
    Runtime(String),
    /// Type error
    Type {
        expected: &'static str,
        got: &'static str,
    },
    /// Stack overflow
    StackOverflow,
    /// Out of fuel
    OutOfFuel,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmError::Compile { message, line, col } => {
                write!(f, "compile error at {line}:{col}: {message}")
            }
            VmError::Runtime(msg) => write!(f, "runtime error: {msg}"),
            VmError::Type { expected, got } => {
                write!(f, "type error: expected {expected}, got {got}")
            }
            VmError::StackOverflow => write!(f, "stack overflow"),
            VmError::OutOfFuel => write!(f, "out of fuel (execution limit exceeded)"),
        }
    }
}

impl std::error::Error for VmError {}
