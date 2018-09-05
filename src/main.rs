#![deny(//missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications)]

/* === CRATES === */
extern crate clap;
extern crate failure;
extern crate futures;
extern crate rayon;
extern crate rusoto_core;
extern crate rusoto_s3;

/* === MODs === */

/* === USE === */
use std::env::current_dir;
use std::fs::{create_dir, metadata, DirBuilder, File};
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use clap::{App, Arg};
use failure::Error;
use futures::stream::Stream;
use futures::Future;
use rayon::prelude::*;
use rusoto_core::Region;
use rusoto_s3::{
  GetObjectError, GetObjectOutput, GetObjectRequest, HeadObjectError, HeadObjectRequest, S3,
  S3Client,
};

/* === CONSTANTS === */
const RETRY_ATTEMPTS: u32 = 3;
const RETRY_WAIT_SECONDS: u64 = 3;

/* === FUNCTIONS ===*/

fn result_to_option_print<T, E: std::fmt::Display>(res_in: Result<T, E>) -> Option<T> {
  match res_in {
    Ok(res) => Some(res),
    Err(err) => {
      eprintln!("Error {}", err);
      None
    }
  }
}

fn line_to_tuple(line: &str) -> Result<(&str, String, String, String), String> {
  if line.len() <= 5 {
    return Err(format!("Invalid line: {}", line));
  }
  let (protocol, location) = line.split_at(5);
  if protocol != "s3://" {
    return Err(format!("Invalid line: {}", line));
  }
  let mut paths: Vec<&str> = location.split('/').collect();
  let file_name = paths.pop().unwrap();
  paths.reverse();
  let bucket = paths.pop().unwrap();
  paths.reverse();
  let file_path = paths.join("/");
  Ok((line, bucket.to_owned(), file_path, file_name.to_owned()))
}

fn check_and_create_directory(dir_path_in: &str) -> Result<(), std::io::Error> {
  let dir_path = Path::new(dir_path_in);
  if dir_path.exists() {
    Ok(())
  } else {
    create_dir(dir_path)
  }
}

fn get_local_locations(
  backup: &str,
  bucket: &str,
  file_path: &str,
  file_name: &str,
) -> (PathBuf, PathBuf, PathBuf) {
  let mut local_dir = PathBuf::new();
  local_dir.push(backup);
  local_dir.push(bucket);
  local_dir.push(file_path);

  let mut local_file = PathBuf::new();
  local_file.push(&local_dir);
  local_file.push(&file_name);

  let mut key = PathBuf::new();
  key.push(file_path);
  key.push(file_name);

  (local_dir, local_file, key)
}

fn get_default_backup_path() -> Result<String, std::io::Error> {
  let current_dir = current_dir()?;
  let mut default_path = PathBuf::new();
  default_path.push(current_dir);
  default_path.push("backup");
  Ok(default_path.to_string_lossy().to_string())
}

fn main() {
  if let Err(e) = run() {
    use std::io::Write;
    let stderr = &mut ::std::io::stderr();
    let errmsg = "Error writing to stderr";

    writeln!(stderr, "error: {}", e.to_string()).unwrap_or_else(|_| panic!(errmsg));

    let backtrace = e.backtrace().to_string();
    if !backtrace.trim().is_empty() {
      writeln!(stderr, "backtrace: {}", backtrace).unwrap_or_else(|_| panic!(errmsg));
    }

    ::std::process::exit(1);
  }
}

fn run() -> Result<(), Error> {
  let default_path_path = get_default_backup_path()?;

  // Init arguments
  let matches = App::new("aws-s3-archive")
    .version("0.1")
    .about("Download and archive files for s3")
    .author("Jonathan Constantinides <jon@joncon.io>")
    .arg(
      Arg::with_name("import")
        .required(true)
        .short("i")
        .long("import")
        .value_name("IMPORT_FILE")
        .help("List of files to archive (i.e. s3://BUCKET/PREFIX)"),
    )
    .arg(
      Arg::with_name("backup")
        .default_value(&default_path_path)
        .short("b")
        .long("backup")
        .value_name("LOCATION")
        .help("Location to backup bucket to"),
    )
    .get_matches();

  let backup = matches.value_of("backup").unwrap();
  check_and_create_directory(&backup)?;

  let s3 = S3Client::new(Region::UsWest2);

  let f = File::open(matches.value_of("import").unwrap())?;
  let f = BufReader::new(f);

  let filter: Vec<String> = f.lines().filter_map(result_to_option_print).collect();

  filter
    .par_iter()
    .map(|line| line_to_tuple(line))
    .filter_map(result_to_option_print)
    .map(|(complete_path, bucket, file_path, file_name)| {
      let (local_dir, local_file, key) =
        get_local_locations(&backup, &bucket, &file_path, &file_name);

      if let Err(err) = DirBuilder::new().recursive(true).create(&local_dir) {
        return Err((complete_path, format!("{:?}", err)));
      }

      if let Ok(compare_file) = metadata(&local_file) {
        // Try 3 times for network related issues then fail
        let mut attempt = 1;
        loop {
          match (
            attempt,
            s3.head_object(HeadObjectRequest {
              bucket: bucket.to_string(),
              key: key.to_string_lossy().to_string(),
              ..Default::default()
            }).sync(),
          ) {
            (1..=RETRY_ATTEMPTS, Err(HeadObjectError::HttpDispatch(_))) => {
              attempt += 1;
              sleep(Duration::from_secs(RETRY_WAIT_SECONDS));
            }
            (1..=RETRY_ATTEMPTS, Err(HeadObjectError::Unknown(_))) => {
              attempt += 1;
              sleep(Duration::from_secs(RETRY_WAIT_SECONDS));
            }
            (_, Err(err)) => {
              return Err((complete_path, format!("{:?}", err)));
            }
            (_, Ok(metadata)) => {
              let remote_size = metadata.content_length.unwrap();
              // If incorrect size then continue and re-download
              if compare_file.len() as i64 == remote_size {
                return Ok(complete_path);
              }
              break;
            }
          };
        }
      }

      // Try 3 times for network related issues then fail
      let mut attempt = 1;
      loop {
        match (
          attempt,
          s3.get_object(GetObjectRequest {
            bucket: bucket.clone(),
            key: key.to_string_lossy().into(),
            ..Default::default()
          }).sync(),
        ) {
          (1..=RETRY_ATTEMPTS, Err(GetObjectError::HttpDispatch(_))) => {
            attempt += 1;
            sleep(Duration::from_secs(RETRY_WAIT_SECONDS));
          }
          (1..=RETRY_ATTEMPTS, Err(GetObjectError::Unknown(_))) => {
            attempt += 1;
            sleep(Duration::from_secs(RETRY_WAIT_SECONDS));
          }
          (_, Err(err)) => return Err((complete_path, format!("{:?}", err))),
          (
            _,
            Ok(GetObjectOutput {
              content_length: Some(remote_size),
              body: Some(body),
              ..
            }),
          ) => {
            let body = body.concat2().wait().unwrap();

            let mut f = File::create(&local_file).unwrap();
            match f.write(&body) {
              Err(err) => return Err((complete_path, format!("{:?}", err))),
              Ok(local_size) => {
                if local_size as i64 == remote_size {
                  return Ok(complete_path);
                } else {
                  return Err((complete_path, "File sizes do not match".to_string()));
                }
              }
            }
          }
          (1..=RETRY_ATTEMPTS, Ok(_)) => {
            attempt += 1;
            sleep(Duration::from_secs(RETRY_WAIT_SECONDS));
          }
          (_, Ok(_)) => {
            return Err((complete_path, "Could not ".to_string()));
          }
        };
      }
    })
    .for_each(|res| {
      match res {
        Ok(file) => println!("{}", file),
        Err((file, err)) => eprintln!("ERROR {}: {}", file, err),
      };
    });

  Ok(())
}
