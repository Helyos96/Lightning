use std::{fs::File, io::{self, BufRead, BufReader, ErrorKind}};
use rustc_hash::FxHashMap;
use regex::{Captures, Regex};
use lightning_model::regex;
use encoding_rs::UTF_16LE;
use encoding_rs_io::DecodeReaderBytesBuilder;
use lazy_static::lazy_static;

/// Parsing for .csd files, usually translation templates

#[derive(Debug, Clone)]
enum Argument {
    SingleValue(i64),
    MinMax(i64, i64),
}

impl Argument {
    pub fn matches(&self, number: i64) -> bool {
        use Argument::*;
        match self {
            SingleValue(i) => *i == number,
            MinMax(min, max) => number >= *min && number <= *max,
        }
    }
}

#[derive(Debug, Clone)]
enum Mutation {
    DivideBy0dp(i64),
    DivideBy1dp(i64),
    DivideBy2dp(i64),
    DivideByThenTimes0dp(i64, i64),
    Negate,
    NegateAndDouble,
    Plus(i64),
    Times(i64),
    TimesFloat(f32),
}

lazy_static! {
    static ref REGEX_SQUARE_BRACKETS: Regex = regex!("\\[([a-zA-Z ]+)(\\|[a-zA-Z ]+)?\\]");
    static ref REGEX_ARG: Regex = regex!("([0-9#-]+)\\|([0-9#-]+)");
    static ref REGEX_TRAD: Regex = regex!("((?:[0-9#|-]+ )+)\"(.+?)\" ?(.+)?");
    static ref REGEX_DESC: Regex = regex!("[0-9]+ ([a-zA-Z0-9_+% -]+)");
}

impl Mutation {
    fn apply_to(&self, number: i64) -> String {
        use Mutation::*;
        let mut ret = match self {
            DivideBy0dp(i) => format!("{}", number / i),
            DivideBy1dp(i) => format!("{:.1}", number as f32 / *i as f32),
            DivideBy2dp(i) => format!("{:.2}", number as f32 / *i as f32),
            DivideByThenTimes0dp(d, t) => format!("{}", (number / d) * t),
            Negate => format!("{}", -number),
            NegateAndDouble => format!("{}", number * -2),
            Plus(i) => format!("{}", number + i),
            Times(i) => format!("{}", number * i),
            TimesFloat(f) => format!("{:.1}", number as f32 * f),
        };
        // Sadly rust's format!() doesn't have an equivalent to "%g"
        // and there are no crates that provide this in a simple way.
        // Just remove trailing zeroes for floats, then trailing '.'
        if ret.contains('.') {
            ret = ret.trim_end_matches('0').trim_end_matches('.').to_string();
        }
        ret
    }

    fn from_str(text: &str) -> Option<Mutation> {
        use Mutation::*;
        match text {
            "30%_of_value" => Some(TimesFloat(0.3)),
            "divide_by_two_0dp" => Some(DivideBy0dp(2)),
            "divide_by_three" => Some(DivideBy1dp(3)),
            "divide_by_four" => Some(DivideBy1dp(4)),
            "divide_by_five" => Some(DivideBy1dp(5)),
            "divide_by_one_hundred" => Some(DivideBy1dp(100)),
            "divide_by_one_hundred_2dp" => Some(DivideBy2dp(100)),
            "divide_by_one_hundred_2dp_if_required" => Some(DivideBy2dp(100)),
            "divide_by_ten_0dp" => Some(DivideBy0dp(10)),
            "divide_by_ten_1dp" => Some(DivideBy1dp(10)),
            "divide_by_ten_1dp_if_required" => Some(DivideBy1dp(10)),
            "divide_by_fifteen_0dp" => Some(DivideBy0dp(15)),
            "divide_by_twenty_then_double_0dp" => Some(DivideByThenTimes0dp(20, 2)),
            "divide_by_fifty" => Some(DivideBy1dp(50)),
            "double" => Some(Times(2)),
            "milliseconds_to_seconds" => Some(DivideBy1dp(1000)),
            "milliseconds_to_seconds_0dp" => Some(DivideBy0dp(1000)),
            "milliseconds_to_seconds_1dp" => Some(DivideBy1dp(1000)),
            "milliseconds_to_seconds_2dp" => Some(DivideBy2dp(1000)),
            "milliseconds_to_seconds_2dp_if_required" => Some(DivideBy2dp(1000)),
            "negate" => Some(Negate),
            "negate_and_double" => Some(NegateAndDouble),
            "per_minute_to_per_second" => Some(DivideBy1dp(60)),
            "per_minute_to_per_second_0dp" => Some(DivideBy0dp(60)),
            "per_minute_to_per_second_1dp" => Some(DivideBy1dp(60)),
            "per_minute_to_per_second_2dp" => Some(DivideBy2dp(60)),
            "per_minute_to_per_second_2dp_if_required" => Some(DivideBy2dp(60)),
            "plus_two_hundred" => Some(Plus(200)),
            "times_one_point_five" => Some(TimesFloat(1.5)),
            "times_twenty" => Some(Times(20)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Translation {
    args: Vec<Argument>,
    mutations: FxHashMap<usize, Mutation>,
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

        for (i, param) in params.iter().copied().enumerate() {
            let param = if let Some(mutation) = translation.mutations.get(&i) {
                mutation.apply_to(param)
            } else {
                format!("{}", param)
            };

            if i == 0 {
                ret = ret.replace("{}", &param);
            }
            ret = ret.replace(&format!("{{{}}}", i), &param);
            ret = ret.replace(&format!("{{{}:d}}", i), &param);
            ret = ret.replace(&format!("{{{}:+d}}", i), &format!("+{}", &param));
        }

        // Takes care of stuff like "[HitDamage|Hits]"
        ret = REGEX_SQUARE_BRACKETS.replace_all(&ret, |caps: &Captures| {
            if caps.get(2).is_some() {
                caps[2][1..].to_string()
            } else {
                caps[1].to_string()
            }
        }).to_string();

        Some(ret)
    }

    /// Attemps to retrieve the amount of arguments for a specific StatId.
    pub fn nb_args(&self, stat_id: &str) -> Option<usize> {
        if let Some(translations) = self.0.get(stat_id) {
            if let Some(translation) = translations.first() {
                return Some(translation.nb_args());
            }
        }
        None
    }
}

fn parse_mutations(txt: &str) -> FxHashMap<usize, Mutation> {
    let mut ret = FxHashMap::default();

    let mut cur_mutation = None;
    for m in txt.split(' ') {
        if let Some(mutation) = Mutation::from_str(m) {
            cur_mutation = Some(mutation);
        } else if let Ok(idx) = m.parse::<usize>() {
            if let Some(cur_mut) = cur_mutation {
                ret.insert(idx - 1, cur_mut);
                cur_mutation = None;
            }
        } else {
            //println!("failed: {m}");
            cur_mutation = None;
        }
    }

    ret
}

fn parse_arg(txt: &str) -> Option<Argument> {
    if txt == "#" {
        return Some(Argument::MinMax(i64::MIN, i64::MAX));
    }
    if let Ok(number) = txt.parse::<i64>() {
        return Some(Argument::SingleValue(number));
    }
    if let Some(cap) = REGEX_ARG.captures(txt) {
        let min = if &cap[1] == "#" {
            i64::MIN
        } else {
            cap[1].parse().unwrap()
        };

        let max = if &cap[2] == "#" {
            i64::MAX
        } else {
            cap[2].parse().unwrap()
        };

        return Some(Argument::MinMax(min, max));
    }
    None
}

fn parse_args(txt: &str) -> Vec<Argument> {
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
pub fn parse_description<R: BufRead>(reader: &mut R) -> io::Result<Vec<Translation>> {
    enum State {
        TradCount,
        Trad(usize),
    }
    use State::*;

    let mut trad_count: usize = 0;
    let mut state = TradCount;
    let mut trads = vec![];

    loop {
        let mut line = String::new();
        let length = reader.read_line(&mut line)?;
        if length == 0 {
            return Err(io::Error::new(ErrorKind::UnexpectedEof, "EOF"));
        }
        let line = line.trim().trim_start_matches('\t');

        match state {
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
                if let Some(cap) = REGEX_TRAD.captures(line) {
                    let args = parse_args(&cap[1]);
                    if !args.is_empty() {
                        let mutations = if cap.get(3).is_some() {
                            parse_mutations(&cap[3])
                        } else {
                            FxHashMap::default()
                        };
                        trads.push(Translation { args, text: cap[2].to_string(), mutations });
                    }
                } else {
                    return Err(io::Error::new(ErrorKind::Other, "Couldn't parse trad"));
                }
                if i == trad_count - 1 {
                    return Ok(trads);
                }
                state = Trad(i + 1);
            },
        }
    }
}

/// Parses a csd file.
pub fn parse_csd(name: &str) -> io::Result<Translations> {
    let file = File::open(name)?;
    let transcoded_reader = DecodeReaderBytesBuilder::new()
        .encoding(Some(UTF_16LE))
        .build(file);
    let mut utf8_reader = BufReader::new(transcoded_reader);
    let mut ret = Translations::default();

    loop {
        let mut line = String::new();
        let length = utf8_reader.read_line(&mut line)?;
        if length == 0 {
            return Ok(ret);
        }

        let trimmed = line.trim();
        if trimmed == "description" {
            let mut line = String::new();
            let length = utf8_reader.read_line(&mut line)?;
            if length == 0 {
                return Ok(ret);
            }
            if let Some(cap) = REGEX_DESC.captures(&line) {
                if let Ok(trads) = parse_description(&mut utf8_reader) {
                    for stat in cap[1].split(' ') {
                        ret.0.insert(stat.to_string(), trads.clone());
                    }
                }
            } else {
                return Err(io::Error::new(ErrorKind::Other, "Couldn't parse description"));
            }
        }
    }
}
