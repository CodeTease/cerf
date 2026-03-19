use std::fs;
use std::path::Path;

pub fn get_mode_string(meta: &fs::Metadata) -> String {
    let mut s = String::new();
    let ft = meta.file_type();

    if ft.is_dir() {
        s.push('d');
    } else if ft.is_symlink() {
        s.push('l');
    } else if ft.is_file() {
        s.push('-');
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::FileTypeExt;
            if ft.is_block_device() {
                s.push('b');
            } else if ft.is_char_device() {
                s.push('c');
            } else if ft.is_fifo() {
                s.push('p');
            } else if ft.is_socket() {
                s.push('s');
            } else {
                s.push('?');
            }
        }
        #[cfg(not(unix))]
        {
            s.push('?');
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        let chars = [
            (0o400, 'r'),
            (0o200, 'w'),
            (0o100, 'x'),
            (0o040, 'r'),
            (0o020, 'w'),
            (0o010, 'x'),
            (0o004, 'r'),
            (0o002, 'w'),
            (0o001, 'x'),
        ];
        for (m, c) in chars {
            if mode & m != 0 {
                s.push(c);
            } else {
                s.push('-');
            }
        }
    }
    #[cfg(windows)]
    {
        let readonly = meta.permissions().readonly();
        s.push('r');
        s.push(if readonly { '-' } else { 'w' });
        s.push('-');
        s.push('r');
        s.push(if readonly { '-' } else { 'w' });
        s.push('-');
        s.push('r');
        s.push(if readonly { '-' } else { 'w' });
        s.push('-');
    }
    #[cfg(not(any(unix, windows)))]
    {
        s.push_str("---------");
    }
    s
}

pub fn get_nlink(_meta: &fs::Metadata) -> u64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.nlink()
    }
    #[cfg(windows)]
    {
        1 // number_of_links is currently unstable on Windows
    }
    #[cfg(not(any(unix, windows)))]
    {
        1
    }
}

#[allow(unused_variables)]
pub fn get_owner_group(meta: &fs::Metadata) -> (String, String) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (meta.uid().to_string(), meta.gid().to_string())
    }
    #[cfg(not(unix))]
    {
        ("unknown".to_string(), "unknown".to_string())
    }
}

#[allow(unused_variables)]
pub fn is_executable(path: &Path, m: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        m.permissions().mode() & 0o111 != 0
    }
    #[cfg(windows)]
    {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(ext.as_str(), "exe" | "bat" | "cmd" | "ps1" | "com")
        } else {
            false
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}
