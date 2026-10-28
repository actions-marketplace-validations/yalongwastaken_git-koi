use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};

use chrono::{DateTime, NaiveDate};
use git_koi::collect::{GhCli, collect_account_snapshot};

const USAGE: &str = "Usage: git-koi collect LOGIN [--output FILE]
Collect LOGIN's public GitHub snapshot as JSON through `gh api`: public owned and
contributed repositories, and daily contributions from the account's creation
through yesterday (UTC), the last complete day.
Writes to standard output, or replaces FILE only after collection succeeds.
Requires an authenticated GitHub CLI; in GitHub Actions, set GH_TOKEN to github.token.
Errors go to stderr and exit 1; invalid usage exits 2.
Example: git-koi collect octocat --output snapshot.json";

fn main() -> ExitCode {
    let program_arguments: Vec<OsString> = env::args_os().skip(1).collect();
    let argument_texts: Vec<Option<&str>> = program_arguments
        .iter()
        .map(|argument| argument.to_str())
        .collect();
    match argument_texts.as_slice() {
        [Some("--help")] | [Some("collect"), Some("--help")] => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [Some("collect"), Some(login)] if !login.starts_with('-') => run_collect(login, None),
        [Some("collect"), Some(login), Some("--output"), _] if !login.starts_with('-') => {
            run_collect(login, Some(Path::new(&program_arguments[3])))
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run_collect(login: &str, output_path: Option<&Path>) -> ExitCode {
    let collection_result = collect_account_snapshot(&GhCli, login, current_utc_date())
        .map_err(|collection_error| collection_error.to_string())
        .and_then(|account_snapshot| {
            serde_json::to_string_pretty(&account_snapshot)
                .map_err(|serialize_error| serialize_error.to_string())
        })
        .and_then(|snapshot_json| write_snapshot(&format!("{snapshot_json}\n"), output_path));
    match collection_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error_message) => {
            eprintln!("git-koi: {error_message}");
            ExitCode::FAILURE
        }
    }
}

// GitHub dates its contribution calendar in UTC.
fn current_utc_date() -> NaiveDate {
    let seconds_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the system clock is after 1970")
        .as_secs();
    DateTime::from_timestamp(
        i64::try_from(seconds_since_epoch).expect("the system clock is in range"),
        0,
    )
    .expect("the system clock is in range")
    .date_naive()
}

/// Write to standard output, or replace the file through a sibling temporary file so a
/// failed write never leaves a partial snapshot in place.
fn write_snapshot(snapshot_json: &str, output_path: Option<&Path>) -> Result<(), String> {
    let Some(output_path) = output_path else {
        return io::stdout()
            .write_all(snapshot_json.as_bytes())
            .map_err(|write_error| format!("standard output: {write_error}"));
    };
    let temporary_path = output_path.with_extension("tmp");
    fs::write(&temporary_path, snapshot_json)
        .and_then(|()| fs::rename(&temporary_path, output_path))
        .map_err(|write_error| {
            let _ = fs::remove_file(&temporary_path);
            format!("{}: {write_error}", output_path.display())
        })
}
