use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use gix::hash::ObjectId;
use jiff::Timestamp;
use tempfile::TempDir;

pub const ME: &str = "dev@example.com";

pub struct TestRepo {
    dir: TempDir,
    repository: gix::Repository,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repository = gix::init(dir.path()).unwrap();
        TestRepo { dir, repository }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn path_buf(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    pub fn git_dir(&self) -> PathBuf {
        self.path().join(".git")
    }

    pub fn append_config(&self, config: &str) {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(self.git_dir().join("config"))
            .unwrap();
        file.write_all(config.as_bytes()).unwrap();
    }

    /// commits `files` on HEAD with the current HEAD commit as the parent
    pub fn commit(&self, email: &str, time: &str, files: &[&str]) -> ObjectId {
        let parents: Vec<ObjectId> = self
            .repository
            .head_id()
            .map(|id| id.detach())
            .into_iter()
            .collect();
        self.commit_on("HEAD", email, time, files, &parents)
    }

    /// writes a tree that holds exactly `files`, so files left out count as deleted
    pub fn commit_on(
        &self,
        reference: &str,
        email: &str,
        time: &str,
        files: &[&str],
        parents: &[ObjectId],
    ) -> ObjectId {
        let seconds = time.parse::<Timestamp>().unwrap().as_second();
        let mut entries: Vec<_> = files
            .iter()
            .map(|name| gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: (*name).into(),
                oid: self
                    .repository
                    .write_blob(format!("{name} {seconds}"))
                    .unwrap()
                    .detach(),
            })
            .collect();
        entries.sort_by(|a, b| a.filename.cmp(&b.filename));
        let tree = self
            .repository
            .write_object(gix::objs::Tree { entries })
            .unwrap();
        let time = format!("{seconds} +0000");
        let signature = gix::actor::SignatureRef {
            name: "dev".into(),
            email: email.into(),
            time: &time,
        };
        self.repository
            .commit_as(
                signature,
                signature,
                reference,
                "change",
                tree,
                parents.iter().copied(),
            )
            .unwrap()
            .detach()
    }

    pub fn detach_head(&self, id: ObjectId) {
        fs::write(self.git_dir().join("HEAD"), format!("{id}\n")).unwrap();
    }

    pub fn delete_ref(&self, name: &str) {
        fs::remove_file(self.git_dir().join(name)).unwrap();
    }

    /// cuts history at `boundary` and deletes `dropped`, like a shallow clone that never fetched it
    pub fn make_shallow(&self, boundary: ObjectId, dropped: ObjectId) {
        fs::write(self.git_dir().join("shallow"), format!("{boundary}\n")).unwrap();
        let hex = dropped.to_string();
        let object = self
            .git_dir()
            .join("objects")
            .join(&hex[..2])
            .join(&hex[2..]);
        fs::remove_file(object).unwrap();
    }

    /// adds a linked worktree in `name` that has `branch` checked out
    pub fn add_worktree(&self, name: &str, branch: &str) -> PathBuf {
        let worktree = self.path().join(name);
        let admin = self.git_dir().join("worktrees").join(name);
        fs::create_dir_all(&worktree).unwrap();
        fs::create_dir_all(&admin).unwrap();
        fs::write(admin.join("HEAD"), format!("ref: refs/heads/{branch}\n")).unwrap();
        fs::write(admin.join("commondir"), "../..\n").unwrap();
        let worktree_git = worktree.join(".git");
        fs::write(
            admin.join("gitdir"),
            format!("{}\n", worktree_git.display()),
        )
        .unwrap();
        fs::write(&worktree_git, format!("gitdir: {}\n", admin.display())).unwrap();
        worktree
    }
}
