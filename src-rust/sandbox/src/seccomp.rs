//! Seccomp (Secure Computing Mode) filtering
//!
//! Provides fine-grained system call filtering on Linux

use std::collections::HashSet;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Known syscalls for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Syscall {
    // File operations
    Read,
    Write,
    Open,
    Close,
    Stat,
    Fstat,
    Lstat,
    Poll,
    Preadv,
    Pwritev,
    Readv,
    Writev,
    Access,
    Pipe,
    Select,
    SchedYield,
    Mremap,
    Msync,
    Mincore,
    Madvise,
    Shmget,
    Shmat,
    Shmctl,
    Dup,
    Dup2,
    Pause,
    Nanosleep,
    Getitimer,
    Alarm,
    Setitimer,
    Getpid,
    Sendfile,
    Socket,
    Connect,
    Accept,
    Sendto,
    Recvfrom,
    Sendmsg,
    Recvmsg,
    Shutdown,
    Bind,
    Listen,
    Getsockname,
    Getpeername,
    Socketpair,
    Setsockopt,
    Getsockopt,
    Clone,
    Fork,
    Vfork,
    Execve,
    Exit,
    Wait4,
    Kill,
    Uname,
    Semget,
    Semop,
    Semctl,
    Shmdt,
    Msgget,
    Msgsnd,
    Msgrcv,
    Msgctl,
    Fcntl,
    Flock,
    Fsync,
    Fdatasync,
    Truncate,
    Ftruncate,
    Getdents,
    Getcwd,
    Chdir,
    Fchdir,
    Rename,
    Mkdir,
    Rmdir,
    Creat,
    Link,
    Unlink,
    Symlink,
    Readlink,
    Chmod,
    Fchmod,
    Chown,
    Fchown,
    Lchown,
    Umask,
    Gettimeofday,
    Getrlimit,
    Getrusage,
    Sysinfo,
    Times,
    Getuid,
    Syslog,
    Getgid,
    Setuid,
    Setgid,
    Geteuid,
    Getegid,
    Setpgid,
    Getppid,
    Getpgrp,
    Setsid,
    Setreuid,
    Setregid,
    Getgroups,
    Setgroups,
    Setresuid,
    Getresuid,
    Setresgid,
    Getresgid,
    Getpgid,
    Setfsuid,
    Setfsgid,
    Getsid,
    Capget,
    Capset,
    RtSigpending,
    RtSigqueueinfo,
    RtSigtimedwait,
    RtSigaction,
    RtSigreturn,
    Setpriority,
    Getpriority,
    Prlimit,
    Prctl,
    ArchPrctl,
    Adjtimex,
    Setrlimit,
    Getrlimit,
    Gettimeofday,
    Settimeofday,
    Gettid,
    Readahead,
    Setxattr,
    Lsetxattr,
    Fsetxattr,
    Getxattr,
    Lgetxattr,
    Fgetxattr,
    Listxattr,
    Llistxattr,
    Flistxattr,
    Removexattr,
    Lremovexattr,
    Fremovexattr,
    ExitGroup,
    EpollWait,
    EpollCtl,
    TimerfdCreate,
    Eventfd,
    Fallocate,
    TimerfdSettime,
    TimerfdGettime,
    Accept4,
    Signalfd,
    EventfdRead,
    EventfdWrite,
    EpollCreate,
    Dup3,
    Pipe2,
    InotifyInit,
    InotifyAddWatch,
    InotifyRmWatch,
    IoSetup,
    IoDestroy,
    IoGetevents,
    IoSubmit,
    IoCancel,
    LookupDcookie,
    EpollCreate1,
    Dnotify,
    Stty,
    SttyRead,
    SttyWrite,
    Ioctl,
    Fallocate,
    Fadvise64,
    ProcessVmReadv,
    ProcessVmWritev,
    Setns,
    Unshare,
    AsyncSleep,
    // Unknown syscall
    Unknown(i32),
}

impl Syscall {
    pub fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Syscall::Read,
            1 => Syscall::Write,
            2 => Syscall::Open,
            3 => Syscall::Close,
            4 => Syscall::Stat,
            5 => Syscall::Fstat,
            6 => Syscall::Lstat,
            _ => Syscall::Unknown(raw),
        }
    }
}

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeccompAction {
    /// Kill the process
    Kill,
    /// Return error
    Errno(u16),
    /// Allow the call
    Allow,
    /// Trace (for debugging)
    Trace,
}

/// A single seccomp rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompRule {
    pub syscall: Syscall,
    pub action: SeccompAction,
    pub args: Vec<SeccompArg>,
}

/// Argument matcher for syscall rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompArg {
    pub index: u8,
    pub op: SeccompOp,
    pub value: u64,
    pub mask: u64,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeccompOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    MaskedEq,
}

/// Seccomp configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompConfig {
    /// Default action for unmatched syscalls
    pub default_action: SeccompAction,
    /// List of rules
    pub rules: Vec<SeccompRule>,
    /// Whether to use NO_NEW_PRIVS
    pub no_new_privs: bool,
}

impl Default for SeccompConfig {
    fn default() -> Self {
        Self {
            default_action: SeccompAction::Errno(1),
            rules: Self::default_whitelist(),
            no_new_privs: true,
        }
    }
}

impl SeccompConfig {
    /// Default whitelist of allowed syscalls
    pub fn default_whitelist() -> Vec<SeccompRule> {
        vec![
            // File operations (read-only)
            SeccompRule { syscall: Syscall::Read, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Write, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Close, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Stat, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Fstat, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Readv, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Writev, action: SeccompAction::Allow, args: vec![] },

            // Memory
            SeccompRule { syscall: Syscall::Brk, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Mmap, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Mprotect, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Munmap, action: SeccompAction::Allow, args: vec![] },

            // Process
            SeccompRule { syscall: Syscall::Execve, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Exit, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::ExitGroup, action: SeccompAction::Allow, args: vec![] },

            // Time
            SeccompRule { syscall: Syscall::Nanosleep, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::Gettimeofday, action: SeccompAction::Allow, args: vec![] },

            // Thread
            SeccompRule { syscall: Syscall::RtSigaction, action: SeccompAction::Allow, args: vec![] },
            SeccompRule { syscall: Syscall::RtSigreturn, action: SeccompAction::Allow, args: vec![] },
        ]
    }

    /// Strict configuration with minimal syscalls
    pub fn strict() -> Self {
        Self {
            default_action: SeccompAction::Kill,
            rules: Self::default_whitelist(),
            no_new_privs: true,
        }
    }

    /// Permissive configuration (no filtering)
    pub fn permissive() -> Self {
        Self {
            default_action: SeccompAction::Allow,
            rules: vec![],
            no_new_privs: false,
        }
    }
}

/// Apply seccomp configuration (Linux only)
#[cfg(target_os = "linux")]
pub fn apply_seccomp(config: &SeccompConfig) -> Result<(), String> {
    use std::ptr::write_bytes;
    use libc::{c_void, prctl, SECCOMP_MODE_FILTER};

    // Check if we can use seccomp
    let has_cap = std::fs::metadata("/proc/self/status")
        .ok()
        .and_then(|m| {
            m.permissions().readonly().then_some(false)
        })
        .unwrap_or(true);

    if !has_cap {
        // Try running in a container or with --privileged
        return Err("Cannot apply seccomp (may need CAP_SYS_ADMIN or --privileged)".to_string());
    }

    // prctl(PR_SET_NO_NEW_PRIVS, 1) prevents privilege escalation
    if config.no_new_privs {
        unsafe {
            if prctl(prctl::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                return Err("prctl(PR_SET_NO_NEW_PRIVS) failed".to_string());
            }
        }
    }

    // TODO: Load BPF filter using seccomp syscall
    // This requires more complex BPF program generation

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn apply_seccomp(_config: &SeccompConfig) -> Result<(), String> {
    Err("Seccomp is only supported on Linux".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_from_raw() {
        assert_eq!(Syscall::from_raw(0), Syscall::Read);
        assert_eq!(Syscall::from_raw(1), Syscall::Write);
        assert_eq!(Syscall::from_raw(999), Syscall::Unknown(999));
    }

    #[test]
    fn test_strict_config() {
        let config = SeccompConfig::strict();
        assert_eq!(config.default_action, SeccompAction::Kill);
        assert!(config.no_new_privs);
    }

    #[test]
    fn test_permissive_config() {
        let config = SeccompConfig::permissive();
        assert_eq!(config.default_action, SeccompAction::Allow);
        assert!(!config.no_new_privs);
    }
}
