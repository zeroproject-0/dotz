use std::collections::VecDeque;
use std::env;
use std::fs;
use std::fs::ReadDir;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

fn main() {
	let mut args: VecDeque<String> = env::args().collect();
	args.pop_front();

	if args.len() == 0 {
		show_help();
		return;
	}

	if args.contains(&String::from("-h")) || args.contains(&String::from("--help")) {
		show_help();
		return;
	}

	let first = args.pop_front();

	let force = first.as_ref().unwrap() == "-f" || first.as_ref().unwrap() == "--force";

	let home = match env::var_os("HOME") {
		Some(home) => home,
		None => {
			println!("Could not find $HOME");
			return;
		}
	};

	let mut path: PathBuf = PathBuf::from(first.unwrap())
		.canonicalize()
		.unwrap_or_default();
	let mut destination = PathBuf::from(home.as_os_str());

	if args.len() == 1 && !force {
		destination = match Path::new(args.pop_front().unwrap().as_str()).canonicalize() {
			Ok(path) => path,
			Err(_) => {
				show_help();
				return;
			}
		};
	} else if args.len() == 1 || args.len() == 2 {
		path = PathBuf::from(args.pop_front().unwrap())
			.canonicalize()
			.unwrap();

		if args.len() == 2 {
			destination = match Path::new(args.pop_front().unwrap().as_str()).canonicalize() {
				Ok(path) => path,
				Err(_) => {
					show_help();
					return;
				}
			};
		}
	}

	if args.len() > 0 {
		show_help();
		return;
	}

	match fs::read_dir(path) {
		Ok(files) => create_symlinks(files, &destination, force),
		Err(_) => {
			show_help();
			return;
		}
	};
}

fn create_symlinks(files: ReadDir, destination: &Path, force: bool) {
	for file in files {
		let file = file.unwrap();
		let file_path = file.path();
		let file_name = file_path.file_name().unwrap();

		if file_name == ".git" || file_name == ".gitignore" {
			continue;
		}

		if file.file_type().unwrap().is_dir() {
			let files = fs::read_dir(&file_path).unwrap();
			let dest = destination.clone().join(file_name);

			match fs::create_dir(&dest) {
				Ok(_) => {
					println!("Created directory {}", dest.display());
				}
				Err(_) => {}
			};

			create_symlinks(files, &dest, force);
			continue;
		}

		let dest = Path::new(&destination).join(file_name);

		println!("Linking {} to {}", file_path.display(), dest.display());

		match symlink(&file_path, &dest) {
			Ok(_) => (),
			Err(e) => {
				if force {
					match fs::remove_file(&dest) {
						Ok(_) => (),
						Err(e) => {
							println!("Failed to remove existing file {}: {}", dest.display(), e);
							continue;
						}
					};

					symlink(&file_path, &dest).unwrap();
				} else {
					println!(
						"Failed to link {} to {}: {}",
						file_path.display(),
						dest.display(),
						e
					);
					continue;
				}
			}
		};
	}
}

fn show_help() {
	println!("dotz - A simple dotfile manager");
	println!("");
	println!("Options:");
	println!("");
	println!("-h, --help\tShow this help message");
	println!("-f, --force\tForce overwrite of existing files");
	println!("");
	println!("");
	println!("Usage: dotz [options] [path] [destination]");
	println!("");
	println!("");
	println!("[path] is the directory where the dotfiles are located");
	println!("[destination] is the directory where the dotfiles will be linked to (optional defaults to $HOME)");
}
