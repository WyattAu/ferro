use std::time::{Duration, Instant};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProgressTracker {
    multi: MultiProgress,
    users_pb: ProgressBar,
    files_pb: ProgressBar,
    shares_pb: ProgressBar,
    tags_pb: ProgressBar,
    favorites_pb: ProgressBar,
    start: Instant,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressTracker {
    pub fn new() -> Self {
        let multi = MultiProgress::new();

        let style_bytes =
            ProgressStyle::with_template("{spinner:.green} {prefix:12} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-");

        let users_pb = multi.add(ProgressBar::new(0).with_prefix("Users").with_style(style_bytes.clone()));
        let files_pb = multi.add(
            ProgressBar::new(0).with_prefix("Files").with_style(
                ProgressStyle::with_template(
                    "{spinner:.green} {prefix:12} [{bar:40.cyan/blue}] {pos}/{len} {binary_bytes_per_sec} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-"),
            ),
        );
        let shares_pb = multi.add(
            ProgressBar::new(0)
                .with_prefix("Shares")
                .with_style(style_bytes.clone()),
        );
        let tags_pb = multi.add(ProgressBar::new(0).with_prefix("Tags").with_style(style_bytes.clone()));
        let favorites_pb = multi.add(ProgressBar::new(0).with_prefix("Favorites").with_style(style_bytes));

        Self {
            multi,
            users_pb,
            files_pb,
            shares_pb,
            tags_pb,
            favorites_pb,
            start: Instant::now(),
        }
    }

    pub fn set_user_total(&self, total: u64) {
        self.users_pb.set_length(total);
    }

    pub fn inc_user(&self) {
        self.users_pb.inc(1);
    }

    pub fn set_file_total(&self, total: u64) {
        self.files_pb.set_length(total);
    }

    pub fn inc_file(&self, bytes: u64) {
        self.files_pb.inc(1);
        self.files_pb.inc(bytes);
    }

    pub fn set_share_total(&self, total: u64) {
        self.shares_pb.set_length(total);
    }

    pub fn inc_share(&self) {
        self.shares_pb.inc(1);
    }

    pub fn set_tag_total(&self, total: u64) {
        self.tags_pb.set_length(total);
    }

    pub fn inc_tag(&self) {
        self.tags_pb.inc(1);
    }

    pub fn set_favorite_total(&self, total: u64) {
        self.favorites_pb.set_length(total);
    }

    pub fn inc_favorite(&self) {
        self.favorites_pb.inc(1);
    }

    pub fn finish(&self) {
        self.multi.clear().ok();
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}
