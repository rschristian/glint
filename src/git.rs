use std::env::current_dir;
use std::ffi::OsStr;
use std::fmt;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};

mod parse_log;

pub use parse_log::LogItem;

#[derive(Debug, Clone)]
pub struct Git {
    cwd: PathBuf,
    repo_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GitStatus(pub Vec<GitStatusItem>);

#[derive(Debug, Clone)]
pub struct GitStatusItem {
    file_name: String,
    staged: Option<GitStatusType>,
    unstaged: Option<GitStatusType>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum GitStatusType {
    Added,
    Modified,
    Renamed,
    Untracked,
    Deleted,
    None,
}

#[derive(Debug)]
pub enum GitError {
    NotGitRepo,
    Io(io::Error),
}

impl Git {
    pub fn from_cwd() -> Result<Self, GitError> {
        let cwd = current_dir().map_err(GitError::Io)?;

        let mut repo_root = None;

        for dir in cwd.ancestors() {
            if dir.join(".git").is_dir() {
                repo_root = Some(dir.into());

                break;
            }
        }

        match repo_root {
            Some(repo_root) => Ok(Git { cwd, repo_root }),
            None => Err(GitError::NotGitRepo),
        }
    }

    pub fn commit<I>(&self, message: &str, other_args: impl IntoIterator<Item = I>) -> io::Result<ExitStatus>
    where
        I: AsRef<OsStr>,
    {
        Command::new("git")
            .current_dir(&self.cwd)
            .args(&["commit", "-m", message])
            .args(other_args.into_iter())
            .status()
    }

    pub fn log<I>(&self, other_args: impl IntoIterator<Item = I>) -> io::Result<Child>
    where
        I: AsRef<OsStr>,
    {
        Command::new("git")
            .current_dir(&self.cwd)
            .args(&["log", "--raw", "--pretty=raw"])
            .args(other_args.into_iter())
            .stdout(Stdio::piped())
            .spawn()
    }

    pub fn log_parsed<I>(&self, other_args: impl IntoIterator<Item = I>) -> io::Result<Vec<LogItem>>
    where
        I: AsRef<OsStr>,
    {
        let log_stdout = self.log(other_args)?.stdout.expect("must be able to access stdout");
        Ok(parse_log::parse_logs(
            BufReader::new(log_stdout).lines().filter_map(Result::ok),
        ))
    }

    /// Stages files using `git add`. Run from the repo root.gs
    pub fn add<I>(&self, files: impl IntoIterator<Item = I>) -> io::Result<()>
    where
        I: AsRef<OsStr>,
    {
        Command::new("git")
            .current_dir(&self.repo_root)
            .args(&["add", "--"])
            .args(files.into_iter())
            .status()?;
        Ok(())
    }

    pub fn diff_less<I>(&self, files: impl IntoIterator<Item = I>) -> io::Result<()>
    where
        I: AsRef<OsStr>,
    {
        let diff = Command::new("git")
            .current_dir(&self.repo_root)
            .args(&["diff", "--color=always", "--"])
            .args(files.into_iter())
            .stdout(Stdio::piped())
            .spawn()?;

        Command::new("less")
            .current_dir(&self.repo_root)
            .arg("-R")
            .stdin(diff.stdout.ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "failed to get stdout of git diff")
            })?)
            .status()?;

        Ok(())
    }

    pub fn status(&self) -> io::Result<GitStatus> {
        let command = Command::new("git")
            .current_dir(&self.cwd)
            .args(&["status", "--porcelain"])
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = command.stdout.ok_or_else(|| {
            io::Error::new(io::ErrorKind::Other, "Could not capture standard output.")
        })?;

        let items = BufReader::new(stdout)
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| {
                let mut chars = line.chars();
                let staged = chars
                    .next()
                    .and_then(GitStatusType::from_char)
                    .filter(|item| match item {
                        GitStatusType::Untracked => false,
                        _ => true,
                    });
                let unstaged = chars.next().and_then(GitStatusType::from_char);

                chars.next();
                let file: String = chars.collect();

                if file.is_empty() {
                    None
                } else {
                    Some(GitStatusItem {
                        file_name: file,
                        staged,
                        unstaged,
                    })
                }
            })
            .collect();

        Ok(GitStatus(items))
    }
}
impl GitStatus {
    pub fn iter(&self) -> impl Iterator<Item = &GitStatusItem> {
        self.0.iter()
    }

    pub fn any_staged(&self) -> bool {
        self.iter().any(|item| item.staged.is_some())
    }

    pub fn any_unstaged(&self) -> bool {
        self.iter().any(|item| item.unstaged.is_some())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl GitStatusItem {
    pub fn new(file_name: String) -> Self {
        GitStatusItem {
            file_name,
            staged: None,
            unstaged: None,
        }
    }
    pub fn file_name(&self) -> &str {
        &self.file_name
    }
    pub fn status(&self) -> &GitStatusType {
        self.unstaged.as_ref().unwrap_or(&GitStatusType::None)
    }
}

impl Into<String> for GitStatusItem {
    fn into(self) -> String {
        (&self).into()
    }
}

impl Into<String> for &'_ GitStatusItem {
    fn into(self) -> String {
        self.file_name().into()
    }
}

impl GitStatusType {
    pub fn from_char(ch: char) -> Option<Self> {
        match ch {
            'A' => Some(GitStatusType::Added),
            'M' => Some(GitStatusType::Modified),
            'R' => Some(GitStatusType::Renamed),
            'D' => Some(GitStatusType::Deleted),
            '?' => Some(GitStatusType::Untracked),
            _ => None,
        }
    }
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotGitRepo => write!(f, "This directory is not a git repository."),
            GitError::Io(err) => write!(f, "Internal I/O error: {}", err),
        }
    }
}
