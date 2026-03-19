pub struct LsArgs {
    pub all: bool,
    pub almost_all: bool,
    pub classify: bool,
    pub long_format: bool,
    pub human_readable: bool,
    pub sort_time: bool,
    pub sort_size: bool,
    pub reverse: bool,
    pub single_column: bool,
    pub targets: Vec<String>,
    pub recursive: bool,
    pub group_directories_first: bool,
    pub quote_name: bool,
}

impl LsArgs {
    pub fn parse(args: &[String]) -> Self {
        let mut all = false;
        let mut almost_all = false;
        let mut classify = false;
        let mut long_format = false;
        let mut human_readable = false;
        let mut sort_time = false;
        let mut sort_size = false;
        let mut reverse = false;
        let mut single_column = false;
        let mut recursive = false;
        let mut group_directories_first = false;
        let mut quote_name = false;
        let mut targets = Vec::new();

        for arg in args {
            if arg == "--group-directories-first" {
                group_directories_first = true;
                continue;
            }
            if arg.starts_with("--") {
                // Ignore other unknown double-dash flags
                continue;
            } else if arg.starts_with('-') && arg.len() > 1 {
                for c in arg[1..].chars() {
                    match c {
                        'a' => {
                            all = true;
                            almost_all = false;
                        }
                        'A' => {
                            almost_all = true;
                            all = false;
                        }
                        'F' => classify = true,
                        'l' => long_format = true,
                        'h' => human_readable = true,
                        't' => sort_time = true,
                        'S' => sort_size = true,
                        'r' => reverse = true,
                        '1' => single_column = true,
                        'R' => recursive = true,
                        'Q' => quote_name = true,
                        _ => {}
                    }
                }
            } else {
                targets.push(arg.clone());
            }
        }

        let targets = if targets.is_empty() {
            vec![".".to_string()]
        } else {
            targets
        };

        Self {
            all,
            almost_all,
            classify,
            long_format,
            human_readable,
            sort_time,
            sort_size,
            reverse,
            single_column,
            recursive,
            group_directories_first,
            quote_name,
            targets,
        }
    }
}
