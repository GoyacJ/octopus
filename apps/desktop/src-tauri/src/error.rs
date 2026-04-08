use octopus_core::AppError;

pub type ShellResult<T> = Result<T, AppError>;

pub fn into_command_error(error: AppError) -> String {
    error.to_string()
}
