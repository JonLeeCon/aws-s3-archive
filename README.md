# aws-s3-archive
Project for backing up and archiving s3 buckets

## Usage
```
aws-s3-archive 0.1
Jonathan Constantinides <jon@joncon.io>
Download and archive files for s3

USAGE:
    aws-s3-archive [FLAGS] [OPTIONS] --import <IMPORT_FILE>

FLAGS:
    -d               Verify and delete from S3
    -h, --help       Prints help information
    -V, --version    Prints version information
    -v               Verify only

OPTIONS:
    -b, --backup <LOCATION>       Location to backup bucket to [default: *current_directory*/backup]
    -i, --import <IMPORT_FILE>    List of files to archive (i.e. s3://BUCKET/PREFIX)
    -r, --region <REGION>         AWS region (us-west-2)
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