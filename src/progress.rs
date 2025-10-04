//! Progress tracking and reporting

use indicatif::{ProgressBar, ProgressStyle};

pub struct ProgressTracker {
    progress_bar: ProgressBar,
    files_copied: u64,
    bytes_copied: u64,
}

impl ProgressTracker {
    pub fn new() -> Self {
        let pb = ProgressBar::new(0);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );

        Self {
            progress_bar: pb,
            files_copied: 0,
            bytes_copied: 0,
        }
    }

    pub fn set_total(&self, total_bytes: u64) {
        self.progress_bar.set_length(total_bytes);
    }

    pub fn update(&mut self, bytes: u64) {
        self.bytes_copied += bytes;
        self.files_copied += 1;
        self.progress_bar.inc(bytes);
    }

    pub fn finish(&self) {
        self.progress_bar.finish_with_message("Copy completed");
    }
}
