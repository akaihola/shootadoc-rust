use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "shootadoc",
    about = "Transform photos of documents to improve readability"
)]
pub struct Arguments {
    #[structopt(parse(from_os_str))]
    pub in_file_path: Vec<PathBuf>,
    #[structopt(short, long)]
    pub debug: bool,
}
pub fn parse_args() -> Arguments {
    Arguments::from_args()
}

pub fn get_out_fname(p: &Path) -> PathBuf {
    let mut out = p.to_owned();
    let mut out_fname = out.file_stem().unwrap_or_default().to_owned();
    out_fname.push(".fixed.");
    let e = out
        .extension()
        .expect("Cannot convert files without extension");
    out_fname.push(e);
    out.set_file_name(out_fname);
    out
}
