## random_pairs_of_s3file Usage:

Usage: random_pairs_of_s3file [OPTIONS] --num-pairs <NUM_PAIRS> --bucket <BUCKET> --directory <DIRECTORY> --url-prefix <URL_PREFIX>

Options:
      --num-pairs <NUM_PAIRS>        Number of pairs to generate
      --bucket <BUCKET>              Name of the S3 bucket
      --directory <DIRECTORY>        Directory (prefix) in the bucket (e.g. "image/")
      --url-prefix <URL_PREFIX>      URL prefix to form the final URL (e.g. "https://api.example.com/s3/api/v1/resource?url=s3://")
      --exclude-file <EXCLUDE_FILE>  File containing keys to exclude
  -h, --help                         Print help
  -V, --version                      Print version
