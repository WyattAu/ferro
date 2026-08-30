use std::sync::Arc;
use std::time::{Duration, Instant};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct ProgressTracker {
    inner: Arc<ProgressTrackerInner>,
}

struct ProgressTrackerInner {
    multi: MultiProgress,
    users_pb: ProgressBar,
    files_pb: ProgressBar,
    shares_pb: ProgressBar,
    tags_pb: ProgressBar,
    favorites_pb: ProgressBar,
    start: Instant,
}

impl Clone for ProgressTracker {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self::new_visible(true)
    }

    pub fn new_visible(visible: bool) -> Self {
        let multi = MultiProgress::new();

        if !visible {
            return Self {
                inner: Arc::new(ProgressTrackerInner {
                    multi,
                    users_pb: ProgressBar::hidden(),
                    files_pb: ProgressBar::hidden(),
                    shares_pb: ProgressBar::hidden(),
                    tags_pb: ProgressBar::hidden(),
                    favorites_pb: ProgressBar::hidden(),
                    start: Instant::now(),
                }),
            };
        }

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
            inner: Arc::new(ProgressTrackerInner {
                multi,
                users_pb,
                files_pb,
                shares_pb,
                tags_pb,
                favorites_pb,
                start: Instant::now(),
            }),
        }
    }

    pub fn set_user_total(&self, total: u64) {
        self.inner.users_pb.set_length(total);
    }

    pub fn inc_user(&self) {
        self.inner.users_pb.inc(1);
    }

    pub fn set_file_total(&self, total: u64) {
        self.inner.files_pb.set_length(total);
    }

    pub fn inc_file(&self, bytes: u64) {
        self.inner.files_pb.inc(1);
        self.inner.files_pb.inc(bytes);
    }

    pub fn set_share_total(&self, total: u64) {
        self.inner.shares_pb.set_length(total);
    }

    pub fn inc_share(&self) {
        self.inner.shares_pb.inc(1);
    }

    pub fn set_tag_total(&self, total: u64) {
        self.inner.tags_pb.set_length(total);
    }

    pub fn inc_tag(&self) {
        self.inner.tags_pb.inc(1);
    }

    pub fn set_favorite_total(&self, total: u64) {
        self.inner.favorites_pb.set_length(total);
    }

    pub fn inc_favorite(&self) {
        self.inner.favorites_pb.inc(1);
    }

    pub fn finish(&self) {
        self.inner.multi.clear().ok();
    }

    pub fn elapsed(&self) -> Duration {
        self.inner.start.elapsed()
    }
}
