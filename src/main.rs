#![deny(//missing_docs,
        missing_debug_implementations, missing_copy_implementations,
        trivial_casts, trivial_numeric_casts,
        unsafe_code,
        unstable_features,
        unused_import_braces, unused_qualifications)]

/* === CRATES === */
extern crate clap;
extern crate failure;
#[macro_use]
extern crate failure_derive;
extern crate rayon;
extern crate rusoto_core;
extern crate rusoto_s3;

/* === MODs === */

/* === USE === */
use std::env::current_dir;
use std::fs::{create_dir, File};
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};

use clap::{App, Arg};
use failure::Error;
use rusoto_core::Region;
use rusoto_s3::{HeadObjectRequest, S3, S3Client};

/* === TYPES === */
type Result<T> = std::result::Result<T, Error>;

/* === CONSTANTS === */

/* === STRUCTS === */
// #[derive(Fail, Debug)]
// #[fail(display = "Error: Invalid String {}", _0)]
// struct ReadLineError(String);

/* === ENUMS === */
// #[derive(Fail, Debug)]
// enum LineError {
//   #[fail(display = "Error: Invalid String {}", _0)]
//   ReadLineError(String),

//   #[fail(display = "{}", _0)]
//   Io(#[cause] std::io::Error),
// }

/* === FUNCTIONS ===*/

// fn get_info_from_line(&line) {

// }

fn check_and_create_directory(dir_path_in: &str) -> Result<()> {
  let dir_path = Path::new(dir_path_in);
  if !dir_path.exists() {
    create_dir(dir_path)?;
  }
  Ok(())
}

// fn get_object_md5() {

// }

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

fn run() -> Result<()> {
  let current_dir = current_dir()?;
  let defaultPath = Path::new("backup").to_str();

  // Init arguments
  let matches = App::new("aws-s3-archive")
    .version("0.1")
    .about(
      "Download and archive files for s3",
    )
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
        .default_value("./backup")
        // .required(true)
        .short("b")
        .long("backup")
        .value_name("LOCATION")
        .help("Location to backup bucket to"),
    )
    // .arg(
    //   Arg::with_name("dir")
    //     .short("d")
    //     .long("directory")
    //     .value_name("DIRECTORY")
    //     .help("Directory/prefix for objects"),
    // )
    .get_matches();

  let backup = matches.value_of("backup").unwrap();

  check_and_create_directory(&backup)?;

  let s3 = S3Client::new(Region::UsWest2);

  let f = File::open(matches.value_of("import").unwrap())?;
  let f = BufReader::new(f);

  // let head_req = HeadObjectRequest {};
  // test.e_tag

  let read_lines = f.lines();
  let filter: Vec<(String, String)> = read_lines
    .filter_map(|read_line| match read_line {
      Ok(line) => Some(line),
      Err(err) => {
        eprintln!("Invalid line: {:?}", err);
        None
      }
    })
    .filter_map(|line| {
      if line.len() > 5 {
        let (protocol, location) = line.split_at(5);
        if protocol == "s3://" {
          let mut paths: Vec<&str> = location.split("/").collect();
          paths.reverse();
          let bucket = paths.pop().unwrap();
          paths.reverse();
          let key = paths.join("/");
          return Some((bucket.to_owned(), key.to_owned()));
        }
      }
      eprintln!("Invalid line: {}", line);
      None
    })
    .collect();

    // s3.head_object(HeadObjectRequest {
    //       bucket: bucket.to_string(),
    //       key: line,
    //       ..Default::default()
    //     }).sync()

  Ok(())
}
