use std::{fs::File, io::{self, BufRead, BufReader, ErrorKind}};
use rustc_hash::FxHashMap;
use regex::{Captures, Regex};
use lightning_model::regex;

/// Parsing for .csd files, usually translation templates

#[derive(Debug)]
pub struct Range {
    min: i64,
    max: i64,
}

#[derive(Debug)]
pub enum Argument {
    SingleValue(i64),
    MinMax(Range),
}

impl Argument {
    pub fn matches(&self, number: i64) -> bool {
        use Argument::*;
        match self {
            SingleValue(i) => {
                *i == number
            }
            MinMax(range) => {
                number >= range.min && number <= range.max
            }
        }
    }
}

#[derive(Debug)]
pub struct Translation {
    args: Vec<Argument>,
    text: String,
}

impl Translation {
    pub fn nb_args(&self) -> usize {
        self.args.len()
    }

    pub fn matches(&self, params: &[i64]) -> bool {
        if params.len() != self.args.len() {
            return false;
        }

        for (arg, param) in self.args.iter().zip(params) {
            if !arg.matches(*param) {
                return false;
            }
        }

        true
    }
}

/// A dictionary of <StatId, [Translations]> which gives you possible translations
/// for a specific StatId depending on parameters' values.
/// StatId is usually of the form 
#[derive(Default, Debug)]
pub struct Translations(pub FxHashMap<String, Vec<Translation>>);

impl Translations {
    /// Attempts to format a stat_id into text depending on parameters.
    /// Will only succeed if a translation matching the parameters (amount and values) is found
    pub fn format(&self, stat_id: &str, params: &[i64]) -> Option<String> {
        let stat_translations = self.0.get(stat_id)?;
        let translation = stat_translations.iter().find(|t| t.matches(params))?;
        let mut ret = translation.text.clone();

        for (i, param) in params.iter().enumerate() {
            ret = ret.replace(&format!("{{{}}}", i), &param.to_string());
        }

        let regex_square_brackets = regex!("\\[([a-zA-Z ]+)(\\|[a-zA-Z ]+)?\\]");
        ret = regex_square_brackets.replace_all(&ret, |caps: &Captures| {
            if caps.get(2).is_some() {
                format!("{}", &caps[2][1..])
            } else {
                format!("{}", &caps[1])
            }
        }).to_string();

        // TODO: negate 1
        Some(ret)
    }

    /// Attemps to retrieve the amount of arguments for a specific StatId.
    pub fn nb_args(&self, stat_id: &str) -> Option<usize> {
        if let Some(translations) = self.0.get(stat_id) {
            if let Some(translation) = translations.get(0) {
                return Some(translation.nb_args());
            }
        }
        None
    }
}

pub fn parse_arg(txt: &str) -> Option<Argument> {
    if txt == "#" {
        return Some(Argument::MinMax(Range { min: i64::min_value(), max: i64::max_value() }));
    }
    if let Ok(number) = txt.parse::<i64>() {
        return Some(Argument::SingleValue(number));
    }
    let regex_arg = regex!("([0-9#-]+)\\|([0-9#-]+)");
    if let Some(cap) = regex_arg.captures(txt) {
        let min = if &cap[1] == "#" {
            i64::min_value()
        } else {
            cap[1].parse().unwrap()
        };

        let max = if &cap[2] == "#" {
            i64::max_value()
        } else {
            cap[2].parse().unwrap()
        };

        return Some(Argument::MinMax(Range { min, max }));
    }
    None
}

pub fn parse_args(txt: &str) -> Vec<Argument> {
    let mut ret = vec![];
    for arg_txt in txt.split(' ') {
        if let Some(arg) = parse_arg(arg_txt) {
            ret.push(arg);
        }
    }
    ret
}

/// Parses something like
///
/// description
/// 1 darkness_per_level
/// 1
///     # "{0:+d} to Maximum Darkness per Level"
///
/// Does not care about languages other than the first one.
pub fn parse_description(reader: &mut BufReader<File>) -> io::Result<(String, Vec<Translation>)> {
    enum State {
        Description,
        TradCount,
        Trad(usize),
    }
    use State::*;

    let regex_desc = regex!("1 ([a-z_+%-]+)");
    let regex_trad = regex!("((?:[0-9#|-]+ )+)\"([a-zA-Z_+%\\[\\]{}0-9 |\\\\-]+)\"");
    let mut trad_count: usize = 0;
    let mut state = Description;
    let mut trads = vec![];
    let mut stat = String::new();

    loop {
        let mut line = String::new();
        let length = reader.read_line(&mut line)?;
        if length == 0 {
            return Err(io::Error::new(ErrorKind::UnexpectedEof, "EOF"));
        }
        let line = line.trim_start_matches('\t');

        match state {
            Description => {
                if let Some(cap) = regex_desc.captures(line) {
                    stat = cap[1].to_string();
                } else {
                    return Err(io::Error::new(ErrorKind::Other, "Couldn't parse description"));
                }
                state = TradCount;
            },
            TradCount => {
                if let Ok(count) = line.trim().parse::<usize>() {
                    if count == 0 {
                        return Err(io::Error::new(ErrorKind::Other, "Count is 0"));
                    }
                    trad_count = count;
                    state = Trad(0);
                } else {
                    return Err(io::Error::new(ErrorKind::Other, "Couldn't parse count"));
                }
            },
            Trad(i) => {
                if let Some(cap) = regex_trad.captures(line) {
                    let args = parse_args(&cap[1]);
                    if args.len() > 0 {
                        trads.push(Translation { args: args, text: cap[2].to_string() });
                    }
                } else {
                    return Err(io::Error::new(ErrorKind::Other, "Couldn't parse trad"));
                }
                if i == trad_count - 1 {
                    return Ok((stat, trads));
                }
                state = Trad(i + 1);
            },
        }
    }
}

/// Parses a csd file.
/// /!\ Assumes UTF-8 encoding, you will most likely need to convert as game files are almost always UTF-16le.
pub fn parse_csd(name: &str) -> io::Result<Translations> {
    let file = File::open(name)?;
    let mut reader = BufReader::new(file);
    let mut ret = Translations::default();

    loop {
        let mut line = String::new();
        let length = reader.read_line(&mut line)?;

        if length == 0 {
            return Ok(ret);
        }

        let trimmed = line.trim();
        if trimmed == "description" {
            if let Ok((stat, trads)) = parse_description(&mut reader) {
                ret.0.insert(stat, trads);
            }
        }
    }
}
