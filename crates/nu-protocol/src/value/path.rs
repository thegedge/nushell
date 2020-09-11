use std::fmt::Write;
use std::iter::FromIterator;
use std::path::{self, PathBuf};

use serde::{Deserialize, Serialize};

const DEFAULT_SEPARATOR_CHAR: char = std::path::MAIN_SEPARATOR;
const DRIVE_SEPARATOR_CHAR: char = ':';
const PARENT_CHAR: char = '.';
const HOME_DIR_CHAR: char = '~';

const HOME_DIR_STR: &str = "~";
const PARENT_STR: &str = ".";

const SEPARATOR_CHARS: &[char] = &['/', '\\'];

/// A path prefix.
///
/// Typically seen on Windows, some examples include drive references, like "C:\", and network
/// locations, like "\\network\path".
///
/// See std::path::Prefix for more information
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum Prefix {
    /// Verbatim prefix, for example, \\?\folder
    Verbatim(String),

    /// Verbatim prefix using Windows' Uniform Naming Convention, for example, \\?\UNC\server\share
    VerbatimUNC(String, String),

    /// Verbatim disk prefix, for example, \\?\C:
    VerbatimDisk(char),

    /// Device namespace prefix, for example,, \\.\COM42
    DeviceNS(String),

    /// Prefix using Windows' Uniform Naming Convention, for example, \\server\share
    UNC(String, String),

    /// Prefix for a disk drive, for example, C:
    Disk(char),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub enum Component {
    /// The root of the filesystem.
    Root,

    /// Home directory, represented by ~ for current user, or ~<user>
    HomeDir(Option<String>),

    /// Any regular part of a path, like "mypath" in /usr/local/mypath/
    Normal(String),

    /// Reference to a parent directory, represented by a series of dots.
    ///
    /// One dot is the current directory.
    /// Two or more dots is a current directory's ancestor.
    Parent(u8),

    /// Path prefix.
    ///
    /// This path component, if it exists in a Path, will always be the first component, and there
    /// should only be one.
    Prefix(Prefix),
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub struct Path {
    components: Vec<Component>,
    separator: char,
}

impl Prefix {
    fn format_with_separator<W>(&self, sep: char, f: &mut W) -> std::fmt::Result
    where
        W: Write,
    {
        match self {
            Prefix::Verbatim(s) => write!(f, "{}{}?{}{}", sep, sep, sep, s),
            Prefix::VerbatimUNC(a, b) => write!(f, "{}{}?{}UNC{}{}{}", sep, sep, sep, a, sep, b),
            Prefix::VerbatimDisk(d) => {
                write!(f, "{}{}?{}{}{}", sep, sep, sep, d, DRIVE_SEPARATOR_CHAR)
            }
            Prefix::DeviceNS(s) => write!(f, "{}{}.{}{}", sep, sep, sep, s),
            Prefix::UNC(a, b) => write!(f, "{}{}{}{}{}", sep, sep, a, sep, b),
            Prefix::Disk(d) => write!(f, "{}{}", d, DRIVE_SEPARATOR_CHAR),
        }
    }
}

impl Component {
    fn format_with_separator<W>(&self, sep: char, f: &mut W) -> std::fmt::Result
    where
        W: Write,
    {
        match self {
            Component::Root => Ok(()),
            Component::HomeDir(None) => f.write_str(HOME_DIR_STR),
            Component::HomeDir(Some(user)) => write!(f, "{}{}", HOME_DIR_CHAR, user),
            Component::Normal(name) => f.write_str(&name),
            Component::Parent(ndots) => f.write_str(&PARENT_STR.repeat(*ndots as usize)),
            Component::Prefix(prefix) => prefix.format_with_separator(sep, f),
        }
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if !self.components.is_empty() {
            self.components[0].format_with_separator(self.separator, f)?;
            for component in self.components[1..].iter() {
                write!(f, "{}", self.separator)?;
                component.format_with_separator(self.separator, f)?;
            }
        }

        Ok(())
    }
}

impl From<path::PrefixComponent<'_>> for Prefix {
    fn from(p: path::PrefixComponent) -> Prefix {
        match p.kind() {
            path::Prefix::Verbatim(s) => Prefix::Verbatim(s.to_string_lossy().into()),
            path::Prefix::VerbatimUNC(a, b) => {
                Prefix::VerbatimUNC(a.to_string_lossy().into(), b.to_string_lossy().into())
            }
            path::Prefix::VerbatimDisk(b) => Prefix::VerbatimDisk(b.into()),
            path::Prefix::DeviceNS(s) => Prefix::DeviceNS(s.to_string_lossy().into()),
            path::Prefix::UNC(a, b) => {
                Prefix::UNC(a.to_string_lossy().into(), b.to_string_lossy().into())
            }
            path::Prefix::Disk(b) => Prefix::Disk(b.into()),
        }
    }
}

impl From<path::Component<'_>> for Component {
    fn from(c: path::Component) -> Component {
        match c {
            path::Component::Prefix(p) => Component::Prefix(p.into()),
            path::Component::RootDir => Component::Root,
            path::Component::CurDir => Component::Parent(1),
            path::Component::ParentDir => Component::Parent(2),
            path::Component::Normal(s) => {
                let s = s.to_string_lossy();
                if s.starts_with(HOME_DIR_CHAR) {
                    if s.len() == HOME_DIR_CHAR.len_utf8() {
                        Component::HomeDir(None)
                    } else {
                        Component::HomeDir(Some(s.into()))
                    }
                } else if s.chars().all(|c| c == PARENT_CHAR) {
                    Component::Parent((s.len() / PARENT_CHAR.len_utf8()) as u8)
                } else {
                    Component::Normal(s.into())
                }
            }
        }
    }
}

impl From<&str> for Path {
    fn from(s: &str) -> Path {
        let separator = s
            .chars()
            .find(|c| SEPARATOR_CHARS.contains(c))
            .unwrap_or(DEFAULT_SEPARATOR_CHAR);

        let separator_str = separator.to_string();

        // Use the default separator everywhere so we can take advantage of `PathBuf`
        let s = s.replace(SEPARATOR_CHARS, &separator_str);

        let components = PathBuf::from(s).components().map(Component::from).collect();

        Path {
            components,
            separator,
        }
    }
}

impl From<&String> for Path {
    fn from(p: &String) -> Path {
        Path::from(p.as_str())
    }
}

impl From<PathBuf> for Path {
    fn from(p: PathBuf) -> Path {
        let components = p.components().map(Component::from).collect();

        Path {
            components,
            separator: DEFAULT_SEPARATOR_CHAR,
        }
    }
}

impl From<&Path> for PathBuf {
    fn from(p: &Path) -> PathBuf {
        PathBuf::from_iter(p.components.iter().map(|c| {
            let mut buf = String::new();
            let _ = c.format_with_separator(DEFAULT_SEPARATOR_CHAR, &mut buf);
            buf
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::Component::*;
    use super::*;

    #[test]
    fn supports_forward_slash_chars() {
        let path = Path::from("a/b/c.txt");

        assert_eq!(
            Path {
                components: vec![
                    Normal("a".to_string()),
                    Normal("b".to_string()),
                    Normal("c.txt".to_string()),
                ],
                separator: '/',
            },
            path
        );
    }

    #[test]
    fn supports_back_slash_chars() {
        let path = Path::from("a\\b\\c.txt");

        assert_eq!(
            Path {
                components: vec![
                    Normal("a".to_string()),
                    Normal("b".to_string()),
                    Normal("c.txt".to_string()),
                ],
                separator: '\\',
            },
            path
        );
    }
}
