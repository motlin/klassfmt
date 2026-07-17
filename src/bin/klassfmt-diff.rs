//! Dev helper: print a unified diff between a file and its formatted output.
use std::fs;

fn main() {
	let path = std::env::args()
		.nth(1)
		.expect("usage: klassfmt-diff <file>");
	let src = fs::read_to_string(&path).expect("read file");
	match klassfmt::format(&src) {
		Ok(formatted) => {
			let diff = similar::TextDiff::from_lines(&src, &formatted);
			for change in diff.iter_all_changes() {
				let sign = match change.tag() {
					similar::ChangeTag::Delete => "-",
					similar::ChangeTag::Insert => "+",
					similar::ChangeTag::Equal => " ",
				};
				if sign != " " {
					print!("{sign}{change}");
				}
			}
		}
		Err(e) => eprintln!("format error: {e}"),
	}
}
