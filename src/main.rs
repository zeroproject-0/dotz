use dotz::errors::Errors;

use std::collections::VecDeque;
use std::env;
use std::fs::{
	copy, create_dir, create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file, ReadDir,
};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Config {
	force: bool,
	verbose: bool,
	static_files: bool,
}

fn main() -> Result<(), Errors> {
	let mut args: VecDeque<String> = env::args().skip(1).collect();

	if args.len() == 0 {
		show_help();
		return Err(Errors::InvalidArgument {
			message: String::from("No arguments provided"),
		});
	}

	if args.contains(&String::from("-h")) || args.contains(&String::from("--help")) {
		show_help();
		return Ok(());
	}

	if args.contains(&String::from("-v")) || args.contains(&String::from("--version")) {
		const VERSION: &str = env!("CARGO_PKG_VERSION");
		println!("v{}", VERSION);
		return Ok(());
	}

	let config = Config {
		force: verify_argument(&mut args, vec!["-f", "--force"]),
		verbose: verify_argument(&mut args, vec!["--verbose"]),
		static_files: verify_argument(&mut args, vec!["-s", "--static"]),
	};

	let is_repo = verify_argument(&mut args, vec!["repo"]);

	let home = match env::var_os("HOME") {
		Some(home) => home.into_string().unwrap(),
		None => {
			println!("Could not find $HOME");
			return Err(Errors::InvalidPath {
				message: String::from("Could not find $HOME"),
			});
		}
	};

	let mut repo_link = String::new();

	if is_repo {
		if let Some(repo) = args.pop_front() {
			repo_link = repo;
		} else {
			show_help();
			return Err(Errors::RepositoryNotCloned {
				message: String::from("No repository link provided"),
			});
		}
	}

	let mut path = if let Some(path_arg) = args.pop_front() {
		PathBuf::from(path_arg)
	} else {
		if is_repo {
			PathBuf::from(&home)
				.join(".dotfiles")
				.canonicalize()
				.unwrap()
		} else {
			println!("No path provided");
			show_help();
			return Err(Errors::InvalidArgument {
				message: String::from("No path provided"),
			});
		}
	};

	if is_repo && !path.exists() {
		match create_dir_all(&path) {
			Ok(_) => {
				if config.verbose {
					println!("Created {}", path.display());
				}
			}
			Err(_) => {
				return Err(Errors::ErrorCreatingDirectory {
					message: String::from(format!("Could not create {}", path.display())),
				});
			}
		};
	}

	if !path.is_absolute() {
		path = path.canonicalize().unwrap();
	}

	let destination = if let Some(dest) = args.pop_front() {
		let dest = PathBuf::from(dest);
		if !dest.exists() {
			return Err(Errors::InvalidPath {
				message: String::from("Destination directory does not exist"),
			});
		}

		dest.canonicalize().unwrap()
	} else {
		PathBuf::from(home)
	};

	if args.len() > 0 {
		show_help();
		return Err(Errors::InvalidArgument {
			message: String::from("Too many arguments provided"),
		});
	}

	if is_repo {
		if config.force && path.exists() {
			match remove_dir_all(&path) {
				Ok(_) => {
					if config.verbose {
						println!("Removed {}", path.display());
					}
				}
				Err(_) => {
					return Err(Errors::ErrorRemovingDirectory {
						message: String::from(format!("Could not remove {}", path.display())),
					});
				}
			};
		}

		if !clone_repo(&repo_link, &path.to_str().unwrap().to_string()) {
			return Err(Errors::RepositoryNotCloned {
				message: String::from("Could not clone repository"),
			});
		}
	}

	let ignore_files = get_ignore_files(&path);

	let success;

	match read_dir(path) {
		Ok(files) => {
			if config.static_files {
				success = files_scope(&create_statics, files, &ignore_files, destination, &config);
			} else {
				success = files_scope(&create_symlinks, files, &ignore_files, destination, &config);
			}

			if success {
				println!("Done!");
			} else {
				return Err(Errors::ErrorCreatingFile {
					message: String::from("Failed to create dotfiles"),
				});
			}
		}
		Err(_) => {
			println!("Could not read directory");
			show_help();
			return Err(Errors::ErrorReadingDirectory {
				message: String::from("Could not read directory"),
			});
		}
	};

	Ok(())
}

fn clone_repo(link: &String, dest: &String) -> bool {
	let mut success = false;
	let git = Command::new("git")
		.arg("clone")
		.arg(&link)
		.arg(&dest)
		.status();

	match git {
		Ok(res) => {
			success = res.success();
			if success {
				println!("Repository cloned successfully");
			} else {
				println!("Failed to clone repository");
			}
		}
		Err(_) => {
			println!("Failed to clone repository");
		}
	}

	success
}

fn files_scope<F>(
	func: &F,
	files: ReadDir,
	ignore_files: &Vec<String>,
	destination: PathBuf,
	config: &Config,
) -> bool
where
	F: Fn(PathBuf, PathBuf) -> bool,
{
	let mut success = true;
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
			let files = read_dir(&file_path).unwrap();

			let dest = destination.join(file_name);

			match create_dir(&dest) {
				Ok(_) => {
					if config.verbose {
						println!("Created directory {}", dest.display());
					}
				}
				Err(_) => {}
			};

			success = files_scope(func, files, ignore_files, dest, &config);
			continue;
		}

		let dest = Path::new(&destination).join(file_name);

		if dest.exists() && config.force {
			match remove_file(&dest) {
				Ok(()) => {
					if config.verbose {
						println!("Removing existing file {}", dest.display())
					}
				}
				Err(_) => {
					println!("Failed to remove existing file {}", dest.display());
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

		if !func(file_path.to_owned(), dest.to_owned()) {
			success = false;
			println!(
				"Failed to create file {} to {}",
				file_path.display(),
				dest.display(),
			);
			continue;
		};
	}

	true && success
}

fn verify_argument(args: &mut VecDeque<String>, names: Vec<&str>) -> bool {
	let verify = |x: &String| {
		let mut res = true;
		for name in &names {
			res = res && x != name;
		}

		res
	};
	let args_length = args.len();

	args.retain(verify);

	args_length != args.len()
}

fn create_statics(file_path: PathBuf, dest: PathBuf) -> bool {
	if dest.exists() {
		return false;
	}

	match copy(&file_path, &dest) {
		Ok(_) => {
			return true;
		}
		Err(e) => {
			println!(
				"Failed to link {} to {}: {}",
				file_path.display(),
				dest.display(),
				e
			);
			return false;
		}
	}
}

fn create_symlinks(file_path: PathBuf, dest: PathBuf) -> bool {
	match symlink(&file_path, &dest) {
		Ok(_) => {
			return true;
		}
		Err(_) => {
			println!(
				"Failed to link {} to {}",
				file_path.display(),
				dest.display()
			);

			return false;
		}
	}
}

fn get_ignore_files(path: &PathBuf) -> Vec<String> {
	let mut ignore_files: Vec<String> = Vec::new();

	let ignore_file_name = String::from(".dotzignore");

	ignore_files.push(ignore_file_name.clone());

	let ignore_path = path.join(ignore_file_name);

	if !ignore_path.exists() {
		return ignore_files;
	}

	let ignore_file = read_to_string(ignore_path).unwrap();

	for line in ignore_file.lines() {
		let line = line.trim();
		if line == "" {
			continue;
		}
		ignore_files.push(line.to_string());
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
	println!("-v, --version\tShow version");
	println!("--verbose\tShow verbose output");
	println!("");
	println!("");
	println!("Usage: dotz [options] [command] [repo] [path] [destination]");
	println!("");
	println!("");
	println!("[path] \tis the directory where the dotfiles are located or in repo mode the path where the dotfiles will be cloned to (optional defaults to $HOME/.dotfiles) make sure the directory is empty");
	println!("[repo] \tis the link to the git repository");
	println!("[destination] \t is the directory where the dotfiles will be linked to (optional defaults to $HOME)");
	println!("");
	println!("");
	println!("Examples:");
	println!("\t # dotz -f -s /home/user/.dotfiles/ /home/user/");
	println!("\t # dotz -f --verbose -s /home/user/.dotfiles/ /home/user/");
	println!("\t # dotz /home/user/.dotfiles/");
	println!("\t # dotz .");
	println!("\t # dotz repo https://github.com/zeroproject-0/.dotfiles.git");
	println!("\t # dotz repo https://github.com/zeroproject-0/.dotfiles.git ./dotfiles");
	println!("\t # dotz repo https://github.com/zeroproject-0/.dotfiles.git ./dotfiles ~/.config");
	println!("\t # dotz -f -s --verbose repo https://github.com/zeroproject-0/.dotfiles.git");
	println!("\t # dotz --help");
}
