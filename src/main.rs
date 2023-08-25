//! ```text
//! $ expac "%n %m" -l'\n' linux firefox llvm dmenu file jq base nix | numfmt --to iec --format "%f" --field=2 --padding=1 | hl -f1:size
//! linux 126M
//! firefox 221M
//! llvm 95M
//! dmenu 52K
//! file 8.4M
//! jq 691K
//! base 0
//! nix 11M
//!
//! $ expac "%n %m" -l'\n' linux firefox llvm dmenu file jq base nix | hl -f1:size         
//! linux 131672735
//! firefox 231061827
//! llvm 99457316
//! dmenu 52591
//! file 8739440
//! jq 706775
//! base 0
//! nix 10773875
//! ```
//! # Skip parameter (-s, --skip)
//! Skip to a substring and match fields after it
//!
//! Highlighted portions are marked with `(` and `)`
//!
//! NOTE: /proc/cpuinfo uses a tab before the `:` character
//! ```text
//! $ hl -f1:red < /proc/cpuinfo | head -n3
//! processor   : (0)
//! vendor_id   : (GenuineIntel)
//! cpu (family  :) 6
//!
//! $ hl -s': ' -f0:red < /proc/cpuinfo | head -n3
//! processor   : (0)
//! vendor_id   : (GenuineIntel)
//! cpu family  : (6)
//! ```

use bpaf::Bpaf;
#[cfg(feature = "size-color")]
use bytesize::ByteSize;
use core::fmt;
use std::fmt::{Display, Formatter, Write as _};
use std::io::Write as _;
use std::num::ParseIntError;
use std::str::FromStr;

#[derive(Debug, Clone, Bpaf)]
pub enum Color {
    Ansi(String),
    #[cfg(feature = "size-color")]
    Size
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Color::Ansi(ansi) => f.write_str(ansi),
            Color::Size => Err(fmt::Error),
        }
    }
}
impl FromStr for Color {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        //let mut f = Cursor::new(String::new());
        let mut f = String::new();
        f.write_str("\x1B[3")?; // change 3 to 4 for background and add 6 for bright colors
        match input {
            "default" => write!(f, "9")?,
            "black" => write!(f, "0")?,
            "red" => write!(f, "1")?,
            "green" => write!(f, "2")?,
            "yellow" => write!(f, "3")?,
            "blue" => write!(f, "4")?,
            "magenta" => write!(f, "5")?,
            "cyan" => write!(f, "6")?,
            "white" => write!(f, "7")?,
            input if input.starts_with("fixed(") && input.ends_with(')') => {
                let in_par = input.strip_prefix("fixed(").unwrap().strip_suffix(')').unwrap();
                let num: usize = in_par.parse()?;
                write!(f, "8;5;{}", num)?;
            },
            input if input.starts_with("rgb(") && input.matches(',').count() == 2 && input.ends_with(')') => {
                let in_par = input.strip_prefix("rgb(").unwrap().strip_suffix(')').unwrap();
                use itertools::Itertools;
                let (red, green, blue) = in_par.splitn(3, ',').collect_tuple().unwrap();
                let (red, green, blue): (usize, usize, usize) = (red.parse()?, green.parse()?, blue.parse()?);
                write!(f, "8;2;{};{};{}", red, green, blue)?;
            },
            #[cfg(feature = "size-color")]
            "size" => return Ok(Color::Size),
            input => return Err(ParseError::UnknownColor(input.to_owned())),
        };
        f.write_str("m")?;
        Ok(Color::Ansi(f))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("unknown color {0}")]
    UnknownColor(String),
    #[error("missing : between field and color")]
    MissingColonField,
    #[error(transparent)]
    IntParseError(#[from] ParseIntError),
    #[error("ANSI fmt error: {0}")]
    AnsiFmtError(#[from] fmt::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error("unknown error")]
    Unknown,
}

#[derive(Debug, Clone)]
struct FieldColor {
    field: isize,
    color: Color,
}
impl FromStr for FieldColor {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let (field, color) = input.split_once(':').ok_or(ParseError::MissingColonField)?;
        Ok(Self {
            field: field.parse()?,
            color: color.parse()?,
        })
    }
}

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Options {
    #[bpaf(short, long("field"), argument("FIELD:COLOR"))]
    /// Color fields
    fields: Vec<FieldColor>,
    #[bpaf(short, long, fallback(" ".to_owned()), debug_fallback)]
    /// Custom delimeter for fields
    delimeter: String,
    #[bpaf(short, long)]
    /// Skip to a substring and match fields after it
    skip: Option<String>,
    #[cfg(feature = "size-color")]
    #[bpaf(long, fallback(ByteSize::mb(20)), display_fallback)]
    /// For the "size" color
    yellow_size: ByteSize,
    #[cfg(feature = "size-color")]
    #[bpaf(long, fallback(ByteSize::mb(100)), display_fallback)]
    /// For the "size" color
    red_size: ByteSize,
}

//fn main() -> Result<(), anyhow::Error> {
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    reset_sigpipe();
    //Err(anyhow::anyhow!("foo").context("bar"))?;
    let options = options().run();

    let default_color = Color::from_str("default")?;
    let green_color = Color::from_str("green")?;
    let yellow_color = Color::from_str("yellow")?;
    let red_color = Color::from_str("red")?;

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    loop {
        let mut buf = String::new();
        let n = stdin.read_line(&mut buf)?;
        if n == 0 {
            // EOF, https://doc.rust-lang.org/std/io/trait.BufRead.html#method.read_line
            break Ok(());
        }

        // Implement skip
        let buf = if let Some(ref pat) = options.skip {
            let (left, right) = buf.split_once(pat).ok_or_else(||anyhow::anyhow!("skip not found"))?;
            stdout.write_all(left.as_bytes())?;
            stdout.write_all(pat.as_bytes())?;
            right.to_owned()
        } else {
            buf
        };

        let split: Vec<_> = buf.split_inclusive(&options.delimeter).collect();
        // TODO: negative indexes for fields
        for (i, text) in split.iter().enumerate() {
            if let Some(fieldcolor) = options.fields.iter().find(|fc| fc.field >= 0 && fc.field as usize == i) {
                match &fieldcolor.color {
                    Color::Ansi(ansi) => {
                        write!(stdout, "{}{}{}", ansi, text, default_color)?;
                    },
                    #[cfg(feature = "size-color")]
                    Color::Size => {
                        let size: ByteSize = text.trim().parse()?;
                        let color = if size > options.red_size {
                            &red_color
                        } else if size > options.yellow_size {
                            &yellow_color
                        } else {
                            &green_color
                        };
                        write!(stdout, "{}{}{}", color, text, default_color)?;
                    }
                }
            } else {
                stdout.write_all(text.as_bytes())?;
            }
        }
    }
}

#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}
