# aws-s3-archive
Project for backing up and archiving s3 buckets

## Usage
```
aws-s3-archive 0.1
Jonathan Constantinides <jon@joncon.io>
Download and archive files for s3

USAGE:
    aws-s3-archive [OPTIONS] --import <IMPORT_FILE>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -b, --backup <LOCATION>       Location to backup bucket to [default: *current_directory*/backup]
    -i, --import <IMPORT_FILE>    List of files to archive (i.e. s3://BUCKET/PREFIX)
```
## Getting Started
### Prerequisites
- A Cargo/Rust setup for compiling
- AWS CLI installed

### Setting Up
- Currently only supports an import file with each line formatted as `s3://bucket/object`

## Authors
Jonathan Constantinides <jon@joncon.io>

## License
This project is licensed under the MIT License