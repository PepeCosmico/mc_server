use std::time::Duration;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::error::Result;

pub async fn run_task<F, T>(
    loading_msg: &str,
    finish_msg: &str,
    finish_error_msg: &str,
    task: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let pb = ProgressBar::new_spinner();

    pb.set_style(
        ProgressStyle::default_bar()
            .tick_chars("McServer")
            .template("[ {spinner} ] ~> {msg}")?,
    );
    pb.enable_steady_tick(Duration::from_millis(120));
    pb.set_message(loading_msg.to_string());

    let result = task.await;

    let final_style = ProgressStyle::default_bar().template("{prefix} ~> {msg}")?;
    match &result {
        Ok(_) => {
            pb.set_style(final_style);
            let check = format!("[ {} ]", "M".bold().green());
            pb.set_prefix(check);
            pb.finish_with_message(finish_msg.to_string());
        }
        Err(_) => {
            pb.set_style(final_style);
            let error = format!("[ {} ]", "M".bold().green());
            pb.set_prefix(error);
            pb.finish_with_message(finish_error_msg.to_string());
        }
    }

    result
}
