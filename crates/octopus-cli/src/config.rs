pub(crate) fn normalize_optional_args(args: Option<&str>) -> Option<&str> {
    args.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) fn is_help_arg(arg: &str) -> bool {
    matches!(arg, "help" | "-h" | "--help")
}

pub(crate) fn help_path_from_args(args: &str) -> Option<Vec<&str>> {
    let parts = args.split_whitespace().collect::<Vec<_>>();
    let help_index = parts.iter().position(|part| is_help_arg(part))?;
    Some(parts[..help_index].to_vec())
}

#[cfg(test)]
mod tests {
    use super::{help_path_from_args, is_help_arg, normalize_optional_args};

    #[test]
    fn trims_optional_args() {
        assert_eq!(normalize_optional_args(Some("  env  ")), Some("env"));
        assert_eq!(normalize_optional_args(Some("   ")), None);
        assert_eq!(normalize_optional_args(None), None);
    }

    #[test]
    fn recognizes_help_args() {
        assert!(is_help_arg("help"));
        assert!(is_help_arg("--help"));
        assert!(!is_help_arg("list"));
    }

    #[test]
    fn extracts_help_path() {
        assert_eq!(help_path_from_args("install --help"), Some(vec!["install"]));
        assert_eq!(help_path_from_args("list"), None);
    }
}
