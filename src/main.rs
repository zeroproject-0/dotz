use dotz::errors::Errors;

use std::collections::VecDeque;
use std::env;
use std::fs::{copy, create_dir_all, read_dir, read_to_string, remove_dir_all, remove_file};
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::Command;

struct Config {
	force: bool,
	verbose: bool,
	static_files: bool,
}

#[cfg(target_os = "windows")]
compile_error!("dotz only works on unix systems");

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
			println!("No path provided, using default path ($HOME/.dotfiles)");
			let path = PathBuf::from(&home).join(".dotfiles");
			path
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

	if !path.exists() || path.is_file() {
		return Err(Errors::InvalidPath {
			message: String::from(format!("Path {} does not exist", path.display())),
		});
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

	if !destination.exists() || destination.is_file() {
		return Err(Errors::InvalidPath {
			message: String::from("Destination directory does not exist"),
		});
	}

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

	let ignore_paths = get_ignore_files(&path);

	let mut ignore_files = ignore_paths.clone();
	ignore_files.retain(|file| !file.is_dir());

	let mut ignore_dirs = ignore_paths.clone();
	ignore_dirs.retain(|file| file.is_dir());

	let mut files = get_files(&path);

	files.retain(|file| !ignore_files.contains(file));
	files.retain(|file| !check_paths_contains_path(&ignore_dirs, file));

	let mut dest_files: Vec<PathBuf> = Vec::new();

	for file in &files {
		let path = file.strip_prefix(&path).unwrap();
		let path_buf = destination.join(path);
		dest_files.push(path_buf);
	}

	let success;

	if config.static_files {
		success = create_files(&create_statics, dest_files, files, &config);
	} else {
		success = create_files(&create_symlinks, dest_files, files, &config);
	}

	if success {
		println!("Done!");
	} else {
		return Err(Errors::ErrorCreatingFile {
			message: String::from("Failed to create dotfiles"),
		});
	}

	Ok(())
}

fn create_files<F>(func: &F, dest_files: Vec<PathBuf>, paths: Vec<PathBuf>, config: &Config) -> bool
where
	F: Fn(&PathBuf, &PathBuf) -> bool,
{
	let mut success = true;

	for i in 0..dest_files.len() {
		let mut ancestors = dest_files[i].ancestors();
		ancestors.next();

		if dest_files[i].exists() {
			if config.force {
				match remove_file(&dest_files[i]) {
					Ok(_) => {
						if config.verbose {
							println!("Removed {}", dest_files[i].display());
						}
					}
					Err(_) => {}
				};
			} else {
				continue;
			}
		} else {
			let dir_path = ancestors.next().unwrap();
			if !dir_path.exists() {
				match create_dir_all(dir_path) {
					Ok(_) => {
						if config.verbose {
							println!("Created {}", ancestors.next().unwrap().display());
						}
					}
					Err(_) => {}
				};
			}
		}

		if !func(&paths[i], &dest_files[i]) {
			println!("Failed to create {}", dest_files[i].display());
			success = false;
		} else {
			if config.verbose {
				println!("Created {}", dest_files[i].display());
			}
		}
	}

	true && success
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

fn get_files(path: &PathBuf) -> Vec<PathBuf> {
	let mut files = Vec::new();

	for file in read_dir(path).unwrap() {
		let file = file.unwrap();
		let file_path = file.path();
		let file_type = file.file_type().unwrap();

		if file_type.is_dir() {
			let mut dir_files = get_files(&file_path);
			files.append(&mut dir_files);
		} else {
			files.push(file_path);
		}
	}

	files
}

fn check_paths_contains_path(paths: &Vec<PathBuf>, path: &PathBuf) -> bool {
	let path_str = path.to_str().unwrap();
	for p in paths {
		let current = p.to_str().unwrap();
		if path_str.starts_with(current) {
			return true;
		}
	}

	false
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

fn create_statics(file_path: &PathBuf, dest: &PathBuf) -> bool {
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

fn create_symlinks(file_path: &PathBuf, dest: &PathBuf) -> bool {
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

fn get_ignore_files(path: &PathBuf) -> Vec<PathBuf> {
	let mut ignore_files: Vec<PathBuf> = Vec::new();

	let ignore_file = path.join("./.dotzignore").canonicalize();

	if ignore_file.is_err() {
		return ignore_files;
	}

	ignore_files.push(ignore_file.unwrap());

	let ignore_file = read_to_string(&ignore_files[0]).unwrap();

	for line in ignore_file.lines() {
		let line = line.trim();
		let file = path.join(line).canonicalize();
		if line == "" || file.is_err() {
			continue;
		}

		let file_path = file.unwrap();
		ignore_files.push(file_path);
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

#[cfg(test)]
mod tests {
	use super::*;
	use assert_cli;
	use std::fs;

	#[test]
	fn test_get_ignore_files() {
		let path = PathBuf::from("./tests/");
		let ignore_files = get_ignore_files(&path);

		assert_eq!(ignore_files.len(), 2);
		assert_eq!(
			ignore_files[0],
			path.join(".dotzignore").canonicalize().unwrap()
		);
		assert_eq!(
			ignore_files[1],
			path.join("./.dot/test.conf").canonicalize().unwrap()
		);
	}

	#[test]
	fn test_verify_argument() {
		let mut args = VecDeque::new();
		args.push_back(String::from("test"));
		args.push_back(String::from("test2"));
		args.push_back(String::from("test3"));

		let names = vec!["test", "test2"];

		assert_eq!(verify_argument(&mut args, names), true);
		assert_eq!(args.len(), 1);
		assert_eq!(args[0], "test3");
	}

	#[test]
	fn test_cli_help() {
		assert_cli::Assert::main_binary()
			.with_args(&["--help"])
			.succeeds()
			.unwrap();
	}

	#[test]
	fn test_cli_version() {
		assert_cli::Assert::main_binary()
			.with_args(&["--version"])
			.succeeds()
			.stdout()
			.is(format!("v{}", env!("CARGO_PKG_VERSION")).as_str())
			.unwrap();
	}

	#[test]
	fn test_cli_repo_without_args() {
		assert_cli::Assert::main_binary()
			.with_args(&["repo"])
			.fails()
			.unwrap()
	}

	#[test]
	fn test_cli_repo_with_args() {
		let repo_path = PathBuf::from("./tests/.dotfilesRepo");
		let dot_path = PathBuf::from("./tests/.dotfiles");
		if repo_path.exists() {
			fs::remove_dir_all(repo_path).unwrap();
		}
		if dot_path.exists() {
			fs::remove_dir_all(dot_path).unwrap();
		}
		fs::create_dir("./tests/.dotfilesRepo").unwrap();
		fs::create_dir("./tests/.dotfiles").unwrap();
		assert_cli::Assert::main_binary()
			.with_args(&[
				"repo",
				"https://github.com/zeroproject-0/.dotfiles.git",
				"./tests/.dotfilesRepo",
				"./tests/.dotfiles",
			])
			.succeeds()
			.unwrap()
	}

	#[test]
	fn test_cli_repo_with_args_fails() {
		assert_cli::Assert::main_binary()
			.with_args(&[
				"repo",
				"https://github.com/zeroproject-0/.dotfiles.git",
				"./tests/.dotfilesRepo",
				"./tests/notExist",
			])
			.fails()
			.unwrap()
	}

	#[test]
	fn test_cli_force() {
		assert_cli::Assert::main_binary()
			.with_args(&["-f", "./tests/.dot", "./tests/.dotDest"])
			.succeeds()
			.unwrap()
	}

	#[test]
	fn test_cli_static() {
		assert_cli::Assert::main_binary()
			.with_args(&["-f", "-s", "./tests/.dot", "./tests/.dotDest"])
			.execute()
			.and_then(|_| {
				let file_type = fs::metadata("./tests/.dotDest/test.conf")
					.unwrap()
					.file_type();
				assert!(!fs::FileType::is_symlink(&file_type));
				Ok(())
			})
			.unwrap()
	}
}
