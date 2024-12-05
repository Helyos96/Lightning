pub mod left;
pub mod top;
pub mod config;
pub mod skills;
pub mod items;

use lightning_model::build::GemLink;

fn text_gemlink(gemlink: &GemLink) -> String {
	if gemlink.active_gems().count() == 0 {
        return String::from("<No Active Skill>");
    }
    let mut ret = String::new();
    for active_gem in gemlink.active_gems() {
        ret += &active_gem.data().base_item.display_name;
        ret += ", ";
    }
    return String::from(ret.trim_end_matches(", "));
}

fn text_gemlink_cutoff(gemlink: &GemLink, mut cutoff: usize) -> String {
	cutoff = cutoff.max(3);
    let mut ret = text_gemlink(gemlink);
    if ret.len() > cutoff {
        ret = String::from(&ret[0..cutoff-3]) + "...";
    }
    return ret;
}
