pub mod presentation;
mod processes;
pub mod runtime;

use arachnid_core::{AppError, AppResult, coordination::InstallationLease};
use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    process::{Child, Command, Stdio},
    sync::atomic::AtomicBool,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub state: String,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Direct,
    Relay,
}

// Constructed only by a reviewed backend recipe. Never deserialize a channel from IPC.
pub struct QualifiedChannel {
    pub executable: String,
    pub sha256: String,
    pub arguments: Vec<String>,
    pub kind: ChannelKind,
}

pub struct Session {
    child: Child,
    kind: ChannelKind,
    status: SessionStatus,
    _lease: InstallationLease,
    _image: File,
    _runtime: Option<runtime::VulkanRuntime>,
}

impl Session {
    pub fn start(lease: InstallationLease, channel: &QualifiedChannel) -> AppResult<Self> {
        Self::start_with_runtime(lease, channel, None)
    }

    pub fn start_with_runtime(
        lease: InstallationLease,
        channel: &QualifiedChannel,
        runtime: Option<runtime::VulkanRuntime>,
    ) -> AppResult<Self> {
        let name = &channel.executable;
        if name.contains(['/', '\\', ':']) || !name.to_ascii_lowercase().ends_with(".exe") {
            return Err(AppError::new("channelUnqualified"));
        }
        let path = lease.root().join(name);
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| AppError::new("fileUnreadable"))?;
        if arachnid_detector::is_link(&metadata) {
            return Err(AppError::new("invalidPath"));
        }
        let mut options = File::options();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.share_mode(1);
        }
        let image = options
            .open(&path)
            .map_err(|_| AppError::new("fileUnreadable"))?;
        let observed = arachnid_detector::inspect_pe(&path, name, &AtomicBool::new(false))?;
        if observed.sha256 != channel.sha256 {
            return Err(AppError::new("fileChanged"));
        }
        if processes::game_present(lease.root())? {
            return Err(AppError::new("gameAlreadyRunning"));
        }
        let mut command = Command::new(&path);
        if let Some(runtime) = &runtime {
            runtime.configure(&mut command);
        }
        let child = command
            .args(&channel.arguments)
            .current_dir(lease.root())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| AppError::new("launchFailed"))?;
        let status = SessionStatus {
            state: "running".into(),
            pid: Some(child.id()),
            exit_code: None,
        };
        Ok(Self {
            child,
            kind: channel.kind,
            status,
            _lease: lease,
            _image: image,
            _runtime: runtime,
        })
    }

    pub fn status(&mut self) -> AppResult<SessionStatus> {
        if self.status.state == "running" {
            match self.child.try_wait() {
                Ok(Some(exit)) => {
                    self.status.state = if self.kind == ChannelKind::Relay {
                        "unknown"
                    } else {
                        "exited"
                    }
                    .into();
                    self.status.exit_code = exit.code();
                }
                Ok(None) => (),
                Err(_) => self.status.state = "unknown".into(),
            }
        }
        Ok(self.status.clone())
    }
}

pub fn ensure_game_stopped(root: &Path) -> AppResult<()> {
    if processes::game_present(root)? {
        Err(AppError::new("gameAlreadyRunning"))
    } else {
        Ok(())
    }
}

// The user reports this local build works through Game.exe. Repeated Arachnid cycles remain a separate acceptance test.
pub fn qualified_game_channel(sha256: &str) -> Option<QualifiedChannel> {
    arachnid_core::game_build::supported(sha256).then(|| QualifiedChannel {
        executable: "Game.exe".into(),
        sha256: sha256.into(),
        arguments: Vec::new(),
        kind: ChannelKind::Direct,
    })
}

#[cfg(test)]
mod channel_tests {
    #[test]
    fn withdrawn_frame_patch_is_no_longer_a_launch_channel() {
        assert!(
            super::qualified_game_channel(
                "033b51e9b4b0a1592733b8140bc14f1e7cfa7c148c6f96a22f473913230ac956"
            )
            .is_none()
        );
        let hash = arachnid_core::game_build::ORIGINAL;
        let channel = super::qualified_game_channel(hash).unwrap();
        assert_eq!(channel.sha256, hash);
        assert_eq!(channel.executable, "Game.exe");
        assert!(super::qualified_game_channel(&"0".repeat(64)).is_none());
    }
}
