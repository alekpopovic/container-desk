//! Conservative alias discovery, never effective configuration evaluation.
//! No process API, DNS lookup, environment expansion or writes occur here.
use crate::domain::*;
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::{Component, Path, PathBuf},
};
const FILE_BYTES: usize = 1024 * 1024;
const TOTAL_BYTES: usize = 4 * FILE_BYTES;
const MAX_FILES: usize = 128;
const MAX_DEPTH: usize = 16;
const MAX_ENTRIES: usize = 4096;

pub fn config_path(home: &Path, selected: Option<&str>) -> Result<String, AppError> {
    let path = selected
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".ssh/config"));
    let Some(text) = path.to_str() else {
        return Err(AppError::new(ErrorCode::InvalidConfigPath));
    };
    if !path.is_absolute() || text.len() > 4096 || text.chars().any(char::is_control) {
        return Err(AppError::new(ErrorCode::InvalidConfigPath));
    }
    // Resolving a reference doesn't open the file or evaluate configuration.
    Ok(path.to_string_lossy().into_owned())
}

pub fn discover(home: &Path, selected: &str) -> Result<HostDiscovery, AppError> {
    let path = config_path(home, Some(selected))?;
    let mut reader = Reader {
        home,
        active: HashSet::new(),
        candidates: BTreeMap::new(),
        warnings: vec![],
        files: 0,
        bytes: 0,
        entries: 0,
    };
    let mut conditional = false;
    reader.file(Path::new(&path), 0, &mut conditional);
    Ok(HostDiscovery {
        config_path: path,
        candidates: reader.candidates.into_values().collect(),
        warnings: reader.warnings,
    })
}
struct Reader<'a> {
    home: &'a Path,
    active: HashSet<PathBuf>,
    candidates: BTreeMap<String, SshCandidate>,
    warnings: Vec<DiscoveryWarning>,
    files: usize,
    bytes: usize,
    entries: usize,
}
impl Reader<'_> {
    fn warn(&mut self, code: DiscoveryWarningCode, path: &Path, line: Option<u32>) {
        if self
            .warnings
            .iter()
            .any(|w| w.code == code && w.source == path.to_string_lossy() && w.line == line)
        {
            return;
        }
        if self.warnings.len() < 127 {
            self.warnings.push(DiscoveryWarning {
                code,
                source: path.to_string_lossy().into_owned(),
                line,
            });
        } else if self.warnings.len() == 127 {
            self.warnings.push(DiscoveryWarning {
                code: DiscoveryWarningCode::LimitReached,
                source: path.to_string_lossy().into_owned(),
                line: None,
            });
        }
    }
    fn file(&mut self, path: &Path, depth: usize, conditional: &mut bool) {
        if depth >= MAX_DEPTH || self.files >= MAX_FILES || self.bytes >= TOTAL_BYTES {
            self.warn(DiscoveryWarningCode::LimitReached, path, None);
            return;
        }
        self.files += 1;
        let canonical = match fs::canonicalize(path) {
            Ok(path) => path,
            Err(error) => {
                self.warn(
                    if error.kind() == std::io::ErrorKind::NotFound {
                        DiscoveryWarningCode::MissingFile
                    } else {
                        DiscoveryWarningCode::UnreadableFile
                    },
                    path,
                    None,
                );
                return;
            }
        };
        if !self.active.insert(canonical.clone()) {
            self.warn(DiscoveryWarningCode::IncludeCycle, path, None);
            return;
        }
        self.read_file(&canonical, depth, conditional);
        self.active.remove(&canonical);
    }
    fn read_file(&mut self, path: &Path, depth: usize, conditional: &mut bool) {
        // O_NONBLOCK avoids FIFO hangs; metadata after open rejects devices/directories.
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path);
        let Ok(file) = file else {
            self.warn(DiscoveryWarningCode::UnreadableFile, path, None);
            return;
        };
        let Ok(metadata) = file.metadata() else {
            self.warn(DiscoveryWarningCode::UnreadableFile, path, None);
            return;
        };
        if !metadata.is_file() {
            self.warn(DiscoveryWarningCode::UnreadableFile, path, None);
            return;
        }
        let limit = FILE_BYTES.min(TOTAL_BYTES - self.bytes);
        if metadata.len() > limit as u64 {
            self.warn(DiscoveryWarningCode::LimitReached, path, None);
            return;
        }
        let mut bytes = Vec::new();
        if file
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .is_err()
        {
            self.warn(DiscoveryWarningCode::UnreadableFile, path, None);
            return;
        }
        self.bytes += bytes.len();
        if bytes.len() > limit {
            self.warn(DiscoveryWarningCode::LimitReached, path, None);
            return;
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            self.warn(DiscoveryWarningCode::UnsupportedSyntax, path, None);
            return;
        };
        for (index, line) in text.lines().enumerate() {
            let number = Some(index as u32 + 1);
            if line.len() > 16 * 1024 || line.chars().any(|c| c.is_control() && c != '\t') {
                self.warn(DiscoveryWarningCode::UnsupportedSyntax, path, number);
                *conditional = true;
                continue;
            }
            let line = line.trim_start();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let end = line
                .find(|c: char| c.is_ascii_whitespace() || c == '=')
                .unwrap_or(line.len());
            let keyword = &line[..end];
            // Ignore executable directives entirely; never tokenize their commands for output.
            if !["host", "include", "match"]
                .iter()
                .any(|key| keyword.eq_ignore_ascii_case(key))
            {
                continue;
            }
            let arguments = line[end..]
                .trim_start()
                .strip_prefix('=')
                .unwrap_or(line[end..].trim_start())
                .trim_start();
            if keyword.eq_ignore_ascii_case("match") {
                *conditional = true;
                self.warn(DiscoveryWarningCode::MatchSkipped, path, number);
                continue;
            }
            let Some(tokens) = tokenize(arguments) else {
                *conditional = true;
                self.warn(DiscoveryWarningCode::UnsupportedSyntax, path, number);
                continue;
            };
            if tokens.is_empty() {
                *conditional = true;
                self.warn(DiscoveryWarningCode::UnsupportedSyntax, path, number);
                continue;
            }
            if keyword.eq_ignore_ascii_case("host") {
                *conditional = !(tokens.len() == 1 && tokens[0] == "*");
                for alias in tokens {
                    if super::validate_alias(&alias).is_err() {
                        self.warn(DiscoveryWarningCode::PatternsSkipped, path, number);
                        continue;
                    }
                    if self.candidates.contains_key(&alias) {
                        continue;
                    }
                    if self.candidates.len() >= 1000 {
                        self.warn(DiscoveryWarningCode::LimitReached, path, number);
                        break;
                    }
                    self.candidates.insert(
                        alias.clone(),
                        SshCandidate {
                            alias,
                            source: path.to_string_lossy().into_owned(),
                            line: index as u32 + 1,
                        },
                    );
                }
            } else if *conditional {
                self.warn(DiscoveryWarningCode::ConditionalInclude, path, number);
            } else {
                for token in tokens {
                    let Some(pattern) = include_path(self.home, &token) else {
                        self.warn(DiscoveryWarningCode::UnsupportedSyntax, path, number);
                        continue;
                    };
                    match expand(&pattern, &mut self.entries) {
                        Ok(files) if files.is_empty() => {
                            self.warn(DiscoveryWarningCode::MissingFile, path, number)
                        }
                        Ok(files) => {
                            for file in files {
                                self.file(&file, depth + 1, conditional);
                            }
                        }
                        Err(code) => self.warn(code, path, number),
                    }
                }
            }
        }
    }
}
fn include_path(home: &Path, token: &str) -> Option<PathBuf> {
    // Only literal paths and simple globs are discoverable without evaluating config context.
    if token.contains(['$', '%', '[', ']', '{', '}', '\\'])
        || token.is_empty()
        || token.len() > 4096
    {
        return None;
    }
    if let Some(relative) = token.strip_prefix("~/") {
        return Some(home.join(relative));
    }
    if token.starts_with('~') {
        return None;
    }
    let path = Path::new(token);
    Some(if path.is_absolute() {
        path.into()
    } else {
        home.join(".ssh").join(path)
    })
}
fn tokenize(text: &str) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '#' if !quoted => break,
            '"' => {
                quoted = !quoted;
                started = true;
            }
            '\\' => {
                let next = chars.next()?;
                if !['"', '\\'].contains(&next) {
                    return None;
                }
                token.push(next);
                started = true;
            }
            c if c.is_ascii_whitespace() && !quoted => {
                if started {
                    tokens.push(std::mem::take(&mut token));
                    started = false;
                }
            }
            c => {
                token.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return None;
    }
    if started {
        tokens.push(token);
    }
    Some(tokens)
}
fn expand(pattern: &Path, entries: &mut usize) -> Result<Vec<PathBuf>, DiscoveryWarningCode> {
    let mut paths = vec![PathBuf::from("/")];
    for part in pattern.components() {
        let token = match part {
            Component::RootDir | Component::CurDir => continue,
            Component::ParentDir => "..",
            Component::Normal(s) => s.to_str().ok_or(DiscoveryWarningCode::UnsupportedSyntax)?,
            _ => return Err(DiscoveryWarningCode::UnsupportedSyntax),
        };
        if token.contains(['*', '?']) {
            let mut matches = Vec::new();
            for directory in paths {
                let children = match fs::read_dir(&directory) {
                    Ok(v) => v,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(_) => return Err(DiscoveryWarningCode::UnreadableFile),
                };
                for child in children {
                    *entries += 1;
                    if *entries > MAX_ENTRIES {
                        return Err(DiscoveryWarningCode::LimitReached);
                    }
                    let child = child.map_err(|_| DiscoveryWarningCode::UnreadableFile)?;
                    let name = child.file_name();
                    let Some(name) = name.to_str() else {
                        continue;
                    };
                    if (!name.starts_with('.') || token.starts_with('.'))
                        && wildcard(token.as_bytes(), name.as_bytes())
                    {
                        matches.push(child.path());
                    }
                }
            }
            paths = matches;
        } else {
            paths = paths.into_iter().map(|path| path.join(token)).collect();
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}
// Bounded greedy matching of * and ? for one filesystem component; no recursive ** semantics.
fn wildcard(pattern: &[u8], name: &[u8]) -> bool {
    let (mut p, mut n, mut star, mut restart) = (0, 0, None, 0);
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some(p);
            p += 1;
            restart = n;
        } else if let Some(s) = star {
            p = s + 1;
            restart += 1;
            n = restart;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests;
