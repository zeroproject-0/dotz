use std::collections::VecDeque;
use std::env;
use std::fs;
use std::fs::{copy, remove_dir_all, remove_file, ReadDir};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default, Clone)]
struct Config {
	destination: PathBuf,
	force: bool,
	verbose: bool,
	static_files: bool,
}

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

	let mut config = Config::default();

	config.force = args.contains(&"-f".to_string()) || args.contains(&"--force".to_string());

	if config.force {
		args.retain(|x| verify_argument(vec!["-f", "--force"], x));
	}

	config.static_files = args.contains(&"-s".to_string()) || args.contains(&"--static".to_string());

	if config.static_files {
		args.retain(|x| verify_argument(vec!["-s", "--static"], x));
	}

	config.verbose = args.contains(&"--verbose".to_string());

	if config.verbose {
		args.retain(|x| verify_argument(vec!["--verbose"], x));
	}

	let is_repo = args.contains(&"repo".to_string());

	if is_repo {
		args.retain(|x| verify_argument(vec!["repo"], x));
	}

	let home = match env::var_os("HOME") {
		Some(home) => home.into_string().unwrap(),
		None => {
			println!("Could not find $HOME");
			return;
		}
	};

	if is_repo {
		let link_repo = match args.pop_front() {
			Some(link_repo) => link_repo,
			None => {
				show_help();
				return;
			}
		};

		let path_repo = PathBuf::from(&home).join(".dotfiles");

		let git = Command::new("git")
			.arg("clone")
			.arg("--depth")
			.arg("1")
			.arg(&link_repo)
			.arg(&path_repo)
			.status();

		match git {
			Ok(res) => {
				if res.success() {
					println!("Repository cloned successfully");

					let ignore_files = get_ignore_files(path_repo.clone());

					config.destination = PathBuf::from(&home);

					match fs::read_dir(path_repo) {
						Ok(files) => {
							if config.static_files {
								create_statics(files, &ignore_files, &config);
							} else {
								create_symlinks(files, &ignore_files, &config);
							}
							println!("Done!");
						}
						Err(_) => {
							show_help();
							return;
						}
					};
				} else {
					if path_repo.exists() && config.force {
						match remove_dir_all(&path_repo) {
							Ok(()) => {
								if config.verbose {
									println!("Removing existing file {}", path_repo.display())
								}
							}
							Err(e) => {
								println!(
									"Failed to remove existing file {}: {}",
									path_repo.display(),
									e
								);
							}
						}

						let git = Command::new("git")
							.arg("clone")
							.arg("--depth")
							.arg("1")
							.arg(&link_repo)
							.arg(&path_repo)
							.status();

						match git {
							Ok(res) => {
								if res.success() {
									println!("Repository cloned successfully");

									let ignore_files = get_ignore_files(path_repo.clone());

									config.destination = PathBuf::from(&home);
									let files = fs::read_dir(path_repo).unwrap();

									if config.static_files {
										create_statics(files, &ignore_files, &config);
									} else {
										create_symlinks(files, &ignore_files, &config);
									}
								} else {
									println!("Error trying to clone repository");
								}
							}
							Err(_) => {
								println!("Could not clone repository");
								return;
							}
						}
					}
				}
			}
			Err(_) => {
				println!("Could not clone repository");
				return;
			}
		}

		return;
	}

	let path = PathBuf::from(args.pop_front().unwrap())
		.canonicalize()
		.unwrap_or_default();

	config.destination = PathBuf::from(home);

	if args.len() == 1 {
		config.destination = match Path::new(args.pop_front().unwrap().as_str()).canonicalize() {
			Ok(path) => path,
			Err(_) => {
				show_help();
				return;
			}
		};

		println!("Destination: {}", config.destination.display());
	}

	if args.len() > 0 {
		show_help();
		return;
	}

	let ignore_files = get_ignore_files(path.clone());

	match fs::read_dir(path) {
		Ok(files) => {
			if config.static_files {
				create_statics(files, &ignore_files, &config);
			} else {
				create_symlinks(files, &ignore_files, &config);
			}
			println!("Done!");
		}
		Err(_) => {
			show_help();
			return;
		}
	};
}

fn verify_argument(names: Vec<&str>, arg: &String) -> bool {
	let mut res = true;
	for name in names {
		res = res && arg != name;
	}

	res
}

fn create_statics(files: ReadDir, ignore_files: &Vec<String>, config: &Config) {
	for file in files {
		let file = file.unwrap();
		let file_path = file.path();
		let file_name = file_path.file_name().unwrap();

		if ignore_files.contains(&file_name.to_str().unwrap().to_string()) {
			if config.verbose {
				println!("Ignoring {}", file_path.display());
			}
			continue;
		}

		if file.file_type().unwrap().is_dir() {
			let files = fs::read_dir(&file_path).unwrap();
			let mut aux_config = config.clone();
			aux_config.destination = config.destination.join(file_name);

			match fs::create_dir(&aux_config.destination) {
				Ok(_) => {
					if config.verbose {
						println!("Created directory {}", aux_config.destination.display());
					}
				}
				Err(_) => {}
			};

			create_statics(files, ignore_files, &aux_config);
			continue;
		}

		let dest = Path::new(&config.destination).join(file_name);

		if dest.exists() && config.force {
			match remove_file(&dest) {
				Ok(()) => {
					if config.verbose {
						println!("Removing existing file {}", dest.display())
					}
				}
				Err(e) => {
					println!("Failed to remove existing file {}: {}", dest.display(), e);
					continue;
				}
			}
		}

		if config.verbose {
			println!(
				"Creating file {} to {}",
				file_path.display(),
				dest.display()
			);
		}

		match copy(&file_path, &dest) {
			Ok(_) => (),
			Err(e) => {
				println!(
					"Failed to link {} to {}: {}",
					file_path.display(),
					dest.display(),
					e
				);
				continue;
			}
		};
	}
}

fn create_symlinks(files: ReadDir, ignore_files: &Vec<String>, config: &Config) {
	for file in files {
		let file = file.unwrap();
		let file_path = file.path();
		let file_name = file_path.file_name().unwrap();

		if ignore_files.contains(&file_name.to_str().unwrap().to_string()) {
			if config.verbose {
				println!("Ignoring {}", file_path.display());
			}
			continue;
		}

		if file.file_type().unwrap().is_dir() {
			let files = fs::read_dir(&file_path).unwrap();
			let mut aux_config = config.clone();
			aux_config.destination = config.destination.join(file_name);

			match fs::create_dir(&aux_config.destination) {
				Ok(_) => {
					if config.verbose {
						println!("Created directory {}", aux_config.destination.display());
					}
				}
				Err(_) => {}
			};

			create_symlinks(files, &ignore_files, &aux_config);
			continue;
		}

		let dest = Path::new(&config.destination).join(file_name);

		if dest.exists() && config.force {
			match remove_file(&dest) {
				Ok(()) => {
					if config.verbose {
						println!("Removing existing file {}", dest.display())
					}
				}
				Err(e) => {
					println!("Failed to remove existing file {}: {}", dest.display(), e);
					continue;
				}
			}
		}

		if config.verbose {
			println!("Linking {} to {}", file_path.display(), dest.display());
		}

		match symlink(&file_path, &dest) {
			Ok(_) => (),
			Err(e) => {
				println!(
					"Failed to link {} to {}: {}",
					file_path.display(),
					dest.display(),
					e
				);
				continue;
			}
		};
	}
}

fn get_ignore_files(path: PathBuf) -> Vec<String> {
	let mut ignore_files: Vec<String> = Vec::new();

	let ignore_file_name = String::from(".dotzignore");

	ignore_files.push(ignore_file_name.clone());

	let ignore_path = path.join(ignore_file_name);

	if !ignore_path.exists() {
		return ignore_files;
	}

	let ignore_file = fs::read_to_string(ignore_path).unwrap();

	for line in ignore_file.lines() {
		ignore_files.push(line.trim().to_string());
	}

	ignore_files
}

fn show_help() {
	println!("dotz - A simple dotfile manager");
	println!("");
	println!("");
	println!("Commands:");
	println!("");
	println!("repo\t\tCreate a new dotfile from a git repository (need git installed)");
	println!("");
	println!("Options:");
	println!("");
	println!("-h, --help\tShow this help message");
	println!("-f, --force\tForce overwrite of existing files");
	println!("-s, --static\tCreate static files");
	println!("--verbose\tShow verbose output");
	println!("");
	println!("");
	println!("Usage: dotz [options] [command] [path/] [destination]");
	println!("");
	println!("");
	println!("[path] is the directory where the dotfiles are located");
	println!("[repo] is the link to the git repository (only for repo mode and default path to clone is $HOME/.dotfiles)");
	println!("[destination] is the directory where the dotfiles will be linked to (optional defaults to $HOME) \n\t(not configurable in repo mode)");
	println!("");
	println!("");
	println!("Examples:");
	println!("\t # dotz -f -s /home/user/.dotfiles/ /home/user/");
	println!("\t # dotz -f --verbose -s /home/user/.dotfiles/ /home/user/");
	println!("\t # dotz /home/user/.dotfiles/");
	println!("\t # dotz .");
	println!("\t # dotz repo https://github.com/zeroproject-0/.dotfiles.git");
	println!("\t # dotz -f -s --verbose repo https://github.com/zeroproject-0/.dotfiles.git");
	println!("\t # dotz --help");
}
