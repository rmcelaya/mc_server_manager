#[derive(Debug)]
pub enum GenericError {
   StdIOError(std::io::Error),
   ExplainedError(String),
   Error,
}

impl From<std::io::Error> for GenericError {
   fn from(err: std::io::Error) -> Self {
      Self::StdIOError(err)
   }
}

impl From<String> for GenericError {
   fn from(s: String) -> Self {
      Self::ExplainedError(s)
   }
}

impl From<&str> for GenericError {
   fn from(s: &str) -> Self {
      Self::ExplainedError(String::from(s))
   }
}

impl From<ini::ini::Error> for GenericError {
   fn from(err: ini::ini::Error) -> Self {
      match err {
         ini::ini::Error::Io(error) => error.into(),
         ini::ini::Error::Parse(p) => GenericError::ExplainedError(format!(
            "Error in line {}, column {}: {}",
            p.line, p.col, p.msg
         )),
      }
   }
}

impl From<tbot::errors::MethodCall> for GenericError {
   fn from(err: tbot::errors::MethodCall) -> Self {
      format!("{}", err).into()
   }
}

pub type GenericResult<T> = std::result::Result<T, GenericError>;

impl std::fmt::Display for GenericError {
   fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
      match self {
         GenericError::StdIOError(e) => write!(f, "(IO error) {}", e),
         GenericError::Error => write!(
            f,
            "Internal error. This message should not have been printed"
         ),
         GenericError::ExplainedError(e) => write!(f, "{}", e),
      }
   }
}

/********** ERROR_UTILS *************/

#[macro_export]
macro_rules! unwrap_exit {
   ($e:expr) => {
      $e.unwrap_or_else(|er| {
         println!("Error: {}", er);
         process::exit(1);
      });
   };
}

#[macro_export]
macro_rules! check {
   ($e:expr) => {
      if let Err(er) = $e {
         println!("{}", er);
      }
   };
}
