pub enum Errors {
	InvalidArgument { message: String },
	InvalidPath { message: String },
	RepositoryNotCloned { message: String },
	ErrorCreatingDirectory { message: String },
	ErrorCreatingFile { message: String },
	ErrorRemovingDirectory { message: String },
	ErrorReadingDirectory { message: String },
}

impl std::fmt::Debug for Errors {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Errors::InvalidArgument { message } => write!(f, "{}", message),
			Errors::InvalidPath { message } => write!(f, "{}", message),
			Errors::RepositoryNotCloned { message } => write!(f, "{}", message),
			Errors::ErrorCreatingDirectory { message } => {
				write!(f, "{}", message)
			}
			Errors::ErrorCreatingFile { message } => write!(f, "{}", message),
			Errors::ErrorRemovingDirectory { message } => {
				write!(f, "{}", message)
			}
			Errors::ErrorReadingDirectory { message } => {
				write!(f, "{}", message)
			}
		}
	}
}

impl std::error::Error for Errors {}

impl std::fmt::Display for Errors {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Errors::InvalidArgument { message } => write!(f, "Error: {}", message),
			Errors::InvalidPath { message } => write!(f, "Error: {}", message),
			Errors::RepositoryNotCloned { message } => write!(f, "Error: {}", message),
			Errors::ErrorCreatingDirectory { message } => {
				write!(f, "Error: {}", message)
			}
			Errors::ErrorCreatingFile { message } => write!(f, "Error: {}", message),
			Errors::ErrorRemovingDirectory { message } => {
				write!(f, "Error: {}", message)
			}
			Errors::ErrorReadingDirectory { message } => {
				write!(f, "Error: {}", message)
			}
		}
	}
}
